use sqlx::PgPool;

use crate::models::AppSettings;

/// Частичное обновление настроек: None — поле не меняется.
#[derive(Debug, Default, Clone)]
pub struct SettingsPatch {
    pub escalation_first_delay_minutes: Option<i32>,
    pub escalation_step_minutes: Option<i32>,
    pub reminder_lookahead_minutes: Option<i32>,
    pub scheduler_tick_seconds: Option<i32>,
}

impl SettingsPatch {
    pub fn is_empty(&self) -> bool {
        self.escalation_first_delay_minutes.is_none()
            && self.escalation_step_minutes.is_none()
            && self.reminder_lookahead_minutes.is_none()
            && self.scheduler_tick_seconds.is_none()
    }
}

/// Текущие глобальные настройки. Строка id = 1 засевается в bootstrap; на всякий
/// случай создаём её здесь, чтобы сервис работал даже на «старой» БД.
pub async fn get(pool: &PgPool) -> Result<AppSettings, sqlx::Error> {
    sqlx::query("INSERT INTO app_settings (id) VALUES (1) ON CONFLICT (id) DO NOTHING")
        .execute(pool)
        .await?;
    sqlx::query_as::<_, AppSettings>("SELECT * FROM app_settings WHERE id = 1")
        .fetch_one(pool)
        .await
}

/// Применить частичное обновление и вернуть актуальные настройки.
pub async fn update(pool: &PgPool, patch: SettingsPatch) -> Result<AppSettings, sqlx::Error> {
    sqlx::query("INSERT INTO app_settings (id) VALUES (1) ON CONFLICT (id) DO NOTHING")
        .execute(pool)
        .await?;
    sqlx::query_as::<_, AppSettings>(
        "UPDATE app_settings SET \
            escalation_first_delay_minutes = COALESCE($1, escalation_first_delay_minutes), \
            escalation_step_minutes = COALESCE($2, escalation_step_minutes), \
            reminder_lookahead_minutes = COALESCE($3, reminder_lookahead_minutes), \
            scheduler_tick_seconds = COALESCE($4, scheduler_tick_seconds), \
            updated_at = now() \
         WHERE id = 1 RETURNING *",
    )
    .bind(patch.escalation_first_delay_minutes)
    .bind(patch.escalation_step_minutes)
    .bind(patch.reminder_lookahead_minutes)
    .bind(patch.scheduler_tick_seconds)
    .fetch_one(pool)
    .await
}
