use chrono::{Duration, Utc};
use chrono_tz::Tz;
use icalendar::{Calendar, Component, Event, EventLike};
use sqlx::PgPool;

use crate::models::{DoseDetail, Household};
use crate::services::schedules::{ensure_future_doses, row_to_detail, DETAIL_SELECT};
use crate::util::generate_api_key;

/// Собрать iCal-ленту для семьи по её calendar_token. None, если токен не найден.
pub async fn build_ical(
    pool: &PgPool,
    tz: Tz,
    public_base_url: &str,
    token: &str,
) -> Result<Option<Vec<u8>>, sqlx::Error> {
    let household =
        sqlx::query_as::<_, Household>("SELECT * FROM households WHERE calendar_token = $1")
            .bind(token)
            .fetch_optional(pool)
            .await?;
    let Some(household) = household else {
        return Ok(None);
    };

    ensure_future_doses(pool, tz, 370).await?;

    let since = Utc::now() - Duration::days(30);
    let sql =
        format!("{DETAIL_SELECT} WHERE g.household_id = $1 AND d.due_at >= $2 ORDER BY d.due_at");
    let rows = sqlx::query(sqlx::AssertSqlSafe(sql))
        .bind(household.id)
        .bind(since)
        .fetch_all(pool)
        .await?;
    let mut details: Vec<DoseDetail> = rows.iter().map(row_to_detail).collect::<Result<_, _>>()?;

    // Бэкфилл api_key для старых доз без ключа.
    for d in &mut details {
        if d.dose.api_key.is_none() {
            let key = generate_api_key();
            sqlx::query("UPDATE doses SET api_key = $2 WHERE id = $1")
                .bind(d.dose.id)
                .bind(&key)
                .execute(pool)
                .await?;
            d.dose.api_key = Some(key);
        }
    }

    let mut cal = Calendar::new();
    cal.name(&format!("{}: уход за собакой", household.name));

    for d in &details {
        let start = d.dose.due_at;
        let end = start + Duration::minutes(15);
        let event = Event::new()
            .uid(&format!("dose-{}@corgitrack", d.dose.id))
            .summary(&format!(
                "{}: {}",
                d.dog_name.as_deref().unwrap_or(""),
                d.name
            ))
            .starts(start)
            .ends(end)
            .description(&event_description(public_base_url, d))
            .add_property("URL", mark_taken_url(public_base_url, d.dose.id, key_of(d)))
            .add_property("STATUS", "CONFIRMED")
            .done();
        cal.push(event);
    }

    Ok(Some(cal.to_string().into_bytes()))
}

fn key_of(d: &DoseDetail) -> &str {
    d.dose.api_key.as_deref().unwrap_or("")
}

/// Ссылка для отметки приёма по api_key (как в Python).
pub fn mark_taken_url(public_base_url: &str, dose_id: i32, api_key: &str) -> String {
    format!(
        "{}/api/doses/{}/taken?key={}",
        public_base_url.trim_end_matches('/'),
        dose_id,
        api_key
    )
}

/// Текст описания события календаря.
pub fn event_description(public_base_url: &str, d: &DoseDetail) -> String {
    let mut parts = vec![format!("Статус: {}", d.dose.status.label())];
    if let Some(cycle_days) = d.cycle_days {
        parts.push(format!("Цикл: каждые {cycle_days} дн."));
    }
    if let Some(label) = &d.dose_label {
        parts.push(format!("Доза: {label}"));
    }
    if let Some(instructions) = &d.instructions {
        parts.push(instructions.clone());
    }
    parts.push(format!(
        "Отметить прием: {}",
        mark_taken_url(public_base_url, d.dose.id, key_of(d))
    ));
    parts.push("Также можно подтвердить прием в Telegram-боте.".to_string());
    parts.join("\n")
}
