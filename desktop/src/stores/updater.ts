import { defineStore } from "pinia";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

// Сырой объект обновления держим вне реактивного state: у него есть методы
// (downloadAndInstall) и внутренний resource id, оборачивать его в Proxy не нужно.
let pending: Update | null = null;

// Проверяем на старте и раз в час.
const CHECK_INTERVAL_MS = 60 * 60 * 1000;

export const useUpdaterStore = defineStore("updater", {
  state: () => ({
    available: false, // вышла версия новее текущей
    version: "", // версия из манифеста latest.json
    notes: "" as string | undefined, // тело релиза (changelog)
    dismissed: false, // пользователь закрыл баннер
    installing: false, // идёт загрузка/установка
    progress: 0, // 0..100; 0 — пока неизвестен размер
  }),

  actions: {
    // Тихая проверка обновлений. Любые ошибки (нет сети, запуск вне Tauri в
    // dev-режиме через браузер) гасим — обновление не критично для работы.
    async check() {
      if (this.installing) return;
      try {
        const update = await check();
        if (update) {
          pending = update;
          this.available = true;
          this.version = update.version;
          this.notes = update.body;
          this.dismissed = false; // новая версия — показываем баннер снова
        }
      } catch (e) {
        console.warn("[updater] проверка обновлений не удалась:", e);
      }
    },

    // Запуск из App.vue: сразу проверяем и дальше раз в час.
    startAuto() {
      void this.check();
      window.setInterval(() => void this.check(), CHECK_INTERVAL_MS);
    },

    dismiss() {
      this.dismissed = true;
    },

    // Скачать и установить. По завершении перезапускаем приложение.
    // Бросает ошибку наверх — баннер покажет тост и предложит ручную загрузку.
    async install() {
      if (!pending || this.installing) return;
      this.installing = true;
      this.progress = 0;

      let total = 0;
      let received = 0;
      try {
        await pending.downloadAndInstall((event) => {
          switch (event.event) {
            case "Started":
              total = event.data.contentLength ?? 0;
              break;
            case "Progress":
              received += event.data.chunkLength;
              this.progress = total
                ? Math.round((received / total) * 100)
                : 0;
              break;
            case "Finished":
              this.progress = 100;
              break;
          }
        });
        await relaunch();
      } catch (e) {
        this.installing = false;
        throw e;
      }
    },
  },
});
