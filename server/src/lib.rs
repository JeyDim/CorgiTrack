pub mod api;
pub mod auth;
pub mod bot;
pub mod config;
pub mod db;
pub mod error;
pub mod models;
pub mod services;
pub mod state;
pub mod util;

use std::sync::Arc;

use anyhow::Context;
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

use crate::config::Settings;
use crate::state::AppState;

/// Точка входа приложения: БД -> схема -> HTTP-сервер (+ бот при наличии токена).
pub async fn run() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let settings = Settings::from_env();
    if settings.service_token == "change-me" {
        tracing::warn!("SERVICE_TOKEN не задан — используется небезопасное значение по умолчанию");
    }

    let pool = db::connect(&settings.database_url)
        .await
        .context("не удалось подключиться к базе данных")?;
    db::bootstrap(&pool)
        .await
        .context("не удалось инициализировать схему")?;

    let state = AppState {
        pool,
        settings: Arc::new(settings),
    };

    let app = api::router(state.clone());
    let listener = TcpListener::bind(state.settings.bind_addr.as_str())
        .await
        .with_context(|| format!("не удалось привязаться к {}", state.settings.bind_addr))?;
    tracing::info!(
        "CorgiTrack API слушает на http://{}",
        state.settings.bind_addr
    );

    let server = async {
        axum::serve(listener, app)
            .await
            .context("ошибка HTTP-сервера")
    };

    let bot_enabled = state.settings.telegram_bot_token.is_some();
    let bot_state = state.clone();
    let bot_task = async move {
        if bot_enabled {
            bot::run(bot_state).await.context("ошибка Telegram-бота")
        } else {
            tracing::info!("TELEGRAM_BOT_TOKEN не задан — бот отключён");
            std::future::pending::<()>().await;
            Ok(())
        }
    };

    tokio::select! {
        result = server => result?,
        result = bot_task => result?,
    }
    Ok(())
}
