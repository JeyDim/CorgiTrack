<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { useRouter } from "vue-router";

import type { Dog, DoseView, Treatment } from "../api/types";
import { useSettingsStore } from "../stores/settings";
import { useToastStore } from "../stores/toast";

const router = useRouter();
const settings = useSettingsStore();
const toast = useToastStore();

const loading = ref(false);
const dogs = ref<Dog[]>([]);
const treatments = ref<Treatment[]>([]);
const taken = ref<DoseView[]>([]);
const selectedDogId = ref<number | null>(null);

const hasHousehold = computed(() => settings.householdId != null);

/** Одна запись «Веткниги» — пройденная доза. */
interface Entry {
  id: number;
  date: string; // ISO: фактический приём (taken_at) либо плановая дата
  name: string;
  dose_label: string | null;
  clinic: string | null;
}

async function reload() {
  if (settings.householdId == null) return;
  loading.value = true;
  try {
    const api = settings.api();
    const [dogList, allTreatments, takenDoses] = await Promise.all([
      api.listDogs(settings.householdId),
      api.listTreatments(),
      api.listDoses({ household_id: settings.householdId, status: "taken" }),
    ]);
    dogs.value = dogList;
    treatments.value = allTreatments;
    taken.value = takenDoses;
  } catch (e) {
    toast.error(`Не удалось загрузить: ${(e as Error).message}`);
  } finally {
    loading.value = false;
  }
}

// Лечение по id — нужно, чтобы узнать вид (kind) дозы и собаку.
const treatmentById = computed(() => {
  const m = new Map<number, Treatment>();
  for (const t of treatments.value) m.set(t.id, t);
  return m;
});

const dogIds = computed(() => new Set(dogs.value.map((d) => d.id)));

function entryDate(d: DoseView): string {
  return d.taken_at ?? d.due_at;
}

// Прививки в паспорте идут от старых к новым (как страницы ветпаспорта).
function byDateAsc(a: Entry, b: Entry): number {
  return new Date(a.date).getTime() - new Date(b.date).getTime();
}

// Разносим пройденные дозы по трём разделам, учитывая выбранную собаку.
const classified = computed(() => {
  const vaccine: Entry[] = [];
  const worm: Entry[] = [];
  const tick: Entry[] = [];

  for (const d of taken.value) {
    const t = treatmentById.value.get(d.treatment_id);
    if (!t) continue;
    if (!dogIds.value.has(t.dog_id)) continue;
    if (selectedDogId.value != null && t.dog_id !== selectedDogId.value) continue;

    const entry: Entry = {
      id: d.id,
      date: entryDate(d),
      name: d.treatment_name,
      dose_label: d.dose_label,
      clinic: d.clinic,
    };

    // Прививки — отдельно; таблетки делим по явной категории.
    // Категория не задана (старые записи) → «от гельминтов».
    if (t.kind === "vaccine") vaccine.push(entry);
    else if (t.category === "tick") tick.push(entry);
    else worm.push(entry);
  }

  vaccine.sort(byDateAsc);
  worm.sort(byDateAsc);
  tick.sort(byDateAsc);
  return { vaccine, worm, tick };
});

// ---- пагинация: листаем разворот, как страницы паспорта ----
const PER_PAGE_VAX = 4; // прививок на левой странице
const PER_PAGE_WORM = 4; // строк «от гельминтов» на правой
const PER_PAGE_TICK = 4; // строк «от клещей» на правой
const page = ref(0); // текущая страница разворота, 0-based

// Сколько разворотов нужно — по самому «длинному» разделу.
const totalPages = computed(() =>
  Math.max(
    1,
    Math.ceil(classified.value.vaccine.length / PER_PAGE_VAX),
    Math.ceil(classified.value.worm.length / PER_PAGE_WORM),
    Math.ceil(classified.value.tick.length / PER_PAGE_TICK),
  ),
);

function pageSlice<T>(arr: T[], per: number): T[] {
  const start = page.value * per;
  return arr.slice(start, start + per);
}

// Прививки текущей страницы группируем по году — как развороты-«годы» в паспорте.
const vaccineGroups = computed(() => {
  const groups = new Map<number, Entry[]>();
  for (const e of pageSlice(classified.value.vaccine, PER_PAGE_VAX)) {
    const y = new Date(e.date).getFullYear();
    if (!groups.has(y)) groups.set(y, []);
    groups.get(y)!.push(e);
  }
  return [...groups.entries()]
    .sort((a, b) => a[0] - b[0])
    .map(([year, items]) => ({ year, items }));
});

const worms = computed(() => pageSlice(classified.value.worm, PER_PAGE_WORM));
const ticks = computed(() => pageSlice(classified.value.tick, PER_PAGE_TICK));

// Глобальные флаги — отличаем «совсем нет записей» от «нет на этой странице».
const hasVaccines = computed(() => classified.value.vaccine.length > 0);
const hasWorms = computed(() => classified.value.worm.length > 0);
const hasTicks = computed(() => classified.value.tick.length > 0);

function goPrev() {
  if (page.value > 0) page.value--;
}
function goNext() {
  if (page.value < totalPages.value - 1) page.value++;
}

// ---- форматирование ----
const dayFmt = new Intl.DateTimeFormat("ru-RU", { day: "2-digit" });
const monthYearFmt = new Intl.DateTimeFormat("ru-RU", {
  month: "long",
  year: "numeric",
});
const stampDateFmt = new Intl.DateTimeFormat("ru-RU", {
  day: "2-digit",
  month: "2-digit",
  year: "numeric",
});
const shortFmt = new Intl.DateTimeFormat("ru-RU", {
  day: "2-digit",
  month: "short",
  year: "numeric",
});

function dayNum(iso: string): string {
  return dayFmt.format(new Date(iso));
}
function monthYear(iso: string): string {
  return monthYearFmt.format(new Date(iso));
}
function stampDate(iso: string): string {
  return stampDateFmt.format(new Date(iso));
}
function rowDate(iso: string): string {
  return shortFmt.format(new Date(iso));
}

onMounted(reload);
// Сменили семью в настройках — перечитываем и сбрасываем выбор собаки.
watch(
  () => settings.householdId,
  () => {
    selectedDogId.value = null;
    reload();
  },
);
// Сменили собаку — открываем разворот с первой страницы.
watch(selectedDogId, () => {
  page.value = 0;
});
// Записей стало меньше (фильтр/перезагрузка) — не выходим за последнюю страницу.
watch(totalPages, (max) => {
  if (page.value > max - 1) page.value = Math.max(0, max - 1);
});
</script>

<template>
  <div class="view">
    <header class="view-head spread">
      <div>
        <h1>📖 Веткнига</h1>
        <p class="muted">История прививок и обработок — как в ветпаспорте.</p>
      </div>
      <select
        v-if="dogs.length > 1"
        v-model.number="selectedDogId"
        class="select dog-select"
      >
        <option :value="null">Все собаки</option>
        <option v-for="d in dogs" :key="d.id" :value="d.id">{{ d.name }}</option>
      </select>
    </header>

    <div v-if="!hasHousehold" class="empty card">
      <div class="empty-emoji">🐕‍🦺</div>
      <h3>Семья не выбрана</h3>
      <p class="muted">Выберите семью в настройках, чтобы открыть Веткнигу.</p>
      <button class="btn btn-primary" @click="router.push({ name: 'settings' })">
        Перейти в настройки
      </button>
    </div>

    <div v-else-if="loading" class="loading">
      <span class="paw-loader"><span>🐾</span><span>🐾</span><span>🐾</span></span>
    </div>

    <div v-else class="book">
      <!-- ЛЕВАЯ СТРАНИЦА — прививки со штампами клиник -->
      <section class="page page-left">
        <div class="page-head">
          <span class="page-ico">💉</span>
          <h2>Прививки</h2>
        </div>

        <div v-if="!hasVaccines" class="page-empty">
          <div class="stamp-empty">место<br />для<br />печати</div>
          <p class="muted">
            Здесь появятся прививки со штампом клиники — отметьте дозу принятой.
          </p>
        </div>

        <div v-else :key="page" class="vax-groups flip-in">
          <div v-for="g in vaccineGroups" :key="g.year" class="vax-year">
            <div class="year-tab">{{ g.year }}</div>

            <article v-for="e in g.items" :key="e.id" class="vax-entry">
              <div class="vax-date">
                <span class="vax-day">{{ dayNum(e.date) }}</span>
                <span class="vax-my">{{ monthYear(e.date) }}</span>
              </div>

              <div class="vax-main">
                <strong>{{ e.name }}</strong>
                <span v-if="e.dose_label" class="muted small">{{ e.dose_label }}</span>
              </div>

              <!-- штамп клиники -->
              <div v-if="e.clinic" class="stamp" :title="e.clinic ?? ''">
                <span class="stamp-name">{{ e.clinic }}</span>
                <span class="stamp-date">{{ stampDate(e.date) }}</span>
              </div>
              <div v-else class="stamp stamp-blank" title="Клиника не указана">
                <span>нет<br />печати</span>
              </div>
            </article>
          </div>
        </div>
      </section>

      <!-- ПРАВАЯ СТРАНИЦА — обработки от паразитов -->
      <section class="page page-right">
        <div :key="page" class="treat-stack flip-in">
          <div class="page-head">
            <span class="page-ico">🪱</span>
            <h2>От гельминтов</h2>
          </div>
          <div class="ruled">
            <p v-if="!hasWorms" class="row-empty muted">Пока нет записей</p>
            <div v-for="e in worms" :key="e.id" class="ruled-row">
              <span class="r-date">{{ rowDate(e.date) }}</span>
              <span class="r-name">{{ e.name }}</span>
              <span v-if="e.dose_label" class="r-dose muted small">{{ e.dose_label }}</span>
            </div>
          </div>

          <div class="page-head page-head-2">
            <span class="page-ico">🕷</span>
            <h2>От клещей</h2>
          </div>
          <div class="ruled">
            <p v-if="!hasTicks" class="row-empty muted">Пока нет записей</p>
            <div v-for="e in ticks" :key="e.id" class="ruled-row">
              <span class="r-date">{{ rowDate(e.date) }}</span>
              <span class="r-name">{{ e.name }}</span>
              <span v-if="e.dose_label" class="r-dose muted small">{{ e.dose_label }}</span>
            </div>
          </div>
        </div>
      </section>

      <!-- листание разворота: одна навигация на обе страницы -->
      <footer v-if="totalPages > 1" class="book-nav">
        <button
          class="page-btn"
          :disabled="page === 0"
          aria-label="Предыдущая страница"
          title="Предыдущая страница"
          @click="goPrev"
        >
          ←
        </button>
        <span class="page-count">стр. {{ page + 1 }} из {{ totalPages }}</span>
        <button
          class="page-btn"
          :disabled="page >= totalPages - 1"
          aria-label="Следующая страница"
          title="Следующая страница"
          @click="goNext"
        >
          →
        </button>
      </footer>
    </div>
  </div>
</template>

<style scoped>
.view {
  display: flex;
  flex-direction: column;
  gap: 1.4rem;
  /* токены «бумаги книги» и фиолетовых чернил штампа */
  --vb-page: #fcf5e8;
  --vb-page-edge: #f0dcc0;
  --vb-rule: rgba(201, 116, 43, 0.16);
  --vb-stamp: #5b4b9b;
}
.view-head h1 {
  font-size: 1.8rem;
}
.view-head p {
  margin: 0.3rem 0 0;
}
.dog-select {
  width: auto;
  min-width: 12rem;
}

.loading {
  display: flex;
  justify-content: center;
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

/* ===================== книга ===================== */
.book {
  position: relative;
  display: grid;
  grid-template-columns: 1fr 1fr;
  max-width: 1040px;
  width: 100%;
  margin: 0 auto;
  background: var(--vb-page);
  border-radius: var(--r-lg);
  box-shadow:
    var(--shadow-lg),
    0 0 0 1px rgba(201, 116, 43, 0.1);
  overflow: hidden;
  animation: book-open 0.5s cubic-bezier(0.2, 0.8, 0.3, 1) both;
}
/* «толщина» книги — лёгкие подложки из-под краёв */
.book::before,
.book::after {
  content: "";
  position: absolute;
  top: 8px;
  bottom: 8px;
  width: 10px;
  border-radius: var(--r-lg);
  background: var(--vb-page-edge);
  z-index: -1;
}
.book::before {
  left: -5px;
}
.book::after {
  right: -5px;
}

.page {
  padding: 1.6rem 1.7rem 2rem;
  min-height: 60vh;
  position: relative;
}
/* корешок: тень по внутренним краям + шов по центру */
.page-left {
  box-shadow: inset -18px 0 24px -18px rgba(120, 80, 40, 0.35);
  border-right: 1px solid rgba(120, 80, 40, 0.18);
}
.page-right {
  box-shadow: inset 18px 0 24px -18px rgba(120, 80, 40, 0.35);
}

/* ---- листание разворота ---- */
.book-nav {
  grid-column: 1 / -1;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 1.1rem;
  padding: 0.85rem 1rem 1.05rem;
  border-top: 1px solid rgba(120, 80, 40, 0.18);
  background: linear-gradient(
    to bottom,
    rgba(120, 80, 40, 0.05),
    transparent
  );
}
.page-btn {
  width: 2rem;
  height: 2rem;
  flex: none;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border-radius: 50%;
  border: 1px solid rgba(120, 80, 40, 0.3);
  background: var(--vb-page);
  color: var(--corgi-deep);
  font-size: 1rem;
  line-height: 1;
  cursor: pointer;
  transition:
    transform 0.15s ease,
    background 0.15s ease,
    opacity 0.15s ease;
}
.page-btn:hover:not(:disabled) {
  background: var(--corgi-wash);
  transform: translateY(-1px);
}
.page-btn:disabled {
  opacity: 0.35;
  cursor: default;
}
.page-count {
  font-family: var(--font-display);
  font-size: 0.85rem;
  font-weight: 600;
  color: var(--corgi-deep);
  letter-spacing: 0.02em;
  min-width: 7rem;
  text-align: center;
}

/* мягкое «перелистывание» содержимого страниц */
.flip-in {
  animation: flip-in 0.34s cubic-bezier(0.2, 0.8, 0.3, 1) both;
}
@keyframes flip-in {
  from {
    opacity: 0;
    transform: translateY(6px);
  }
  to {
    opacity: 1;
    transform: none;
  }
}

.page-head {
  display: flex;
  align-items: center;
  gap: 0.55rem;
  padding-bottom: 0.5rem;
  margin-bottom: 1rem;
  border-bottom: 2px solid var(--corgi-wash);
}
.page-head-2 {
  margin-top: 1.8rem;
}
.page-ico {
  font-size: 1.3rem;
}
.page-head h2 {
  font-size: 1.25rem;
  color: var(--ink);
}

/* ---- пустая страница прививок ---- */
.page-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 1rem;
  padding: 2.4rem 1rem;
  text-align: center;
}
.stamp-empty {
  width: 110px;
  height: 110px;
  border-radius: 50%;
  border: 2px dashed var(--ink-faint);
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--ink-faint);
  font-family: var(--font-display);
  text-transform: uppercase;
  font-size: 0.7rem;
  letter-spacing: 0.12em;
  line-height: 1.5;
  transform: rotate(-7deg);
}

/* ===================== прививки ===================== */
.vax-groups {
  display: flex;
  flex-direction: column;
  gap: 1.4rem;
}
.year-tab {
  display: inline-block;
  font-family: var(--font-display);
  font-weight: 600;
  font-size: 0.78rem;
  color: var(--corgi-deep);
  background: var(--corgi-wash);
  padding: 0.1rem 0.7rem;
  border-radius: var(--r-pill);
  margin-bottom: 0.7rem;
}
.vax-year {
  display: flex;
  flex-direction: column;
}

.vax-entry {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr) auto;
  align-items: center;
  gap: 1.2rem;
  /* минимальная высота строки: штамп повёрнут и визуально выше своей строки —
     запас не даёт ему «лечь» на текст соседней прививки */
  min-height: 5rem;
  padding: 0.8rem 0.2rem 1rem;
  border-bottom: 1px dashed var(--vb-rule);
  transition:
    transform 0.18s ease,
    background 0.18s ease;
}
.vax-entry:hover {
  transform: translateY(-1px);
  background: rgba(255, 255, 255, 0.4);
}
.vax-date {
  display: flex;
  flex-direction: column;
  align-items: center;
  width: 3.4rem;
  line-height: 1;
}
.vax-day {
  font-family: var(--font-display);
  font-size: 1.7rem;
  font-weight: 700;
  color: var(--ink);
}
.vax-my {
  font-size: 0.66rem;
  color: var(--ink-soft);
  text-align: center;
  margin-top: 0.15rem;
}
.vax-main {
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 0.15rem;
  /* длинное название прививки переносим, а не пускаем «под печать» */
  overflow-wrap: anywhere;
}
.vax-main strong {
  font-size: 1rem;
  color: var(--ink);
  overflow-wrap: anywhere;
}
.small {
  font-size: 0.82rem;
}

/* ---- штамп клиники (сигнатура раздела) ---- */
.stamp {
  position: relative;
  width: 9.6rem;
  min-height: 3.6rem;
  flex: none;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  text-align: center;
  gap: 0.22rem;
  padding: 0.45rem 0.7rem 0.5rem;
  /* горизонтальный отступ компенсирует вынос от поворота — печать
     не заходит на текст слева и на край страницы справа */
  margin: 0.25rem 0.25rem;
  border: 2.5px solid var(--vb-stamp);
  border-radius: 9px;
  color: var(--vb-stamp);
  background: rgba(91, 75, 155, 0.05);
  opacity: 0.9;
  transform: rotate(-8deg);
  transition: transform 0.28s cubic-bezier(0.2, 0.8, 0.3, 1.4);
}
/* вторая рамка — как у настоящей резиновой печати */
.stamp::before {
  content: "";
  position: absolute;
  inset: 3px;
  border: 1px solid var(--vb-stamp);
  border-radius: 6px;
  opacity: 0.5;
  pointer-events: none;
}
.vax-entry:hover .stamp {
  transform: rotate(0deg) scale(1.03);
}
.stamp-head {
  font-family: var(--font-display);
  font-size: 0.56rem;
  font-weight: 600;
  letter-spacing: 0.14em;
  text-transform: uppercase;
  opacity: 0.85;
}
.stamp-name {
  font-family: var(--font-display);
  font-size: 0.82rem;
  font-weight: 700;
  line-height: 1.15;
  text-transform: uppercase;
  letter-spacing: 0.01em;
  /* полное название клиники, перенос по словам */
  overflow-wrap: anywhere;
}
.stamp-date {
  font-family: var(--font-display);
  font-size: 0.7rem;
  font-weight: 600;
  letter-spacing: 0.04em;
  width: 72%;
  padding-top: 0.2rem;
  border-top: 1px solid rgba(91, 75, 155, 0.35);
}
.stamp-blank {
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 3.6rem;
  border: 2px dashed var(--ink-faint);
  background: none;
  opacity: 1;
  color: var(--ink-faint);
  font-family: var(--font-display);
  text-transform: uppercase;
  font-size: 0.6rem;
  line-height: 1.4;
  text-align: center;
  letter-spacing: 0.08em;
}
.stamp-blank::before {
  display: none;
}

/* ===================== обработки (правая страница) ===================== */
.ruled {
  display: flex;
  flex-direction: column;
}
.ruled-row {
  display: grid;
  grid-template-columns: 6.6rem 1fr auto;
  align-items: baseline;
  gap: 0.6rem;
  padding: 0.55rem 0.2rem;
  border-bottom: 1px solid var(--vb-rule);
}
.r-date {
  font-family: var(--font-display);
  font-size: 0.86rem;
  color: var(--corgi-deep);
  font-weight: 500;
}
.r-name {
  color: var(--ink);
  min-width: 0;
}
.r-dose {
  white-space: nowrap;
}
.row-empty {
  padding: 0.7rem 0.2rem;
  font-size: 0.9rem;
}

@keyframes book-open {
  from {
    opacity: 0;
    transform: perspective(1400px) rotateX(6deg) scale(0.97);
  }
  to {
    opacity: 1;
    transform: perspective(1400px) rotateX(0) scale(1);
  }
}

/* книга «закрывается» в один столбец на узких экранах */
@media (max-width: 880px) {
  .book {
    grid-template-columns: 1fr;
  }
  .page-left {
    box-shadow: inset 0 -18px 24px -18px rgba(120, 80, 40, 0.35);
    border-right: none;
    border-bottom: 1px solid rgba(120, 80, 40, 0.18);
  }
  .page-right {
    box-shadow: inset 0 18px 24px -18px rgba(120, 80, 40, 0.35);
  }
}

@media (prefers-reduced-motion: reduce) {
  .book,
  .flip-in {
    animation: none;
  }
  .stamp,
  .vax-entry,
  .page-btn {
    transition: none;
  }
}
</style>
