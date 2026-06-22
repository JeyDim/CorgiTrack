use axum::extract::{Path, Query, State};
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::error::{AppError, AppResult};
use crate::models::Dog;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/dogs", get(list).post(create))
        .route("/dogs/{id}", get(get_one).patch(update).delete(delete))
}

#[derive(Debug, Deserialize)]
pub struct DogFilter {
    pub household_id: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct CreateDog {
    pub household_id: i32,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateDog {
    pub name: Option<String>,
}

async fn list(
    State(state): State<AppState>,
    Query(filter): Query<DogFilter>,
) -> AppResult<Json<Vec<Dog>>> {
    let rows = match filter.household_id {
        Some(hh) => {
            sqlx::query_as::<_, Dog>("SELECT * FROM dogs WHERE household_id = $1 ORDER BY id")
                .bind(hh)
                .fetch_all(&state.pool)
                .await?
        }
        None => {
            sqlx::query_as::<_, Dog>("SELECT * FROM dogs ORDER BY id")
                .fetch_all(&state.pool)
                .await?
        }
    };
    Ok(Json(rows))
}

async fn create(
    State(state): State<AppState>,
    Json(body): Json<CreateDog>,
) -> AppResult<Json<Dog>> {
    let row = sqlx::query_as::<_, Dog>(
        "INSERT INTO dogs (household_id, name) VALUES ($1, $2) RETURNING *",
    )
    .bind(body.household_id)
    .bind(body.name)
    .fetch_one(&state.pool)
    .await?;
    Ok(Json(row))
}

async fn get_one(State(state): State<AppState>, Path(id): Path<i32>) -> AppResult<Json<Dog>> {
    sqlx::query_as::<_, Dog>("SELECT * FROM dogs WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.pool)
        .await?
        .map(Json)
        .ok_or_else(|| AppError::NotFound("Собака не найдена".to_string()))
}

async fn update(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(body): Json<UpdateDog>,
) -> AppResult<Json<Dog>> {
    sqlx::query_as::<_, Dog>("UPDATE dogs SET name = COALESCE($2, name) WHERE id = $1 RETURNING *")
        .bind(id)
        .bind(body.name)
        .fetch_optional(&state.pool)
        .await?
        .map(Json)
        .ok_or_else(|| AppError::NotFound("Собака не найдена".to_string()))
}

async fn delete(State(state): State<AppState>, Path(id): Path<i32>) -> AppResult<Json<Value>> {
    let mut tx = state.pool.begin().await?;

    // Удаление собаки убирает её назначения и дозы целиком. Дозы удаляем явно: иначе
    // каскад dogs → treatments оставил бы их «сиротами» (treatment_id → NULL по
    // ON DELETE SET NULL). SET NULL предназначен только для удаления самого назначения.
    sqlx::query(
        "DELETE FROM doses WHERE treatment_id IN (SELECT id FROM treatments WHERE dog_id = $1)",
    )
    .bind(id)
    .execute(&mut *tx)
    .await?;

    let result = sqlx::query("DELETE FROM dogs WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Собака не найдена".to_string()));
    }

    tx.commit().await?;
    Ok(Json(json!({ "deleted": id })))
}
