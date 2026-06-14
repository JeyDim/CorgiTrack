import { defineStore } from "pinia";
import { load, type Store } from "@tauri-apps/plugin-store";
import { CorgiApi } from "../api/client";

// Локальный файл настроек (в каталоге данных приложения Tauri).
let tauriStore: Store | null = null;
async function getStore(): Promise<Store> {
  if (!tauriStore) {
    tauriStore = await load("settings.json", { autoSave: false, defaults: {} });
  }
  return tauriStore;
}

interface SettingsState {
  baseUrl: string;
  token: string;
  householdId: number | null;
  /** true после первой попытки загрузки (чтобы не моргать редиректом). */
  loaded: boolean;
}

export const useSettingsStore = defineStore("settings", {
  state: (): SettingsState => ({
    baseUrl: "",
    token: "",
    householdId: null,
    loaded: false,
  }),

  getters: {
    configured: (s): boolean => s.baseUrl.trim() !== "" && s.token.trim() !== "",
  },

  actions: {
    async load() {
      try {
        const store = await getStore();
        this.baseUrl = (await store.get<string>("baseUrl")) ?? "";
        this.token = (await store.get<string>("token")) ?? "";
        this.householdId =
          (await store.get<number | null>("householdId")) ?? null;
      } catch (e) {
        // Вне Tauri-рантайма (например, открыли vite-сервер в обычном
        // браузере) плагин недоступен — это не критично.
        console.warn("Не удалось загрузить настройки:", e);
      } finally {
        this.loaded = true;
      }
    },

    async persist() {
      const store = await getStore();
      await store.set("baseUrl", this.baseUrl);
      await store.set("token", this.token);
      await store.set("householdId", this.householdId);
      await store.save();
    },

    async setConnection(baseUrl: string, token: string) {
      this.baseUrl = baseUrl.trim();
      this.token = token.trim();
      await this.persist();
    },

    async setHousehold(id: number | null) {
      this.householdId = id;
      await this.persist();
    },

    /** Свежий API-клиент с текущими baseUrl/token. */
    api(): CorgiApi {
      return new CorgiApi({ baseUrl: this.baseUrl, token: this.token });
    },
  },
});
