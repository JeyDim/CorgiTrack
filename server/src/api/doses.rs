use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::QueryBuilder;

use crate::error::{AppError, AppResult};
use crate::models::{Dose, DoseDetail, DoseStatus, DoseView};
use crate::services::schedules::{
    mark_taken_by_api_key, row_to_detail, snapshot_treatment_into_dose, DETAIL_SELECT,
};
use crate::state::AppState;

pub fn protected_router() -> Router<AppState> {
    Router::new()
        .route("/doses", get(list_doses))
        .route("/doses/:id/status", post(update_status))
}

#[derive(Debug, Deserialize)]
pub struct KeyQuery {
    pub key: String,
}

/// GET|POST /api/doses/{id}/taken?key=... — публичная отметка приёма по api_key.
pub async fn mark_taken_public(
    State(state): State<AppState>,
    Path(dose_id): Path<i32>,
    Query(q): Query<KeyQuery>,
) -> AppResult<Json<Value>> {
    if q.key.len() < 16 {
        return Err(AppError::BadRequest("ключ слишком короткий".to_string()));
    }
    let dose = mark_taken_by_api_key(
        &state.pool,
        dose_id,
        &q.key,
        Some("Отмечено по ссылке из календаря"),
    )
    .await?;
    match dose {
        Some(d) => Ok(Json(json!({ "status": "taken", "dose_id": d.id }))),
        None => Err(AppError::NotFound(
            "Доза не найдена или ключ неверный".to_string(),
        )),
    }
}

#[derive(Debug, Deserialize)]
pub struct DoseFilter {
    pub household_id: Option<i32>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub status: Option<DoseStatus>,
}

/// GET /api/v1/doses — список доз с фильтрами.
pub async fn list_doses(
    State(state): State<AppState>,
    Query(filter): Query<DoseFilter>,
) -> AppResult<Json<Vec<DoseView>>> {
    let mut qb = QueryBuilder::new(DETAIL_SELECT);
    qb.push(" WHERE 1=1");
    if let Some(hh) = filter.household_id {
        // COALESCE со снимком: дозы удалённых назначений (g.household_id = NULL)
        // продолжают находиться по семье из снимка дозы.
        qb.push(" AND COALESCE(g.household_id, d.household_id) = ")
            .push_bind(hh);
    }
    if let Some(status) = filter.status {
        qb.push(" AND d.status = ").push_bind(status);
    }
    if let Some(from) = filter.from {
        qb.push(" AND d.due_at >= ").push_bind(from);
    }
    if let Some(to) = filter.to {
        qb.push(" AND d.due_at <= ").push_bind(to);
    }
    qb.push(" ORDER BY d.due_at");

    let rows = qb.build().fetch_all(&state.pool).await?;
    let details: Vec<DoseDetail> = rows.iter().map(row_to_detail).collect::<Result<_, _>>()?;
    Ok(Json(details.iter().map(DoseView::from_detail).collect()))
}

#[derive(Debug, Deserialize)]
pub struct StatusUpdate {
    pub status: DoseStatus,
    pub note: Option<String>,
    pub member_id: Option<i32>,
}

/// POST /api/v1/doses/{id}/status — изменить статус дозы (taken/skipped/...).
pub async fn update_status(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(body): Json<StatusUpdate>,
) -> AppResult<Json<Dose>> {
    let dose = sqlx::query_as::<_, Dose>(
        "UPDATE doses SET \
            status = $2, \
            note = COALESCE($3, note), \
            taken_at = CASE WHEN $2 = 'taken' THEN now() ELSE taken_at END, \
            confirmed_by_member_id = COALESCE($4, confirmed_by_member_id) \
         WHERE id = $1 RETURNING *",
    )
    .bind(id)
    .bind(body.status)
    .bind(body.note)
    .bind(body.member_id)
    .fetch_optional(&state.pool)
    .await?;
    let Some(mut dose) = dose else {
        return Err(AppError::NotFound("Доза не найдена".to_string()));
    };
    // В момент отметки «принято» фиксируем снимок настроек назначения и собаки в
    // саму дозу — дальше дозу/назначение можно править или удалять, не трогая историю.
    if dose.status == DoseStatus::Taken {
        snapshot_treatment_into_dose(&state.pool, dose.id).await?;
        dose = sqlx::query_as::<_, Dose>("SELECT * FROM doses WHERE id = $1")
            .bind(dose.id)
            .fetch_one(&state.pool)
            .await?;
    }
    Ok(Json(dose))
}
