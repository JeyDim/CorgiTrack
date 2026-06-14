use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

/// Идемпотентная схема (см. migrations/0001_bootstrap.sql). Встраивается в бинарь,
/// чтобы не зависеть от внешних файлов в рантайме.
const BOOTSTRAP_SQL: &str = include_str!("../migrations/0001_bootstrap.sql");

pub async fn connect(database_url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(5)
        .test_before_acquire(true) // аналог pool_pre_ping из SQLAlchemy
        .connect(database_url)
        .await
}

/// Создаёт типы/таблицы/индексы, если их ещё нет. Безопасно для существующих БД.
pub async fn bootstrap(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::raw_sql(BOOTSTRAP_SQL).execute(pool).await?;
    Ok(())
}
