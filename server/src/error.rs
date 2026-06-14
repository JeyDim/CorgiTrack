use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

/// Ошибки уровня API. Конвертируются в JSON-ответ с нужным статусом.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("не найдено: {0}")]
    NotFound(String),

    #[error("неверный запрос: {0}")]
    BadRequest(String),

    #[error("не авторизовано")]
    Unauthorized,

    #[error(transparent)]
    Database(#[from] sqlx::Error),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, "не авторизовано".to_string()),
            AppError::Database(err) => {
                tracing::error!(error = %err, "ошибка базы данных");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "внутренняя ошибка".to_string(),
                )
            }
            AppError::Other(err) => {
                tracing::error!(error = %err, "внутренняя ошибка");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "внутренняя ошибка".to_string(),
                )
            }
        };
        (status, Json(json!({ "detail": message }))).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;
