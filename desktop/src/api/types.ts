// Типы зеркалят serde-сериализацию бэкенда (server/src/models.rs).
// Перечисления сериализуются в lowercase; даты — RFC3339-строки.

export type TreatmentKind = "pill" | "vaccine";

/** Категория таблетки (для kind = "pill"): от клещей / от гельминтов. */
export type PillCategory = "tick" | "worm";

export type DoseStatus =
  | "planned"
  | "reminded"
  | "taken"
  | "missed"
  | "skipped";

export interface Household {
  id: number;
  name: string;
  calendar_token: string;
  created_at: string;
}

export interface Dog {
  id: number;
  household_id: number;
  name: string;
  created_at: string;
}

export interface FamilyMember {
  id: number;
  household_id: number;
  display_name: string;
  telegram_user_id: number | null;
  notify: boolean;
  /** Порядок обзвона при эскалации: 0 уведомляется первым, затем 1, 2, ... */
  escalation_order: number;
  created_at: string;
}

/** Частичное обновление члена семьи: переданы только изменяемые поля. */
export interface UpdateMember {
  display_name?: string;
  telegram_user_id?: number | null;
  notify?: boolean;
  escalation_order?: number;
}

export interface AppSettings {
  /** Пауза после первого напоминания до повторного вопроса тому же человеку (минуты). */
  escalation_first_delay_minutes: number;
  /** Пауза между последующими шагами эскалации — повтор/следующий по списку (минуты). */
  escalation_step_minutes: number;
  /** За сколько до приёма слать первое напоминание (минуты). */
  reminder_lookahead_minutes: number;
  /** Период фонового шедулера напоминаний (секунды). */
  scheduler_tick_seconds: number;
  updated_at: string;
}

/** Частичное обновление настроек: переданы только изменяемые поля. */
export type UpdateAppSettings = Partial<
  Omit<AppSettings, "updated_at">
>;

export interface Treatment {
  id: number;
  dog_id: number;
  name: string;
  kind: TreatmentKind;
  /** Категория таблетки (для kind = "pill"). NULL у прививок и старых записей. */
  category: PillCategory | null;
  dose_label: string | null;
  cycle_days: number;
  start_at: string;
  /** Формат HH:MM:SS. */
  reminder_time: string;
  instructions: string | null;
  active: boolean;
  /** Текущая ветклиника назначения (для прививок). */
  clinic: string | null;
  created_at: string;
}

/** Плоское представление дозы для UI (без секретного api_key). */
export interface DoseView {
  id: number;
  treatment_id: number;
  treatment_name: string;
  dog_name: string;
  dose_label: string | null;
  instructions: string | null;
  due_at: string;
  status: DoseStatus;
  reminded_at: string | null;
  taken_at: string | null;
  note: string | null;
  /** Ветклиника записи: снимок дозы, для старых записей — клиника лечения. */
  clinic: string | null;
}

/** Сырой ряд дозы, который возвращает POST /doses/{id}/status. */
export interface Dose {
  id: number;
  treatment_id: number;
  due_at: string;
  status: DoseStatus;
  reminded_at: string | null;
  taken_at: string | null;
  confirmed_by_member_id: number | null;
  note: string | null;
  created_at: string;
}

// ---- payloads ----

export interface CreateTreatment {
  dog_id: number;
  name: string;
  kind: TreatmentKind;
  category?: PillCategory | null;
  dose_label?: string | null;
  cycle_days: number;
  start_at: string;
  reminder_time?: string | null;
  instructions?: string | null;
  active?: boolean | null;
  clinic?: string | null;
}

export type UpdateTreatment = Partial<Omit<CreateTreatment, "dog_id">>;

export interface StatusUpdate {
  status: DoseStatus;
  note?: string | null;
  member_id?: number | null;
}

export interface DoseFilter {
  household_id?: number;
  from?: string;
  to?: string;
  status?: DoseStatus;
}

// ---- человекочитаемые метки (для UI) ----

export const DOSE_STATUS_LABEL: Record<DoseStatus, string> = {
  planned: "Запланировано",
  reminded: "Напоминание отправлено",
  taken: "Принято",
  missed: "Пропущено",
  skipped: "Пропущено вручную",
};

export const TREATMENT_KIND_LABEL: Record<TreatmentKind, string> = {
  pill: "Таблетка",
  vaccine: "Прививка",
};

export const PILL_CATEGORY_LABEL: Record<PillCategory, string> = {
  tick: "От клещей",
  worm: "От гельминтов",
};
