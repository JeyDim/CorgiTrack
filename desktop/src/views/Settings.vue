<script setup lang="ts">
import { onMounted, ref } from "vue";
import { save } from "@tauri-apps/plugin-dialog";
import { writeFile } from "@tauri-apps/plugin-fs";
import { openUrl } from "@tauri-apps/plugin-opener";

import { CorgiApi } from "../api/client";
import type { Household } from "../api/types";
import { useSettingsStore } from "../stores/settings";
import { useToastStore } from "../stores/toast";

const settings = useSettingsStore();
const toast = useToastStore();

const baseUrl = ref(settings.baseUrl);
const token = ref(settings.token);
const showToken = ref(false);

const checking = ref(false);
const households = ref<Household[]>([]);
const selectedHousehold = ref<number | null>(settings.householdId);

const calendarUrl = ref<string | null>(null);
const downloading = ref(false);

onMounted(async () => {
  // Если уже настроено — подтянем список семей для выбора.
  if (settings.configured) {
    try {
      households.value = await settings.api().listHouseholds();
    } catch {
      /* молча: пользователь увидит при «Проверить» */
    }
  }
});

async function checkConnection() {
  if (!baseUrl.value.trim() || !token.value.trim()) {
    toast.error("Заполните адрес и service-токен");
    return;
  }
  checking.value = true;
  try {
    const api = new CorgiApi({
      baseUrl: baseUrl.value.trim(),
      token: token.value.trim(),
    });
    await api.health(); // адрес доступен?
    const list = await api.listHouseholds(); // токен валиден?

    await settings.setConnection(baseUrl.value, token.value);
    households.value = list;

    // Авто-выбор семьи, если она одна и ещё не выбрана.
    if (selectedHousehold.value == null && list.length === 1) {
      selectedHousehold.value = list[0].id;
      await settings.setHousehold(list[0].id);
    }
    toast.success("Подключение работает 🐾");
  } catch (e) {
    toast.error(`Не удалось подключиться: ${(e as Error).message}`);
  } finally {
    checking.value = false;
  }
}

async function chooseHousehold() {
  await settings.setHousehold(selectedHousehold.value);
  calendarUrl.value = null;
  toast.success("Семья выбрана");
}

async function showCalendarUrl() {
  if (settings.householdId == null) return;
  try {
    const res = await settings.api().calendarUrl(settings.householdId);
    calendarUrl.value = res.calendar_url;
  } catch (e) {
    toast.error(`Ошибка: ${(e as Error).message}`);
  }
}

async function copyCalendar() {
  if (!calendarUrl.value) return;
  try {
    await navigator.clipboard.writeText(calendarUrl.value);
    toast.success("Ссылка скопирована");
  } catch {
    toast.error("Не удалось скопировать");
  }
}

async function openCalendar() {
  if (!calendarUrl.value) return;
  try {
    await openUrl(calendarUrl.value);
  } catch (e) {
    toast.error(`Не удалось открыть: ${(e as Error).message}`);
  }
}

async function downloadCsv() {
  if (settings.householdId == null) {
    toast.error("Сначала выберите семью");
    return;
  }
  downloading.value = true;
  try {
    const csv = await settings.api().reportCsv(settings.householdId);
    const path = await save({
      defaultPath: "corgitrack-prinyatye-dozy.csv",
      filters: [{ name: "CSV", extensions: ["csv"] }],
    });
    if (!path) return; // пользователь отменил
    await writeFile(path, csv); // байты как есть, с BOM — иначе Excel ломает кириллицу
    toast.success("Отчёт сохранён");
  } catch (e) {
    toast.error(`Ошибка отчёта: ${(e as Error).message}`);
  } finally {
    downloading.value = false;
  }
}
</script>

<template>
  <div class="view">
    <header class="view-head">
      <h1>⚙️ Настройки</h1>
      <p class="muted">Подключение к серверу CorgiTrack и инструменты семьи.</p>
    </header>

    <section class="card pad">
      <h3>🔌 Подключение</h3>
      <div class="grid">
        <div class="field">
          <label>Адрес сервера</label>
          <input
            v-model="baseUrl"
            class="input"
            placeholder="http://localhost:8000"
            autocomplete="off"
            spellcheck="false"
          />
        </div>
        <div class="field">
          <label>Service-токен</label>
          <div class="token-row">
            <input
              v-model="token"
              class="input"
              :type="showToken ? 'text' : 'password'"
              placeholder="Bearer-токен из SERVICE_TOKEN"
              autocomplete="off"
              spellcheck="false"
            />
            <button
              class="btn btn-ghost btn-sm"
              type="button"
              @click="showToken = !showToken"
            >
              {{ showToken ? "🙈" : "👁" }}
            </button>
          </div>
        </div>
      </div>
      <div class="actions">
        <button class="btn btn-primary" :disabled="checking" @click="checkConnection">
          {{ checking ? "Проверяю…" : "Проверить и сохранить" }}
        </button>
      </div>
    </section>

    <section v-if="households.length" class="card pad">
      <h3>🏠 Семья</h3>
      <p class="muted small">
        Выбранная семья используется на дашборде и в отчётах.
      </p>
      <div class="row wrap">
        <select v-model.number="selectedHousehold" class="select household-select">
          <option :value="null" disabled>— выберите семью —</option>
          <option v-for="h in households" :key="h.id" :value="h.id">
            {{ h.name }} (#{{ h.id }})
          </option>
        </select>
        <button
          class="btn btn-primary"
          :disabled="selectedHousehold == null"
          @click="chooseHousehold"
        >
          Сохранить выбор
        </button>
      </div>
    </section>

    <section v-if="settings.householdId != null" class="card pad">
      <h3>🗓️ Инструменты</h3>
      <div class="tools">
        <div class="tool">
          <div class="stack">
            <strong>Подписка-календарь (iCal)</strong>
            <span class="muted small"
              >Добавьте в Google / Apple Calendar для напоминаний.</span
            >
          </div>
          <div class="row">
            <button class="btn btn-sm" @click="showCalendarUrl">Показать ссылку</button>
          </div>
        </div>

        <div v-if="calendarUrl" class="cal-url">
          <code>{{ calendarUrl }}</code>
          <div class="row">
            <button class="btn btn-sm" @click="copyCalendar">📋 Копировать</button>
            <button class="btn btn-sm" @click="openCalendar">↗ Открыть</button>
          </div>
        </div>

        <div class="tool">
          <div class="stack">
            <strong>CSV-отчёт</strong>
            <span class="muted small">Даты, когда дозы были приняты.</span>
          </div>
          <button class="btn btn-sm" :disabled="downloading" @click="downloadCsv">
            {{ downloading ? "Сохраняю…" : "💾 Скачать CSV" }}
          </button>
        </div>
      </div>
    </section>
  </div>
</template>

<style scoped>
.view {
  max-width: 760px;
  display: flex;
  flex-direction: column;
  gap: 1.3rem;
}
.view-head h1 {
  font-size: 1.7rem;
}
.view-head p {
  margin: 0.3rem 0 0;
}
.card.pad {
  padding: 1.3rem 1.4rem;
  display: flex;
  flex-direction: column;
  gap: 0.9rem;
}
.card.pad h3 {
  font-size: 1.05rem;
}
.small {
  font-size: 0.82rem;
}
.grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 1rem;
}
.token-row {
  display: flex;
  gap: 0.4rem;
  align-items: stretch;
}
.actions {
  display: flex;
  justify-content: flex-end;
}
.row.wrap {
  flex-wrap: wrap;
}
.household-select {
  flex: 1;
  min-width: 220px;
}
.tools {
  display: flex;
  flex-direction: column;
  gap: 0.9rem;
}
.tool {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  padding: 0.85rem 1rem;
  background: var(--surface-2);
  border-radius: var(--r-md);
}
.cal-url {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  flex-wrap: wrap;
  padding: 0.7rem 1rem;
  background: var(--corgi-wash);
  border-radius: var(--r-md);
}
.cal-url code {
  font-size: 0.82rem;
  color: var(--ink);
  word-break: break-all;
}
@media (max-width: 720px) {
  .grid {
    grid-template-columns: 1fr;
  }
}
</style>
