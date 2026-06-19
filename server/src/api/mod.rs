pub mod calendar;
pub mod dogs;
pub mod doses;
pub mod health;
pub mod households;
pub mod members;
pub mod reports;
pub mod settings;
pub mod treatments;

use axum::routing::get;
use axum::{middleware, Router};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::auth::require_service_token;
use crate::state::AppState;

/// Полный роутер приложения: публичные маршруты + защищённый /api/v1/**.
pub fn router(state: AppState) -> Router {
    let public = Router::new()
        .route("/health", get(health::health))
        .route("/calendar/:filename", get(calendar::calendar_ics))
        .route(
            "/api/doses/:dose_id/taken",
            get(doses::mark_taken_public).post(doses::mark_taken_public),
        );

    let protected = Router::new()
        .merge(households::router())
        .merge(dogs::router())
        .merge(members::router())
        .merge(treatments::router())
        .merge(doses::protected_router())
        .merge(reports::router())
        .merge(settings::router())
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_service_token,
        ));

    Router::new()
        .merge(public)
        .nest("/api/v1", protected)
        // Семейный локальный инструмент: разрешаем любой источник (в т.ч. Tauri webview).
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
