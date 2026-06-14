use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::offset::LocalResult;
use chrono::TimeZone;
use chrono::{DateTime, NaiveDateTime, Utc};
use chrono_tz::Tz;
use rand::RngCore;
use uuid::Uuid;

/// Аналог secrets.token_urlsafe(32) из Python: 32 случайных байта в base64url.
pub fn generate_api_key() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

/// Аналог uuid4().hex — 32 шестнадцатеричных символа.
pub fn generate_calendar_token() -> String {
    Uuid::new_v4().simple().to_string()
}

/// Разбор таймзоны из настроек; при ошибке — UTC.
pub fn timezone(name: &str) -> Tz {
    name.parse().unwrap_or(Tz::UTC)
}

/// Локальное naive-время в нужной зоне -> UTC. При DST-неоднозначности берём
/// раннее значение (как fold=0 в Python), при «дыре» — трактуем как UTC.
pub fn local_to_utc(tz: Tz, naive: NaiveDateTime) -> DateTime<Utc> {
    match tz.from_local_datetime(&naive) {
        LocalResult::Single(dt) => dt.with_timezone(&Utc),
        LocalResult::Ambiguous(dt, _) => dt.with_timezone(&Utc),
        LocalResult::None => DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc),
    }
}
