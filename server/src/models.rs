use chrono::{DateTime, NaiveTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "treatmentkind", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum TreatmentKind {
    Pill,
    Vaccine,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "dosestatus", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum DoseStatus {
    Planned,
    Reminded,
    Taken,
    Missed,
    Skipped,
}

impl DoseStatus {
    /// Человекочитаемая метка статуса (для iCal/UI).
    pub fn label(self) -> &'static str {
        match self {
            DoseStatus::Planned => "запланировано",
            DoseStatus::Reminded => "напоминание отправлено",
            DoseStatus::Taken => "принято",
            DoseStatus::Missed => "пропущено",
            DoseStatus::Skipped => "пропущено вручную",
        }
    }
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Household {
    pub id: i32,
    pub name: String,
    pub calendar_token: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct FamilyMember {
    pub id: i32,
    pub household_id: i32,
    pub display_name: String,
    pub telegram_user_id: Option<i64>,
    pub notify: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Dog {
    pub id: i32,
    pub household_id: i32,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Treatment {
    pub id: i32,
    pub dog_id: i32,
    pub name: String,
    pub kind: TreatmentKind,
    pub dose_label: Option<String>,
    pub cycle_days: i32,
    pub start_at: DateTime<Utc>,
    pub reminder_time: NaiveTime,
    pub instructions: Option<String>,
    pub active: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Dose {
    pub id: i32,
    pub treatment_id: i32,
    pub due_at: DateTime<Utc>,
    pub status: DoseStatus,
    pub api_key: Option<String>,
    pub reminded_at: Option<DateTime<Utc>>,
    pub taken_at: Option<DateTime<Utc>>,
    pub confirmed_by_member_id: Option<i32>,
    pub note: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Доза вместе с данными назначения и собаки — то, что нужно для напоминаний,
/// календаря и отчётов (аналог selectinload-загрузок в Python).
#[derive(Debug, Clone)]
pub struct DoseDetail {
    pub dose: Dose,
    pub treatment: Treatment,
    pub dog_name: String,
    pub household_id: i32,
}

/// Плоское представление дозы для JSON-ответов API (без секретного api_key).
#[derive(Debug, Clone, Serialize)]
pub struct DoseView {
    pub id: i32,
    pub treatment_id: i32,
    pub treatment_name: String,
    pub dog_name: String,
    pub dose_label: Option<String>,
    pub instructions: Option<String>,
    pub due_at: DateTime<Utc>,
    pub status: DoseStatus,
    pub reminded_at: Option<DateTime<Utc>>,
    pub taken_at: Option<DateTime<Utc>>,
    pub note: Option<String>,
}

impl DoseView {
    pub fn from_detail(d: &DoseDetail) -> Self {
        Self {
            id: d.dose.id,
            treatment_id: d.treatment.id,
            treatment_name: d.treatment.name.clone(),
            dog_name: d.dog_name.clone(),
            dose_label: d.treatment.dose_label.clone(),
            instructions: d.treatment.instructions.clone(),
            due_at: d.dose.due_at,
            status: d.dose.status,
            reminded_at: d.dose.reminded_at,
            taken_at: d.dose.taken_at,
            note: d.dose.note.clone(),
        }
    }
}
