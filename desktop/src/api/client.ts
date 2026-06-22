// HTTP-клиент CorgiTrack. Использует fetch из @tauri-apps/plugin-http
// (drop-in fetch через Rust-стек: без CORS/CSP/mixed-content проблем при
// произвольном адресе бэкенда, например http://<pi-в-локалке>:8000).

import { fetch } from "@tauri-apps/plugin-http";
import type {
  AppSettings,
  CreateMember,
  CreateTreatment,
  Dog,
  Dose,
  DoseFilter,
  DoseView,
  FamilyMember,
  Household,
  StatusUpdate,
  Treatment,
  UpdateAppSettings,
  UpdateMember,
  UpdateTreatment,
} from "./types";

export class ApiError extends Error {
  status: number;
  constructor(status: number, message: string) {
    super(message);
    this.name = "ApiError";
    this.status = status;
  }
}

export interface ApiConfig {
  baseUrl: string;
  token: string;
}

type Query = Record<string, string | number | boolean | undefined | null>;

interface ReqOptions {
  auth?: boolean;
  body?: unknown;
  query?: Query;
  raw?: boolean;
  bytes?: boolean;
}

export class CorgiApi {
  constructor(private cfg: ApiConfig) {}

  private buildUrl(path: string, query?: Query): string {
    const base = this.cfg.baseUrl.replace(/\/+$/, "");
    const url = new URL(base + path);
    if (query) {
      for (const [k, v] of Object.entries(query)) {
        if (v !== undefined && v !== null && v !== "") {
          url.searchParams.set(k, String(v));
        }
      }
    }
    return url.toString();
  }

  private async request<T>(
    method: string,
    path: string,
    opts: ReqOptions = {},
  ): Promise<T> {
    const { auth = true, body, query, raw = false, bytes = false } = opts;
    const headers: Record<string, string> = {};
    if (auth) headers["Authorization"] = `Bearer ${this.cfg.token}`;
    if (body !== undefined) headers["Content-Type"] = "application/json";

    let res: Response;
    try {
      res = await fetch(this.buildUrl(path, query), {
        method,
        headers,
        body: body !== undefined ? JSON.stringify(body) : undefined,
      });
    } catch (e) {
      throw new ApiError(0, `Сеть недоступна: ${(e as Error).message ?? e}`);
    }

    if (!res.ok) {
      let message = `${res.status} ${res.statusText}`;
      try {
        const data = (await res.json()) as { detail?: string };
        if (data?.detail) message = data.detail;
      } catch {
        // тело не JSON — оставляем статус
      }
      throw new ApiError(res.status, message);
    }

    if (bytes) return new Uint8Array(await res.arrayBuffer()) as unknown as T;
    if (raw) return (await res.text()) as unknown as T;
    if (res.status === 204) return undefined as T;
    return (await res.json()) as T;
  }

  // ---- публичные ----

  health(): Promise<{ status: string }> {
    return this.request("GET", "/health", { auth: false });
  }

  // ---- households ----

  listHouseholds(): Promise<Household[]> {
    return this.request("GET", "/api/v1/households");
  }

  calendarUrl(householdId: number): Promise<{ calendar_url: string }> {
    return this.request("GET", `/api/v1/households/${householdId}/calendar-url`);
  }

  // Сырые байты ответа (UTF-8 + BOM с сервера). Через res.text() BOM срезается
  // декодером, и Excel на русской локали читает CSV как cp1251 → кракозябры.
  reportCsv(householdId: number): Promise<Uint8Array> {
    return this.request("GET", `/api/v1/households/${householdId}/report.csv`, {
      bytes: true,
    });
  }

  // ---- doses ----

  getDue(householdId: number, lookaheadHours: number): Promise<DoseView[]> {
    return this.request("GET", `/api/v1/households/${householdId}/due`, {
      query: { lookahead_hours: lookaheadHours },
    });
  }

  listDoses(filter: DoseFilter = {}): Promise<DoseView[]> {
    return this.request("GET", "/api/v1/doses", { query: filter as Query });
  }

  setDoseStatus(doseId: number, body: StatusUpdate): Promise<Dose> {
    return this.request("POST", `/api/v1/doses/${doseId}/status`, { body });
  }

  // ---- treatments ----

  listTreatments(dogId?: number): Promise<Treatment[]> {
    return this.request("GET", "/api/v1/treatments", {
      query: { dog_id: dogId },
    });
  }

  createTreatment(body: CreateTreatment): Promise<Treatment> {
    return this.request("POST", "/api/v1/treatments", { body });
  }

  updateTreatment(id: number, body: UpdateTreatment): Promise<Treatment> {
    return this.request("PATCH", `/api/v1/treatments/${id}`, { body });
  }

  deleteTreatment(id: number): Promise<{ deleted: number }> {
    return this.request("DELETE", `/api/v1/treatments/${id}`);
  }

  // ---- dogs / members ----

  listDogs(householdId?: number): Promise<Dog[]> {
    return this.request("GET", "/api/v1/dogs", {
      query: { household_id: householdId },
    });
  }

  listMembers(householdId?: number): Promise<FamilyMember[]> {
    return this.request("GET", "/api/v1/members", {
      query: { household_id: householdId },
    });
  }

  createMember(body: CreateMember): Promise<FamilyMember> {
    return this.request("POST", "/api/v1/members", { body });
  }

  updateMember(id: number, patch: UpdateMember): Promise<FamilyMember> {
    return this.request("PATCH", `/api/v1/members/${id}`, { body: patch });
  }

  deleteMember(id: number): Promise<{ deleted: number }> {
    return this.request("DELETE", `/api/v1/members/${id}`);
  }

  // ---- settings ----

  getSettings(): Promise<AppSettings> {
    return this.request("GET", "/api/v1/settings");
  }

  updateSettings(patch: UpdateAppSettings): Promise<AppSettings> {
    return this.request("PATCH", "/api/v1/settings", { body: patch });
  }
}
