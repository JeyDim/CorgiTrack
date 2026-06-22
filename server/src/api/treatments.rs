use axum::extract::{Path, Query, State};
use axum::routing::get;
use axum::{Json, Router};
use chrono::{DateTime, NaiveTime, Utc};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::error::{AppError, AppResult};
use crate::models::{PillCategory, Treatment, TreatmentKind};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/treatments", get(list).post(create))
        .route("/treatments/{id}", get(get_one).patch(update).delete(delete))
}

#[derive(Debug, Deserialize)]
pub struct TreatmentFilter {
    pub dog_id: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct CreateTreatment {
    pub dog_id: i32,
    pub name: String,
    pub kind: TreatmentKind,
    /// Категория таблетки (для kind = Pill): tick / worm.
    pub category: Option<PillCategory>,
    pub dose_label: Option<String>,
    pub cycle_days: i32,
    pub start_at: DateTime<Utc>,
    /// Формат HH:MM:SS; по умолчанию 09:00:00.
    pub reminder_time: Option<NaiveTime>,
    pub instructions: Option<String>,
    pub active: Option<bool>,
    /// Ветклиника (для прививок).
    pub clinic: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTreatment {
    pub name: Option<String>,
    pub kind: Option<TreatmentKind>,
    pub category: Option<PillCategory>,
    pub dose_label: Option<String>,
    pub cycle_days: Option<i32>,
    pub start_at: Option<DateTime<Utc>>,
    pub reminder_time: Option<NaiveTime>,
    pub instructions: Option<String>,
    pub active: Option<bool>,
    pub clinic: Option<String>,
}

async fn list(
    State(state): State<AppState>,
    Query(filter): Query<TreatmentFilter>,
) -> AppResult<Json<Vec<Treatment>>> {
    let rows = match filter.dog_id {
        Some(dog_id) => {
            sqlx::query_as::<_, Treatment>("SELECT * FROM treatments WHERE dog_id = $1 ORDER BY id")
                .bind(dog_id)
                .fetch_all(&state.pool)
                .await?
        }
        None => {
            sqlx::query_as::<_, Treatment>("SELECT * FROM treatments ORDER BY id")
                .fetch_all(&state.pool)
                .await?
        }
    };
    Ok(Json(rows))
}

async fn create(
    State(state): State<AppState>,
    Json(body): Json<CreateTreatment>,
) -> AppResult<Json<Treatment>> {
    let row = sqlx::query_as::<_, Treatment>(
        "INSERT INTO treatments \
            (dog_id, name, kind, dose_label, cycle_days, start_at, reminder_time, instructions, active, clinic, category) \
         VALUES ($1, $2, $3, $4, $5, $6, COALESCE($7, '09:00'::time), $8, COALESCE($9, TRUE), $10, $11) \
         RETURNING *",
    )
    .bind(body.dog_id)
    .bind(body.name)
    .bind(body.kind)
    .bind(body.dose_label)
    .bind(body.cycle_days)
    .bind(body.start_at)
    .bind(body.reminder_time)
    .bind(body.instructions)
    .bind(body.active)
    .bind(body.clinic)
    .bind(body.category)
    .fetch_one(&state.pool)
    .await?;
    Ok(Json(row))
}

async fn get_one(State(state): State<AppState>, Path(id): Path<i32>) -> AppResult<Json<Treatment>> {
    sqlx::query_as::<_, Treatment>("SELECT * FROM treatments WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.pool)
        .await?
        .map(Json)
        .ok_or_else(|| AppError::NotFound("Назначение не найдено".to_string()))
}

async fn update(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(body): Json<UpdateTreatment>,
) -> AppResult<Json<Treatment>> {
    sqlx::query_as::<_, Treatment>(
        "UPDATE treatments SET \
            name = COALESCE($2, name), \
            kind = COALESCE($3, kind), \
            dose_label = COALESCE($4, dose_label), \
            cycle_days = COALESCE($5, cycle_days), \
            start_at = COALESCE($6, start_at), \
            reminder_time = COALESCE($7, reminder_time), \
            instructions = COALESCE($8, instructions), \
            active = COALESCE($9, active), \
            clinic = COALESCE($10, clinic), \
            category = COALESCE($11, category) \
         WHERE id = $1 RETURNING *",
    )
    .bind(id)
    .bind(body.name)
    .bind(body.kind)
    .bind(body.dose_label)
    .bind(body.cycle_days)
    .bind(body.start_at)
    .bind(body.reminder_time)
    .bind(body.instructions)
    .bind(body.active)
    .bind(body.clinic)
    .bind(body.category)
    .fetch_optional(&state.pool)
    .await?
    .map(Json)
    .ok_or_else(|| AppError::NotFound("Назначение не найдено".to_string()))
}

async fn delete(State(state): State<AppState>, Path(id): Path<i32>) -> AppResult<Json<Value>> {
    let mut tx = state.pool.begin().await?;

    // Перед удалением фиксируем снимок настроек назначения/собаки в ПРИНЯТЫЕ дозы,
    // чтобы после удаления (treatment_id → NULL по ON DELETE SET NULL) история
    // приёмов сохранила название, тип, дозу, клинику и принадлежность к собаке/семье.
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
         WHERE doses.treatment_id = $1 AND t.id = $1 AND doses.status = 'taken'",
    )
    .bind(id)
    .execute(&mut *tx)
    .await?;

    // Непринятые дозы (запланированные/напомненные/пропущенные) — это будущее
    // расписание, а не история приёмов. Удаляем их, чтобы не плодить «сирот»;
    // в живых остаётся только история принятых доз.
    sqlx::query("DELETE FROM doses WHERE treatment_id = $1 AND status <> 'taken'")
        .bind(id)
        .execute(&mut *tx)
        .await?;

    let result = sqlx::query("DELETE FROM treatments WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Назначение не найдено".to_string()));
    }

    tx.commit().await?;
    Ok(Json(json!({ "deleted": id })))
}
