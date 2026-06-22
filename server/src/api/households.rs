use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;

use crate::error::{AppError, AppResult};
use crate::models::Household;
use crate::state::AppState;
use crate::util::generate_calendar_token;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/households", get(list).post(create))
        .route("/households/{id}", get(get_one).patch(update).delete(delete))
}

#[derive(Debug, Deserialize)]
pub struct CreateHousehold {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateHousehold {
    pub name: Option<String>,
}

async fn list(State(state): State<AppState>) -> AppResult<Json<Vec<Household>>> {
    let rows = sqlx::query_as::<_, Household>("SELECT * FROM households ORDER BY id")
        .fetch_all(&state.pool)
        .await?;
    Ok(Json(rows))
}

async fn create(
    State(state): State<AppState>,
    Json(body): Json<CreateHousehold>,
) -> AppResult<Json<Household>> {
    let row = sqlx::query_as::<_, Household>(
        "INSERT INTO households (name, calendar_token) VALUES ($1, $2) RETURNING *",
    )
    .bind(body.name)
    .bind(generate_calendar_token())
    .fetch_one(&state.pool)
    .await?;
    Ok(Json(row))
}

async fn get_one(State(state): State<AppState>, Path(id): Path<i32>) -> AppResult<Json<Household>> {
    sqlx::query_as::<_, Household>("SELECT * FROM households WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.pool)
        .await?
        .map(Json)
        .ok_or_else(|| AppError::NotFound("Семья не найдена".to_string()))
}

async fn update(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(body): Json<UpdateHousehold>,
) -> AppResult<Json<Household>> {
    sqlx::query_as::<_, Household>(
        "UPDATE households SET name = COALESCE($2, name) WHERE id = $1 RETURNING *",
    )
    .bind(id)
    .bind(body.name)
    .fetch_optional(&state.pool)
    .await?
    .map(Json)
    .ok_or_else(|| AppError::NotFound("Семья не найдена".to_string()))
}

async fn delete(State(state): State<AppState>, Path(id): Path<i32>) -> AppResult<Json<Value>> {
    let mut tx = state.pool.begin().await?;

    // Удаление семьи убирает её собак, назначения и дозы целиком. Дозы удаляем явно,
    // чтобы каскад не оставил их «сиротами» через ON DELETE SET NULL на treatment_id.
    sqlx::query(
        "DELETE FROM doses WHERE treatment_id IN ( \
            SELECT t.id FROM treatments t JOIN dogs g ON g.id = t.dog_id \
            WHERE g.household_id = $1)",
    )
    .bind(id)
    .execute(&mut *tx)
    .await?;

    let result = sqlx::query("DELETE FROM households WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Семья не найдена".to_string()));
    }

    tx.commit().await?;
    Ok(Json(serde_json::json!({ "deleted": id })))
}

use serde_json::Value;
