use std::env;

/// Настройки приложения. Аналог `corgitrack.config.Settings` из Python-версии,
/// плюс `service_token` для защищённого API.
#[derive(Debug, Clone)]
pub struct Settings {
    pub database_url: String,
    pub public_base_url: String,
    pub app_timezone: String,
    pub bind_addr: String,
    /// Bearer-токен для всех /api/v1/** эндпоинтов.
    pub service_token: String,
    pub telegram_bot_token: Option<String>,
    /// Кастомный Bot API server (по умолчанию прокси, как в Python).
    /// Пустая строка в env очищает значение (прямой доступ к api.telegram.org).
    pub telegram_api_server_url: Option<String>,
}

impl Settings {
    pub fn from_env() -> Self {
        Self {
            database_url: env_or(
                "DATABASE_URL",
                "postgres://corgitrack:corgitrack@localhost:5432/corgitrack",
            ),
            public_base_url: env_or("PUBLIC_BASE_URL", "http://localhost:8000"),
            app_timezone: env_or("APP_TIMEZONE", "Europe/Astrakhan"),
            bind_addr: env_or("BIND_ADDR", "0.0.0.0:8000"),
            service_token: env_or("SERVICE_TOKEN", "change-me"),
            telegram_bot_token: env_opt("TELEGRAM_BOT_TOKEN"),
            telegram_api_server_url: telegram_api_server_url(),
        }
    }
}

fn env_or(key: &str, default: &str) -> String {
    match env::var(key) {
        Ok(value) if !value.trim().is_empty() => value,
        _ => default.to_string(),
    }
}

/// Возвращает значение только если переменная задана и непуста.
fn env_opt(key: &str) -> Option<String> {
    match env::var(key) {
        Ok(value) if !value.trim().is_empty() => Some(value),
        _ => None,
    }
}

/// Семантика как в Python: по умолчанию используется прокси Bot API server;
/// переменную можно переопределить или явно очистить (пустая строка).
fn telegram_api_server_url() -> Option<String> {
    const DEFAULT: &str = "https://tgproxy.advsrvone.pw/";
    match env::var("TELEGRAM_API_SERVER_URL") {
        // переменная не задана вовсе -> дефолтный прокси
        Err(_) => Some(DEFAULT.to_string()),
        // задана пустой -> прямой доступ к Telegram
        Ok(value) if value.trim().is_empty() => None,
        Ok(value) => Some(value),
    }
}
