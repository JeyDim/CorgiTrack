use axum::extract::{Request, State};
use axum::http::header::AUTHORIZATION;
use axum::middleware::Next;
use axum::response::Response;

use crate::error::AppError;
use crate::state::AppState;

/// Middleware для защищённых /api/v1/** маршрутов: требует
/// `Authorization: Bearer <SERVICE_TOKEN>`.
pub async fn require_service_token(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let provided = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::trim);

    match provided {
        Some(token) if ct_eq(token.as_bytes(), state.settings.service_token.as_bytes()) => {
            Ok(next.run(req).await)
        }
        _ => Err(AppError::Unauthorized),
    }
}

/// Сравнение за постоянное время (защита от timing-атак на токен).
fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}
