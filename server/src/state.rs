use std::sync::Arc;

use sqlx::PgPool;

use crate::config::Settings;

/// Общее состояние, прокидываемое в axum-хендлеры.
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub settings: Arc<Settings>,
}
