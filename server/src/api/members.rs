use axum::extract::{Path, Query, State};
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::error::{AppError, AppResult};
use crate::models::FamilyMember;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/members", get(list).post(create))
        .route("/members/{id}", get(get_one).patch(update).delete(delete))
}

#[derive(Debug, Deserialize)]
pub struct MemberFilter {
    pub household_id: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct CreateMember {
    pub household_id: i32,
    pub display_name: String,
    pub telegram_user_id: Option<i64>,
    pub notify: Option<bool>,
    pub escalation_order: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateMember {
    pub display_name: Option<String>,
    pub telegram_user_id: Option<i64>,
    pub notify: Option<bool>,
    pub escalation_order: Option<i32>,
}

async fn list(
    State(state): State<AppState>,
    Query(filter): Query<MemberFilter>,
) -> AppResult<Json<Vec<FamilyMember>>> {
    let rows = match filter.household_id {
        Some(hh) => {
            sqlx::query_as::<_, FamilyMember>(
                "SELECT * FROM family_members WHERE household_id = $1 ORDER BY id",
            )
            .bind(hh)
            .fetch_all(&state.pool)
            .await?
        }
        None => {
            sqlx::query_as::<_, FamilyMember>("SELECT * FROM family_members ORDER BY id")
                .fetch_all(&state.pool)
                .await?
        }
    };
    Ok(Json(rows))
}

async fn create(
    State(state): State<AppState>,
    Json(body): Json<CreateMember>,
) -> AppResult<Json<FamilyMember>> {
    let row = sqlx::query_as::<_, FamilyMember>(
        "INSERT INTO family_members \
            (household_id, display_name, telegram_user_id, notify, escalation_order) \
         VALUES ($1, $2, $3, COALESCE($4, TRUE), COALESCE($5, 0)) RETURNING *",
    )
    .bind(body.household_id)
    .bind(body.display_name)
    .bind(body.telegram_user_id)
    .bind(body.notify)
    .bind(body.escalation_order)
    .fetch_one(&state.pool)
    .await?;
    Ok(Json(row))
}

async fn get_one(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> AppResult<Json<FamilyMember>> {
    sqlx::query_as::<_, FamilyMember>("SELECT * FROM family_members WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.pool)
        .await?
        .map(Json)
        .ok_or_else(|| AppError::NotFound("Участник не найден".to_string()))
}

async fn update(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(body): Json<UpdateMember>,
) -> AppResult<Json<FamilyMember>> {
    sqlx::query_as::<_, FamilyMember>(
        "UPDATE family_members SET \
            display_name = COALESCE($2, display_name), \
            telegram_user_id = COALESCE($3, telegram_user_id), \
            notify = COALESCE($4, notify), \
            escalation_order = COALESCE($5, escalation_order) \
         WHERE id = $1 RETURNING *",
    )
    .bind(id)
    .bind(body.display_name)
    .bind(body.telegram_user_id)
    .bind(body.notify)
    .bind(body.escalation_order)
    .fetch_optional(&state.pool)
    .await?
    .map(Json)
    .ok_or_else(|| AppError::NotFound("Участник не найден".to_string()))
}

async fn delete(State(state): State<AppState>, Path(id): Path<i32>) -> AppResult<Json<Value>> {
    let result = sqlx::query("DELETE FROM family_members WHERE id = $1")
        .bind(id)
        .execute(&state.pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Участник не найден".to_string()));
    }
    Ok(Json(json!({ "deleted": id })))
}
