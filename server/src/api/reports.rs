use axum::extract::{Path, Query, State};
use axum::http::header::{CONTENT_DISPOSITION, CONTENT_TYPE};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use chrono::Duration;
use serde::Deserialize;
use serde_json::json;

use crate::error::{AppError, AppResult};
use crate::models::{DoseView, Household};
use crate::services::reports::taken_csv_for_household;
use crate::services::schedules::{ensure_future_doses, get_due_for_household};
use crate::state::AppState;
use crate::util::timezone;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/households/:id/report.csv", get(report_csv))
        .route("/households/:id/due", get(due))
        .route("/households/:id/calendar-url", get(calendar_url))
}

async fn report_csv(State(state): State<AppState>, Path(id): Path<i32>) -> AppResult<Response> {
    let body = taken_csv_for_household(&state.pool, id).await?;
    Ok((
        [
            (CONTENT_TYPE, "text/csv; charset=utf-8"),
            (
                CONTENT_DISPOSITION,
                "attachment; filename=\"corgitrack-prinyatye-dozy.csv\"",
            ),
        ],
        body,
    )
        .into_response())
}

#[derive(Debug, Deserialize)]
pub struct DueQuery {
    pub lookahead_hours: Option<i64>,
}

async fn due(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Query(q): Query<DueQuery>,
) -> AppResult<Json<Vec<DoseView>>> {
    let tz = timezone(&state.settings.app_timezone);
    ensure_future_doses(&state.pool, tz, 370).await?;
    let lookahead = Duration::hours(q.lookahead_hours.unwrap_or(24));
    let details = get_due_for_household(&state.pool, id, lookahead).await?;
    Ok(Json(details.iter().map(DoseView::from_detail).collect()))
}

async fn calendar_url(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> AppResult<Json<serde_json::Value>> {
    let household = sqlx::query_as::<_, Household>("SELECT * FROM households WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Семья не найдена".to_string()))?;
    let url = format!(
        "{}/calendar/{}.ics",
        state.settings.public_base_url.trim_end_matches('/'),
        household.calendar_token
    );
    Ok(Json(json!({ "calendar_url": url })))
}
