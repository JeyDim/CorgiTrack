<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useRouter } from "vue-router";

import type { Dog, Treatment } from "../api/types";
import { TREATMENT_KIND_LABEL } from "../api/types";
import { useSettingsStore } from "../stores/settings";
import { useToastStore } from "../stores/toast";
import { formatDate } from "../util/format";
import TreatmentForm from "../components/TreatmentForm.vue";
import ModalDialog from "../components/ModalDialog.vue";

const router = useRouter();
const settings = useSettingsStore();
const toast = useToastStore();

const loading = ref(false);
const dogs = ref<Dog[]>([]);
const treatments = ref<Treatment[]>([]);

const showForm = ref(false);
const editing = ref<Treatment | null>(null);
const deleteTarget = ref<Treatment | null>(null);
const busyId = ref<number | null>(null);

const hasHousehold = computed(() => settings.householdId != null);

interface DogGroup {
  dog: Dog;
  items: Treatment[];
}

const grouped = computed<DogGroup[]>(() => {
  const byDog = new Map<number, Treatment[]>();
  for (const t of treatments.value) {
    if (!byDog.has(t.dog_id)) byDog.set(t.dog_id, []);
    byDog.get(t.dog_id)!.push(t);
  }
  return dogs.value.map((dog) => ({ dog, items: byDog.get(dog.id) ?? [] }));
});

async function reload() {
  if (settings.householdId == null) return;
  loading.value = true;
  try {
    const api = settings.api();
    const [dogList, allTreatments] = await Promise.all([
      api.listDogs(settings.householdId),
      api.listTreatments(),
    ]);
    dogs.value = dogList;
    const dogIds = new Set(dogList.map((d) => d.id));
    treatments.value = allTreatments.filter((t) => dogIds.has(t.dog_id));
  } catch (e) {
    toast.error(`Не удалось загрузить: ${(e as Error).message}`);
  } finally {
    loading.value = false;
  }
}

function openCreate() {
  if (!dogs.value.length) {
    toast.error("Сначала добавьте собаку (через бота или API)");
    return;
  }
  editing.value = null;
  showForm.value = true;
}

function openEdit(t: Treatment) {
  editing.value = t;
  showForm.value = true;
}

function onSaved() {
  showForm.value = false;
  editing.value = null;
  reload();
}

async function toggleActive(t: Treatment) {
  busyId.value = t.id;
  try {
    const updated = await settings
      .api()
      .updateTreatment(t.id, { active: !t.active });
    t.active = updated.active;
  } catch (e) {
    toast.error(`Ошибка: ${(e as Error).message}`);
  } finally {
    busyId.value = null;
  }
}

async function confirmDelete() {
  if (!deleteTarget.value) return;
  const target = deleteTarget.value;
  busyId.value = target.id;
  try {
    await settings.api().deleteTreatment(target.id);
    toast.success("Назначение удалено");
    deleteTarget.value = null;
    await reload();
  } catch (e) {
    toast.error(`Ошибка удаления: ${(e as Error).message}`);
  } finally {
    busyId.value = null;
  }
}

onMounted(reload);
</script>

<template>
  <div class="view">
    <header class="view-head spread">
      <div>
        <h1>💊 Лечения</h1>
        <p class="muted">Таблетки и прививки — расписание приёмов.</p>
      </div>
      <button v-if="hasHousehold" class="btn btn-primary" @click="openCreate">
        ➕ Добавить
      </button>
    </header>

    <div v-if="!hasHousehold" class="empty card">
      <div class="empty-emoji">🐕‍🦺</div>
      <h3>Семья не выбрана</h3>
      <p class="muted">Выберите семью в настройках, чтобы управлять лечениями.</p>
      <button class="btn btn-primary" @click="router.push({ name: 'settings' })">
        Перейти в настройки
      </button>
    </div>

    <div v-else-if="loading" class="loading">
      <span class="paw-loader"><span>🐾</span><span>🐾</span><span>🐾</span></span>
    </div>

    <div v-else-if="!dogs.length" class="empty card">
      <div class="empty-emoji">🦴</div>
      <h3>Нет собак</h3>
      <p class="muted">
        Добавьте собаку через Telegram-бота или API — и здесь появятся назначения.
      </p>
    </div>

    <div v-else class="dogs">
      <section v-for="g in grouped" :key="g.dog.id" class="dog-block">
        <div class="dog-head">
          <span class="dog-emoji">🐶</span>
          <h2>{{ g.dog.name }}</h2>
          <span class="count muted">{{ g.items.length }}</span>
        </div>

        <div v-if="!g.items.length" class="card empty-row muted">
          Пока нет назначений
        </div>

        <div v-else class="t-list">
          <article
            v-for="t in g.items"
            :key="t.id"
            class="t-row card"
            :class="{ inactive: !t.active }"
          >
            <div class="t-kind" :title="TREATMENT_KIND_LABEL[t.kind]">
              {{ t.kind === "vaccine" ? "💉" : "💊" }}
            </div>

            <div class="t-main">
              <strong>{{ t.name }}</strong>
              <div class="t-meta muted small">
                <span v-if="t.dose_label">{{ t.dose_label }}</span>
                <span>· каждые {{ t.cycle_days }} дн.</span>
                <span>· с {{ formatDate(t.start_at) }}</span>
                <span>· ⏰ {{ t.reminder_time.slice(0, 5) }}</span>
              </div>
            </div>

            <button
              class="toggle"
              :class="{ on: t.active }"
              :disabled="busyId === t.id"
              :title="t.active ? 'Активно' : 'Выключено'"
              @click="toggleActive(t)"
            >
              <span class="knob" />
            </button>

            <div class="t-actions">
              <button class="btn btn-ghost btn-sm" @click="openEdit(t)">✏️</button>
              <button
                class="btn btn-ghost btn-sm"
                @click="deleteTarget = t"
              >
                🗑
              </button>
            </div>
          </article>
        </div>
      </section>
    </div>

    <!-- форма создания/редактирования -->
    <TreatmentForm
      v-if="showForm"
      :dogs="dogs"
      :treatment="editing"
      @close="showForm = false"
      @saved="onSaved"
    />

    <!-- подтверждение удаления -->
    <ModalDialog
      v-if="deleteTarget"
      title="Удалить назначение?"
      @close="deleteTarget = null"
    >
      <p>
        Назначение <strong>«{{ deleteTarget.name }}»</strong> и связанные с ним
        дозы будут удалены. Действие необратимо.
      </p>
      <template #footer>
        <button class="btn btn-ghost" @click="deleteTarget = null">Отмена</button>
        <button
          class="btn btn-danger-soft"
          :disabled="busyId === deleteTarget.id"
          @click="confirmDelete"
        >
          Удалить
        </button>
      </template>
    </ModalDialog>
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

.dogs {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
}
.dog-head {
  display: flex;
  align-items: center;
  gap: 0.6rem;
  margin-bottom: 0.7rem;
}
.dog-head h2 {
  font-size: 1.15rem;
}
.dog-emoji {
  font-size: 1.3rem;
}
.count {
  font-size: 0.85rem;
  background: var(--corgi-wash);
  padding: 0.05rem 0.55rem;
  border-radius: var(--r-pill);
}
.empty-row {
  padding: 0.9rem 1.1rem;
  font-size: 0.9rem;
}

.t-list {
  display: flex;
  flex-direction: column;
  gap: 0.6rem;
}
.t-row {
  display: flex;
  align-items: center;
  gap: 0.9rem;
  padding: 0.8rem 1rem;
  transition:
    transform 0.14s ease,
    box-shadow 0.14s ease;
}
.t-row:hover {
  transform: translateY(-1px);
  box-shadow: var(--shadow-md);
}
.t-row.inactive {
  opacity: 0.6;
}
.t-kind {
  font-size: 1.5rem;
  width: 2rem;
  text-align: center;
}
.t-main {
  flex: 1;
  min-width: 0;
}
.t-main strong {
  font-size: 1rem;
}
.t-meta {
  display: flex;
  gap: 0.35rem;
  flex-wrap: wrap;
  margin-top: 0.2rem;
}
.small {
  font-size: 0.82rem;
}
.t-actions {
  display: flex;
  gap: 0.2rem;
}

/* переключатель активности */
.toggle {
  width: 44px;
  height: 26px;
  border-radius: var(--r-pill);
  border: none;
  background: var(--paper-deep);
  position: relative;
  cursor: pointer;
  transition: background 0.18s ease;
  flex: none;
}
.toggle.on {
  background: linear-gradient(135deg, var(--corgi), var(--corgi-deep));
}
.toggle .knob {
  position: absolute;
  top: 3px;
  left: 3px;
  width: 20px;
  height: 20px;
  border-radius: 50%;
  background: #fff;
  box-shadow: var(--shadow-sm);
  transition: transform 0.18s cubic-bezier(0.2, 0.8, 0.3, 1.2);
}
.toggle.on .knob {
  transform: translateX(18px);
}
.toggle:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}
</style>
