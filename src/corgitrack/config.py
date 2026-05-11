from pydantic_settings import BaseSettings, SettingsConfigDict


class Settings(BaseSettings):
    database_url: str = "postgresql+asyncpg://corgitrack:corgitrack@localhost:5432/corgitrack"
    public_base_url: str = "http://localhost:8000"
    app_timezone: str = "Europe/Astrakhan"
    telegram_bot_token: str | None = None
    telegram_api_server_url: str | None = "https://tgproxy.advsrvone.pw/"
    telegram_webhook_secret: str = "change-me"
    missed_grace_minutes: int = 120
    reminder_lookahead_minutes: int = 30
    scheduler_tick_seconds: int = 60

    model_config = SettingsConfigDict(env_file=".env", env_file_encoding="utf-8")


settings = Settings()
