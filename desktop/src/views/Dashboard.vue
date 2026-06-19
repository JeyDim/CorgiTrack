<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { useRouter } from "vue-router";

import type { DoseStatus, DoseView, FamilyMember } from "../api/types";
import { useSettingsStore } from "../stores/settings";
import { useToastStore } from "../stores/toast";
import { dayKey, formatTime, humanDay } from "../util/format";
import StatusBadge from "../components/StatusBadge.vue";

const router = useRouter();
const settings = useSettingsStore();
const toast = useToastStore();

const LOOKAHEADS = [
  { label: "Сутки", hours: 24 },
  { label: "Неделя", hours: 168 },
  { label: "Месяц", hours: 720 },
];

const lookahead = ref(24);
const loading = ref(false);
const doses = ref<DoseView[]>([]);
const members = ref<FamilyMember[]>([]);
const actingMember = ref<number | null>(null);
const busyId = ref<number | null>(null);

const hasHousehold = computed(() => settings.householdId != null);

interface DayGroup {
  key: string;
  label: string;
  items: DoseView[];
}

const groups = computed<DayGroup[]>(() => {
  const map = new Map<string, DayGroup>();
  for (const d of doses.value) {
    const key = dayKey(d.due_at);
    if (!map.has(key)) {
      map.set(key, { key, label: humanDay(d.due_at), items: [] });
    }
    map.get(key)!.items.push(d);
  }
  return Array.from(map.values());
});

const pendingCount = computed(
  () => doses.value.filter((d) => d.status !== "taken").length,
);

async function reload() {
  if (settings.householdId == null) return;
  loading.value = true;
  try {
    doses.value = await settings
      .api()
      .getDue(settings.householdId, lookahead.value);
  } catch (e) {
    toast.error(`Не удалось загрузить дозы: ${(e as Error).message}`);
  } finally {
    loading.value = false;
  }
}

async function loadMembers() {
  if (settings.householdId == null) return;
  try {
    members.value = await settings.api().listMembers(settings.householdId);
  } catch {
    /* участники не обязательны */
  }
}

async function mark(dose: DoseView, status: DoseStatus) {
  busyId.value = dose.id;
  try {
    const updated = await settings.api().setDoseStatus(dose.id, {
      status,
      member_id: actingMember.value ?? undefined,
    });
    dose.status = updated.status;
    dose.taken_at = updated.taken_at;
    toast.success(status === "taken" ? "Принято 🐾" : "Отмечено пропущенным");
  } catch (e) {
    toast.error(`Ошибка: ${(e as Error).message}`);
  } finally {
    busyId.value = null;
  }
}

onMounted(() => {
  reload();
  loadMembers();
});
watch(lookahead, reload);
</script>

<template>
  <div class="view">
    <header class="view-head spread">
      <div>
        <h1>🦴 Уведомления</h1>
        <p class="muted">
          <template v-if="hasHousehold">
            {{ pendingCount }}
            {{ pendingCount === 1 ? "доза ждёт" : "доз ждут" }} отметки
          </template>
          <template v-else>Сначала выберите семью в настройках</template>
        </p>
      </div>

      <div v-if="hasHousehold" class="controls">
        <select
          v-if="members.length"
          v-model.number="actingMember"
          class="select member-select"
          title="Кто отмечает"
        >
          <option :value="null">Отмечает: —</option>
          <option v-for="m in members" :key="m.id" :value="m.id">
            {{ m.display_name }}
          </option>
        </select>

        <div class="segmented">
          <button
            v-for="opt in LOOKAHEADS"
            :key="opt.hours"
            class="seg"
            :class="{ on: lookahead === opt.hours }"
            @click="lookahead = opt.hours"
          >
            {{ opt.label }}
          </button>
        </div>

        <button class="btn btn-ghost btn-sm" :disabled="loading" @click="reload">
          ⟳
        </button>
      </div>
    </header>

    <!-- нет семьи -->
    <div v-if="!hasHousehold" class="empty card">
      <div class="empty-emoji">🐕‍🦺</div>
      <h3>Семья не выбрана</h3>
      <p class="muted">Откройте настройки и выберите семью, чтобы видеть дозы.</p>
      <button class="btn btn-primary" @click="router.push({ name: 'settings' })">
        Перейти в настройки
      </button>
    </div>

    <!-- загрузка -->
    <div v-else-if="loading" class="loading">
      <span class="paw-loader"><span>🐾</span><span>🐾</span><span>🐾</span></span>
      <p class="muted">Нюхаем расписание…</p>
    </div>

    <!-- пусто -->
    <div v-else-if="!doses.length" class="empty card">
      <div class="empty-emoji">🎉</div>
      <h3>Всё под контролем</h3>
      <p class="muted">На выбранный период ближайших доз нет.</p>
    </div>

    <!-- список по дням -->
    <div v-else class="days">
      <section v-for="g in groups" :key="g.key" class="day">
        <div class="day-head">
          <span class="paw">🐾</span>
          <h2>{{ g.label }}</h2>
          <span class="count muted">{{ g.items.length }}</span>
        </div>

        <div class="dose-list">
          <article
            v-for="d in g.items"
            :key="d.id"
            class="dose card"
            :class="{ done: d.status === 'taken' }"
          >
            <div class="time">{{ formatTime(d.due_at) }}</div>

            <div class="dose-main">
              <div class="dose-title">
                <strong>{{ d.treatment_name }}</strong>
                <span class="dog">· {{ d.dog_name }}</span>
              </div>
              <div class="dose-meta">
                <span v-if="d.dose_label" class="chip">{{ d.dose_label }}</span>
                <span v-if="d.instructions" class="muted small">{{
                  d.instructions
                }}</span>
              </div>
            </div>

            <div class="dose-side">
              <StatusBadge :status="d.status" />
              <div v-if="d.status !== 'taken'" class="dose-actions">
                <button
                  class="btn btn-ok btn-sm"
                  :disabled="busyId === d.id"
                  @click="mark(d, 'taken')"
                >
                  Принято
                </button>
                <button
                  class="btn btn-danger-soft btn-sm"
                  :disabled="busyId === d.id"
                  @click="mark(d, 'skipped')"
                >
                  Пропустить
                </button>
              </div>
              <span v-else-if="d.taken_at" class="muted small">
                в {{ formatTime(d.taken_at) }}
              </span>
            </div>
          </article>
        </div>
      </section>
    </div>
  </div>
</template>

<style scoped>
.view {
  display: flex;
  flex-direction: column;
  gap: 1.4rem;
}
.view-head h1 {
  font-size: 1.8rem;
}
.view-head p {
  margin: 0.3rem 0 0;
}
.controls {
  display: flex;
  align-items: center;
  gap: 0.6rem;
}
.member-select {
  width: auto;
  padding: 0.4rem 0.7rem;
  font-size: 0.85rem;
}

.segmented {
  display: inline-flex;
  background: var(--surface);
  border-radius: var(--r-pill);
  padding: 3px;
  box-shadow: var(--shadow-sm);
}
.seg {
  border: none;
  background: transparent;
  font-family: var(--font-display);
  font-weight: 500;
  font-size: 0.85rem;
  color: var(--ink-soft);
  padding: 0.35rem 0.85rem;
  border-radius: var(--r-pill);
  cursor: pointer;
  transition: all 0.14s ease;
}
.seg.on {
  background: linear-gradient(135deg, var(--corgi), var(--corgi-deep));
  color: #fff;
  box-shadow: var(--shadow-sm);
}

.loading {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.6rem;
  padding: 3rem;
}

.empty {
  text-align: center;
  padding: 2.6rem 2rem;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.6rem;
}
.empty-emoji {
  font-size: 3rem;
}

.days {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
}
.day-head {
  display: flex;
  align-items: center;
  gap: 0.6rem;
  margin-bottom: 0.7rem;
}
.day-head h2 {
  font-size: 1.05rem;
}
.day-head .paw {
  opacity: 0.5;
}
.day-head .count {
  font-size: 0.85rem;
  background: var(--corgi-wash);
  padding: 0.05rem 0.55rem;
  border-radius: var(--r-pill);
}

.dose-list {
  display: flex;
  flex-direction: column;
  gap: 0.7rem;
}
.dose {
  display: flex;
  align-items: center;
  gap: 1rem;
  padding: 0.9rem 1.1rem;
  transition:
    transform 0.14s ease,
    box-shadow 0.14s ease;
}
.dose:hover {
  transform: translateY(-1px);
  box-shadow: var(--shadow-md);
}
.dose.done {
  opacity: 0.72;
}
.time {
  font-family: var(--font-display);
  font-weight: 600;
  font-size: 1.15rem;
  color: var(--corgi-deep);
  min-width: 56px;
}
.dose-main {
  flex: 1;
  min-width: 0;
}
.dose-title strong {
  font-size: 1rem;
}
.dose-title .dog {
  color: var(--ink-soft);
  font-size: 0.9rem;
}
.dose-meta {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  margin-top: 0.25rem;
  flex-wrap: wrap;
}
.chip {
  font-size: 0.78rem;
  font-weight: 700;
  color: var(--corgi-deep);
  background: var(--corgi-wash);
  padding: 0.1rem 0.55rem;
  border-radius: var(--r-pill);
}
.dose-side {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 0.5rem;
}
.dose-actions {
  display: flex;
  gap: 0.4rem;
}
.small {
  font-size: 0.82rem;
}
</style>
