<script setup lang="ts">
import { onMounted, ref } from "vue";
import { getVersion } from "@tauri-apps/api/app";
import { save } from "@tauri-apps/plugin-dialog";
import { writeFile } from "@tauri-apps/plugin-fs";
import { openUrl } from "@tauri-apps/plugin-opener";

import { CorgiApi } from "../api/client";
import type { AppSettings, FamilyMember, Household } from "../api/types";
import { useSettingsStore } from "../stores/settings";
import { useToastStore } from "../stores/toast";
import { useUpdaterStore } from "../stores/updater";

const settings = useSettingsStore();
const toast = useToastStore();
const updater = useUpdaterStore();

// Версия приложения из tauri.conf.json. В dev через браузер вызов недоступен —
// тогда оставляем прочерк.
const appVersion = ref("…");
const checkingUpdate = ref(false);

const baseUrl = ref(settings.baseUrl);
const token = ref(settings.token);
const showToken = ref(false);

const checking = ref(false);
const households = ref<Household[]>([]);
const selectedHousehold = ref<number | null>(settings.householdId);

const calendarUrl = ref<string | null>(null);
const downloading = ref(false);

// Глобальные операционные настройки (тайминги эскалации и шедулера).
const appSettings = ref<AppSettings | null>(null);
const savingSettings = ref(false);

// Порядок обзвона членов семьи.
const members = ref<FamilyMember[]>([]);
const savingMembers = ref(false);

onMounted(async () => {
  try {
    appVersion.value = await getVersion();
  } catch {
    appVersion.value = "—"; // запуск вне Tauri (dev в браузере)
  }

  // Если уже настроено — подтянем список семей и глобальные настройки.
  if (settings.configured) {
    try {
      households.value = await settings.api().listHouseholds();
    } catch {
      /* молча: пользователь увидит при «Проверить» */
    }
    await loadAppSettings();
    await loadMembers();
  }
});

async function loadAppSettings() {
  try {
    appSettings.value = await settings.api().getSettings();
  } catch {
    /* молча: появится при следующем действии */
  }
}

async function saveAppSettings() {
  const cur = appSettings.value;
  if (!cur) return;
  savingSettings.value = true;
  try {
    appSettings.value = await settings.api().updateSettings({
      escalation_first_delay_minutes: cur.escalation_first_delay_minutes,
      escalation_step_minutes: cur.escalation_step_minutes,
      reminder_lookahead_minutes: cur.reminder_lookahead_minutes,
      scheduler_tick_seconds: cur.scheduler_tick_seconds,
    });
    toast.success("Настройки эскалации сохранены");
  } catch (e) {
    toast.error(`Ошибка: ${(e as Error).message}`);
  } finally {
    savingSettings.value = false;
  }
}

async function loadMembers() {
  if (settings.householdId == null) {
    members.value = [];
    return;
  }
  try {
    const list = await settings.api().listMembers(settings.householdId);
    members.value = list.sort(
      (a, b) => a.escalation_order - b.escalation_order || a.id - b.id,
    );
  } catch {
    members.value = [];
  }
}

async function saveMembers() {
  savingMembers.value = true;
  try {
    for (const m of members.value) {
      await settings.api().updateMember(m.id, {
        escalation_order: m.escalation_order,
        notify: m.notify,
      });
    }
    toast.success("Порядок обзвона сохранён");
    await loadMembers();
  } catch (e) {
    toast.error(`Ошибка: ${(e as Error).message}`);
  } finally {
    savingMembers.value = false;
  }
}

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
    await loadAppSettings();
    await loadMembers();
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
  await loadMembers();
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

async function checkUpdates() {
  checkingUpdate.value = true;
  try {
    const res = await updater.checkNow();
    if (res === "available") {
      // Баннер обновления покажется сам — он реагирует на updater.available.
      toast.success(`Доступна версия ${updater.version}`);
    } else {
      toast.success("У вас последняя версия 🎉");
    }
  } catch (e) {
    toast.error(`Не удалось проверить обновления: ${(e as Error).message}`);
  } finally {
    checkingUpdate.value = false;
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

    <section
      v-if="settings.householdId != null && members.length"
      class="card pad"
    >
      <h3>📣 Порядок напоминаний</h3>
      <p class="muted small">
        При напоминании сначала пишем участнику с наименьшим номером. Если за
        отведённое время никто не нажал «Принято» — повтор тому же человеку,
        затем следующий по порядку. 0 уведомляется первым. Учитываются только
        участники с галочкой и привязанным Telegram.
      </p>
      <div class="members">
        <div v-for="m in members" :key="m.id" class="member-row">
          <div class="stack member-name">
            <strong>{{ m.display_name }}</strong>
            <span class="muted small">
              {{
                m.telegram_user_id
                  ? `Telegram ID: ${m.telegram_user_id}`
                  : "Telegram не привязан"
              }}
            </span>
          </div>
          <label class="order-field">
            <span class="muted small">Порядок</span>
            <input
              v-model.number="m.escalation_order"
              type="number"
              min="0"
              class="input order-input"
            />
          </label>
          <label class="notify-field">
            <input v-model="m.notify" type="checkbox" />
            <span class="muted small">Уведомлять</span>
          </label>
        </div>
      </div>
      <div class="actions">
        <button
          class="btn btn-primary"
          :disabled="savingMembers"
          @click="saveMembers"
        >
          {{ savingMembers ? "Сохраняю…" : "Сохранить порядок" }}
        </button>
      </div>
    </section>

    <section v-if="appSettings" class="card pad">
      <h3>⏱️ Эскалация и напоминания</h3>
      <p class="muted small">
        Глобальные тайминги бота. Хранятся на сервере и применяются без
        перезапуска.
      </p>
      <div class="grid grid-4">
        <div class="field">
          <label>Повтор первому, мин</label>
          <input
            v-model.number="appSettings.escalation_first_delay_minutes"
            type="number"
            min="1"
            class="input"
          />
          <span class="muted small"
            >пауза до повторного вопроса тому же человеку</span
          >
        </div>
        <div class="field">
          <label>Шаг эскалации, мин</label>
          <input
            v-model.number="appSettings.escalation_step_minutes"
            type="number"
            min="1"
            class="input"
          />
          <span class="muted small">пауза между следующими шагами</span>
        </div>
        <div class="field">
          <label>Окно напоминания, мин</label>
          <input
            v-model.number="appSettings.reminder_lookahead_minutes"
            type="number"
            min="0"
            class="input"
          />
          <span class="muted small">за сколько до приёма слать первое</span>
        </div>
        <div class="field">
          <label>Период шедулера, сек</label>
          <input
            v-model.number="appSettings.scheduler_tick_seconds"
            type="number"
            min="1"
            class="input"
          />
          <span class="muted small">как часто проверять напоминания</span>
        </div>
      </div>
      <div class="actions">
        <button
          class="btn btn-primary"
          :disabled="savingSettings"
          @click="saveAppSettings"
        >
          {{ savingSettings ? "Сохраняю…" : "Сохранить настройки" }}
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

    <section class="card pad">
      <h3>ℹ️ О приложении</h3>
      <div class="tool">
        <div class="stack">
          <strong>CorgiTrack</strong>
          <span class="muted small">Версия {{ appVersion }}</span>
        </div>
        <button class="btn btn-sm" :disabled="checkingUpdate" @click="checkUpdates">
          {{ checkingUpdate ? "Проверяю…" : "🔄 Проверить обновления" }}
        </button>
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
.grid-4 {
  grid-template-columns: repeat(4, 1fr);
}
.field .muted.small {
  margin-top: 0.15rem;
}
.members {
  display: flex;
  flex-direction: column;
  gap: 0.6rem;
}
.member-row {
  display: flex;
  align-items: center;
  gap: 1rem;
  padding: 0.7rem 1rem;
  background: var(--surface-2);
  border-radius: var(--r-md);
}
.member-name {
  flex: 1;
  min-width: 0;
}
.order-field {
  display: flex;
  flex-direction: column;
  gap: 0.2rem;
}
.order-input {
  width: 84px;
}
.notify-field {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  white-space: nowrap;
  cursor: pointer;
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
  .grid-4 {
    grid-template-columns: 1fr 1fr;
  }
  .member-row {
    flex-wrap: wrap;
  }
}
</style>
