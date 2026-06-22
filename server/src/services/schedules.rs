use std::collections::HashSet;

use chrono::{DateTime, Duration, NaiveTime, Utc};
use chrono_tz::Tz;
use sqlx::postgres::PgRow;
use sqlx::{PgPool, Row};

use crate::models::{Dose, DoseDetail, DoseStatus, FamilyMember, Household, Treatment};
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

// Колонки назначения/собаки берём как COALESCE(снимок дозы, живое назначение):
// для отмеченных доз и для доз удалённых назначений работает снимок, для будущих
// (ещё не принятых) — живые treatments/dogs. JOIN-ы LEFT, чтобы дозы без живого
// назначения (treatment_id = NULL) не выпадали из выборки истории.
pub(crate) const DETAIL_SELECT: &str = "SELECT \
    d.id AS dose_id, d.treatment_id AS dose_treatment_id, d.due_at, d.status, d.api_key, \
    d.reminded_at, d.escalation_level, d.last_escalated_at, \
    d.taken_at, d.confirmed_by_member_id, d.note, d.clinic AS dose_clinic, d.created_at AS dose_created_at, \
    COALESCE(d.treatment_name, t.name)       AS eff_name, \
    COALESCE(d.kind, t.kind)                 AS eff_kind, \
    COALESCE(d.category, t.category)         AS eff_category, \
    COALESCE(d.dose_label, t.dose_label)     AS eff_dose_label, \
    COALESCE(d.instructions, t.instructions) AS eff_instructions, \
    COALESCE(d.cycle_days, t.cycle_days)     AS eff_cycle_days, \
    COALESCE(d.clinic, t.clinic)             AS eff_clinic, \
    COALESCE(d.dog_name, g.name)             AS eff_dog_name, \
    COALESCE(d.dog_id, t.dog_id)             AS eff_dog_id, \
    COALESCE(d.household_id, g.household_id) AS eff_household_id \
    FROM doses d \
    LEFT JOIN treatments t ON t.id = d.treatment_id \
    LEFT JOIN dogs g ON g.id = t.dog_id";

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
    Ok(DoseDetail {
        name: row
            .try_get::<Option<String>, _>("eff_name")?
            .unwrap_or_default(),
        kind: row.try_get("eff_kind")?,
        category: row.try_get("eff_category")?,
        dose_label: row.try_get("eff_dose_label")?,
        instructions: row.try_get("eff_instructions")?,
        cycle_days: row.try_get("eff_cycle_days")?,
        clinic: row.try_get("eff_clinic")?,
        dog_name: row.try_get("eff_dog_name")?,
        dog_id: row.try_get("eff_dog_id")?,
        household_id: row.try_get("eff_household_id")?,
        dose,
    })
}

/// Снять снимок настроек назначения и собаки в дозу. Идемпотентно: заполняет
/// только пустые снимок-колонки (COALESCE с текущим значением), поэтому повторный
/// вызов и уже сделанный ранее снимок ничего не портят. Если у дозы нет живого
/// назначения (treatment_id = NULL), UPDATE просто не находит строк.
pub async fn snapshot_treatment_into_dose(pool: &PgPool, dose_id: i32) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE doses SET \
            treatment_name = COALESCE(doses.treatment_name, t.name), \
            kind           = COALESCE(doses.kind, t.kind), \
            category       = COALESCE(doses.category, t.category), \
            dose_label     = COALESCE(doses.dose_label, t.dose_label), \
            instructions   = COALESCE(doses.instructions, t.instructions), \
            cycle_days     = COALESCE(doses.cycle_days, t.cycle_days), \
            clinic         = COALESCE(doses.clinic, t.clinic), \
            dog_name       = COALESCE(doses.dog_name, g.name), \
            dog_id         = COALESCE(doses.dog_id, g.id), \
            household_id   = COALESCE(doses.household_id, g.household_id) \
         FROM treatments t JOIN dogs g ON g.id = t.dog_id \
         WHERE doses.id = $1 AND t.id = doses.treatment_id",
    )
    .bind(dose_id)
    .execute(pool)
    .await?;
    Ok(())
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
    let rows = sqlx::query(sqlx::AssertSqlSafe(sql))
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
    let updated: Option<i32> = sqlx::query_scalar(
        "UPDATE doses SET status = 'taken', taken_at = now(), \
         confirmed_by_member_id = $2, note = $3 \
         WHERE id = $1 RETURNING id",
    )
    .bind(dose_id)
    .bind(member.id)
    .bind(note)
    .fetch_optional(pool)
    .await?;
    let Some(id) = updated else {
        return Ok(None);
    };
    snapshot_treatment_into_dose(pool, id).await?;
    sqlx::query_as::<_, Dose>("SELECT * FROM doses WHERE id = $1")
        .bind(id)
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
    let updated: Option<i32> = sqlx::query_scalar(
        "UPDATE doses SET status = 'taken', taken_at = now(), note = $3 \
         WHERE id = $1 AND api_key = $2 RETURNING id",
    )
    .bind(dose_id)
    .bind(api_key)
    .bind(note)
    .fetch_optional(pool)
    .await?;
    let Some(id) = updated else {
        return Ok(None);
    };
    snapshot_treatment_into_dose(pool, id).await?;
    sqlx::query_as::<_, Dose>("SELECT * FROM doses WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
}

/// Все активные напоминания (статус reminded) с деталями — для шага эскалации.
pub async fn get_reminded_doses(pool: &PgPool) -> Result<Vec<DoseDetail>, sqlx::Error> {
    let sql = format!(
        "{DETAIL_SELECT} WHERE t.active = true AND d.status = 'reminded' ORDER BY d.due_at"
    );
    let rows = sqlx::query(sqlx::AssertSqlSafe(sql)).fetch_all(pool).await?;
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
    Notify {
        member_index: usize,
        next_level: i32,
    },
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
    let rows = sqlx::query(sqlx::AssertSqlSafe(sql))
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
