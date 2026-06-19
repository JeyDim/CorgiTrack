use std::collections::HashSet;

use chrono::{DateTime, Duration, NaiveTime, Utc};
use chrono_tz::Tz;
use sqlx::postgres::PgRow;
use sqlx::{PgPool, Row};

use crate::models::{
    Dose, DoseDetail, DoseStatus, FamilyMember, Household, Treatment, TreatmentKind,
};
use crate::util::{generate_api_key, local_to_utc};

/// Плановое UTC-время приёма для дня `day`, взяв время `reminder_time` в локальной зоне.
pub fn combine_due(tz: Tz, day: DateTime<Utc>, reminder_time: NaiveTime) -> DateTime<Utc> {
    let local_date = day.with_timezone(&tz).date_naive();
    let naive = local_date.and_time(reminder_time);
    local_to_utc(tz, naive)
}

/// Все плановые даты приёма от старта назначения до `until` включительно.
pub fn iter_due_dates(
    tz: Tz,
    start_at: DateTime<Utc>,
    reminder_time: NaiveTime,
    cycle_days: i32,
    until: DateTime<Utc>,
) -> Vec<DateTime<Utc>> {
    let mut dates = Vec::new();
    let step = Duration::days(cycle_days.max(1) as i64);
    let mut due = combine_due(tz, start_at, reminder_time);
    while due <= until {
        dates.push(due);
        due += step;
    }
    dates
}

/// Догенерировать будущие дозы на горизонте `horizon_days` для всех активных назначений.
pub async fn ensure_future_doses(
    pool: &PgPool,
    tz: Tz,
    horizon_days: i64,
) -> Result<u64, sqlx::Error> {
    let until = Utc::now() + Duration::days(horizon_days);
    let treatments = sqlx::query_as::<_, Treatment>("SELECT * FROM treatments WHERE active = true")
        .fetch_all(pool)
        .await?;

    let mut created: u64 = 0;
    for t in treatments {
        let existing: Vec<DateTime<Utc>> =
            sqlx::query_scalar("SELECT due_at FROM doses WHERE treatment_id = $1 AND due_at <= $2")
                .bind(t.id)
                .bind(until)
                .fetch_all(pool)
                .await?;
        let existing: HashSet<DateTime<Utc>> = existing.into_iter().collect();

        for due in iter_due_dates(tz, t.start_at, t.reminder_time, t.cycle_days, until) {
            if !existing.contains(&due) {
                sqlx::query(
                    "INSERT INTO doses (treatment_id, due_at, status, api_key) \
                     VALUES ($1, $2, 'planned', $3)",
                )
                .bind(t.id)
                .bind(due)
                .bind(generate_api_key())
                .execute(pool)
                .await?;
                created += 1;
            }
        }
    }
    Ok(created)
}

pub(crate) const DETAIL_SELECT: &str = "SELECT \
    d.id AS dose_id, d.treatment_id AS dose_treatment_id, d.due_at, d.status, d.api_key, \
    d.reminded_at, d.escalation_level, d.last_escalated_at, \
    d.taken_at, d.confirmed_by_member_id, d.note, d.clinic AS dose_clinic, d.created_at AS dose_created_at, \
    t.id AS t_id, t.dog_id AS t_dog_id, t.name AS t_name, t.kind AS t_kind, t.dose_label, \
    t.cycle_days, t.start_at, t.reminder_time, t.instructions, t.active, t.clinic AS t_clinic, t.created_at AS t_created_at, \
    g.name AS dog_name, g.household_id AS household_id \
    FROM doses d \
    JOIN treatments t ON t.id = d.treatment_id \
    JOIN dogs g ON g.id = t.dog_id";

pub(crate) fn row_to_detail(row: &PgRow) -> Result<DoseDetail, sqlx::Error> {
    let dose = Dose {
        id: row.try_get("dose_id")?,
        treatment_id: row.try_get("dose_treatment_id")?,
        due_at: row.try_get("due_at")?,
        status: row.try_get("status")?,
        api_key: row.try_get("api_key")?,
        reminded_at: row.try_get("reminded_at")?,
        escalation_level: row.try_get("escalation_level")?,
        last_escalated_at: row.try_get("last_escalated_at")?,
        taken_at: row.try_get("taken_at")?,
        confirmed_by_member_id: row.try_get("confirmed_by_member_id")?,
        note: row.try_get("note")?,
        clinic: row.try_get("dose_clinic")?,
        created_at: row.try_get("dose_created_at")?,
    };
    let treatment = Treatment {
        id: row.try_get("t_id")?,
        dog_id: row.try_get("t_dog_id")?,
        name: row.try_get("t_name")?,
        kind: row.try_get::<TreatmentKind, _>("t_kind")?,
        dose_label: row.try_get("dose_label")?,
        cycle_days: row.try_get("cycle_days")?,
        start_at: row.try_get("start_at")?,
        reminder_time: row.try_get("reminder_time")?,
        instructions: row.try_get("instructions")?,
        active: row.try_get("active")?,
        clinic: row.try_get("t_clinic")?,
        created_at: row.try_get("t_created_at")?,
    };
    Ok(DoseDetail {
        dog_name: row.try_get("dog_name")?,
        household_id: row.try_get("household_id")?,
        dose,
        treatment,
    })
}

pub async fn get_household_for_telegram(
    pool: &PgPool,
    telegram_user_id: i64,
) -> Result<Option<Household>, sqlx::Error> {
    sqlx::query_as::<_, Household>(
        "SELECT h.* FROM households h \
         JOIN family_members m ON m.household_id = h.id \
         WHERE m.telegram_user_id = $1 LIMIT 1",
    )
    .bind(telegram_user_id)
    .fetch_optional(pool)
    .await
}

/// Дозы семьи, которые скоро нужно принять (статусы planned/reminded).
pub async fn get_due_for_household(
    pool: &PgPool,
    household_id: i32,
    lookahead: Duration,
) -> Result<Vec<DoseDetail>, sqlx::Error> {
    let cutoff = Utc::now() + lookahead;
    let sql = format!(
        "{DETAIL_SELECT} WHERE t.active = true AND g.household_id = $1 \
         AND d.status IN ('planned', 'reminded') AND d.due_at <= $2 ORDER BY d.due_at"
    );
    let rows = sqlx::query(&sql)
        .bind(household_id)
        .bind(cutoff)
        .fetch_all(pool)
        .await?;
    rows.iter().map(row_to_detail).collect()
}

/// Отметить дозу принятой по Telegram-пользователю (подтвердившему).
pub async fn mark_taken(
    pool: &PgPool,
    dose_id: i32,
    telegram_user_id: i64,
    note: Option<&str>,
) -> Result<Option<Dose>, sqlx::Error> {
    let member = sqlx::query_as::<_, FamilyMember>(
        "SELECT * FROM family_members WHERE telegram_user_id = $1",
    )
    .bind(telegram_user_id)
    .fetch_optional(pool)
    .await?;
    let Some(member) = member else {
        return Ok(None);
    };
    sqlx::query_as::<_, Dose>(
        "UPDATE doses SET status = 'taken', taken_at = now(), \
         confirmed_by_member_id = $2, note = $3, \
         clinic = COALESCE(clinic, (SELECT t.clinic FROM treatments t WHERE t.id = doses.treatment_id)) \
         WHERE id = $1 RETURNING *",
    )
    .bind(dose_id)
    .bind(member.id)
    .bind(note)
    .fetch_optional(pool)
    .await
}

/// Отметить дозу принятой по per-dose api_key (ссылка из календаря).
pub async fn mark_taken_by_api_key(
    pool: &PgPool,
    dose_id: i32,
    api_key: &str,
    note: Option<&str>,
) -> Result<Option<Dose>, sqlx::Error> {
    sqlx::query_as::<_, Dose>(
        "UPDATE doses SET status = 'taken', taken_at = now(), note = $3, \
         clinic = COALESCE(clinic, (SELECT t.clinic FROM treatments t WHERE t.id = doses.treatment_id)) \
         WHERE id = $1 AND api_key = $2 RETURNING *",
    )
    .bind(dose_id)
    .bind(api_key)
    .bind(note)
    .fetch_optional(pool)
    .await
}

/// Все активные напоминания (статус reminded) с деталями — для шага эскалации.
pub async fn get_reminded_doses(pool: &PgPool) -> Result<Vec<DoseDetail>, sqlx::Error> {
    let sql =
        format!("{DETAIL_SELECT} WHERE t.active = true AND d.status = 'reminded' ORDER BY d.due_at");
    let rows = sqlx::query(&sql).fetch_all(pool).await?;
    rows.iter().map(row_to_detail).collect()
}

/// Члены семьи, подлежащие уведомлению, в порядке эскалации (0 — первым).
pub async fn ordered_notify_members(
    pool: &PgPool,
    household_id: i32,
) -> Result<Vec<FamilyMember>, sqlx::Error> {
    sqlx::query_as::<_, FamilyMember>(
        "SELECT * FROM family_members \
         WHERE household_id = $1 AND notify = TRUE AND telegram_user_id IS NOT NULL \
         ORDER BY escalation_order, id",
    )
    .bind(household_id)
    .fetch_all(pool)
    .await
}

/// Зафиксировать шаг эскалации: новый уровень и момент отправки.
pub async fn advance_escalation(
    pool: &PgPool,
    dose_id: i32,
    next_level: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE doses SET escalation_level = $2, last_escalated_at = now() \
         WHERE id = $1 AND status = 'reminded'",
    )
    .bind(dose_id)
    .bind(next_level)
    .execute(pool)
    .await?;
    Ok(())
}

/// Перевести дозу в missed (эскалация исчерпала всех членов семьи).
pub async fn mark_missed(pool: &PgPool, dose_id: i32) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE doses SET status = 'missed' WHERE id = $1 AND status = 'reminded'")
        .bind(dose_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Что делать с напоминанием на текущем тике эскалации.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EscalationAction {
    /// Интервал ещё не вышел — ничего не делаем.
    Wait,
    /// Уведомить участника с этим индексом в списке эскалации и поднять уровень.
    Notify { member_index: usize, next_level: i32 },
    /// Список исчерпан — отметить дозу пропущенной.
    Missed,
}

/// Чистое правило эскалации: по текущему уровню/времени решает следующий шаг.
///
/// Уровни: 1 — отправлено первое напоминание участнику №0; ожидание
/// `first_delay` → повтор тому же участнику №0 (уровень 2); далее каждые `step`
/// минут уведомляется следующий участник по списку. Когда участники кончились —
/// доза переходит в missed.
pub fn next_escalation_action(
    level: i32,
    last_escalated_at: Option<DateTime<Utc>>,
    member_count: usize,
    now: DateTime<Utc>,
    first_delay_minutes: i64,
    step_minutes: i64,
) -> EscalationAction {
    if level < 1 {
        return EscalationAction::Wait;
    }
    let Some(last) = last_escalated_at else {
        return EscalationAction::Wait;
    };
    let required = if level == 1 {
        first_delay_minutes
    } else {
        step_minutes
    };
    if now - last < Duration::minutes(required) {
        return EscalationAction::Wait;
    }
    // Индекс участника для шага: уровень 1 -> повтор №0, уровень 2 -> №1, и т.д.
    let member_index = (level - 1) as usize;
    if member_index < member_count {
        EscalationAction::Notify {
            member_index,
            next_level: level + 1,
        }
    } else {
        EscalationAction::Missed
    }
}

/// Перевести наступившие planned-дозы в reminded (готовые к рассылке напоминаний).
pub async fn mark_ready_to_remind(
    pool: &PgPool,
    lookahead_minutes: i64,
) -> Result<Vec<DoseDetail>, sqlx::Error> {
    let now = Utc::now();
    let from = now - Duration::minutes(5);
    let to = now + Duration::minutes(lookahead_minutes);
    let sql = format!(
        "{DETAIL_SELECT} WHERE t.active = true AND d.status = 'planned' \
         AND d.due_at >= $1 AND d.due_at <= $2"
    );
    let rows = sqlx::query(&sql)
        .bind(from)
        .bind(to)
        .fetch_all(pool)
        .await?;
    let mut details: Vec<DoseDetail> = rows.iter().map(row_to_detail).collect::<Result<_, _>>()?;
    if details.is_empty() {
        return Ok(details);
    }
    let ids: Vec<i32> = details.iter().map(|d| d.dose.id).collect();
    sqlx::query(
        "UPDATE doses SET status = 'reminded', reminded_at = $2, \
         escalation_level = 1, last_escalated_at = $2 WHERE id = ANY($1)",
    )
    .bind(&ids)
    .bind(now)
    .execute(pool)
    .await?;
    for d in &mut details {
        d.dose.status = DoseStatus::Reminded;
        d.dose.reminded_at = Some(now);
        d.dose.escalation_level = 1;
        d.dose.last_escalated_at = Some(now);
    }
    Ok(details)
}
