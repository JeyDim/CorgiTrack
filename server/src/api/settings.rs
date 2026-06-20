use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;

use crate::error::{AppError, AppResult};
use crate::models::AppSettings;
use crate::services::settings::{self, SettingsPatch};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/settings", get(get_settings).patch(update_settings))
}

#[derive(Debug, Deserialize)]
pub struct UpdateSettings {
    pub escalation_first_delay_minutes: Option<i32>,
    pub escalation_step_minutes: Option<i32>,
    pub reminder_lookahead_minutes: Option<i32>,
    pub scheduler_tick_seconds: Option<i32>,
}

async fn get_settings(State(state): State<AppState>) -> AppResult<Json<AppSettings>> {
    Ok(Json(settings::get(&state.pool).await?))
}

async fn update_settings(
    State(state): State<AppState>,
    Json(body): Json<UpdateSettings>,
) -> AppResult<Json<AppSettings>> {
    // Минимальные адекватные значения, чтобы шедулер не «крутился» вхолостую.
    check_min(
        "escalation_first_delay_minutes",
        body.escalation_first_delay_minutes,
        1,
    )?;
    check_min("escalation_step_minutes", body.escalation_step_minutes, 1)?;
    check_min(
        "reminder_lookahead_minutes",
        body.reminder_lookahead_minutes,
        0,
    )?;
    check_min("scheduler_tick_seconds", body.scheduler_tick_seconds, 1)?;

    let patch = SettingsPatch {
        escalation_first_delay_minutes: body.escalation_first_delay_minutes,
        escalation_step_minutes: body.escalation_step_minutes,
        reminder_lookahead_minutes: body.reminder_lookahead_minutes,
        scheduler_tick_seconds: body.scheduler_tick_seconds,
    };
    if patch.is_empty() {
        return Err(AppError::BadRequest(
            "не передано ни одного поля для обновления".to_string(),
        ));
    }
    Ok(Json(settings::update(&state.pool, patch).await?))
}

fn check_min(field: &str, value: Option<i32>, min: i32) -> AppResult<()> {
    if let Some(v) = value {
        if v < min {
            return Err(AppError::BadRequest(format!(
                "{field} должно быть не меньше {min}"
            )));
        }
    }
    Ok(())
}
