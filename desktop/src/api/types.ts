// Типы зеркалят serde-сериализацию бэкенда (server/src/models.rs).
// Перечисления сериализуются в lowercase; даты — RFC3339-строки.

export type TreatmentKind = "pill" | "vaccine";

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
  created_at: string;
}

export interface Treatment {
  id: number;
  dog_id: number;
  name: string;
  kind: TreatmentKind;
  dose_label: string | null;
  cycle_days: number;
  start_at: string;
  /** Формат HH:MM:SS. */
  reminder_time: string;
  instructions: string | null;
  active: boolean;
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
  dose_label?: string | null;
  cycle_days: number;
  start_at: string;
  reminder_time?: string | null;
  instructions?: string | null;
  active?: boolean | null;
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
