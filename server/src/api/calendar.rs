use axum::extract::{Path, State};
use axum::http::header::CONTENT_TYPE;
use axum::response::{IntoResponse, Response};

use crate::error::{AppError, AppResult};
use crate::services::calendar::build_ical;
use crate::state::AppState;
use crate::util::timezone;

/// GET /calendar/{token}.ics — iCal-подписка (без авторизации).
pub async fn calendar_ics(
    State(state): State<AppState>,
    Path(filename): Path<String>,
) -> AppResult<Response> {
    let token = filename.strip_suffix(".ics").unwrap_or(&filename);
    let tz = timezone(&state.settings.app_timezone);

    match build_ical(&state.pool, tz, &state.settings.public_base_url, token).await? {
        Some(body) => Ok(([(CONTENT_TYPE, "text/calendar; charset=utf-8")], body).into_response()),
        None => Err(AppError::NotFound("Календарь не найден".to_string())),
    }
}
