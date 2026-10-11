<script setup lang="ts">
import { computed, onMounted, reactive, ref, watch } from "vue";
import { CalendarPlus } from "@lucide/vue";

import type { Dog, DoseView, FamilyMember, Treatment } from "../api/types";
import { useSettingsStore } from "../stores/settings";
import { useToastStore } from "../stores/toast";
import { formatDateTime, toDatetimeLocal } from "../util/format";
import ModalDialog from "./ModalDialog.vue";
import StatusBadge from "./StatusBadge.vue";

const props = defineProps<{
  /** Предвыбранное назначение (кнопка в списке лечений). */
  treatmentId?: number | null;
  /** Кто дал — по умолчанию (выбор «Отмечает» на главной). */
  memberId?: number | null;
}>();
const emit = defineEmits<{ (e: "close"): void; (e: "saved"): void }>();

const settings = useSettingsStore();
const toast = useToastStore();

/** Сколько последних неотмеченных плановых приёмов предлагать на выбор. */
const MAX_CANDIDATES = 5;
const UNSCHEDULED = "unscheduled";

interface FormState {
  treatment_id: number | null;
  /** id планового приёма, который отмечаем задним числом, либо «вне расписания». */
  target: number | typeof UNSCHEDULED;
  taken_at: string;
  member_id: number | null;
  note: string;
}

const loading = ref(true);
const saving = ref(false);
const loadingCandidates = ref(false);
const dogs = ref<Dog[]>([]);
const treatments = ref<Treatment[]>([]);
const members = ref<FamilyMember[]>([]);
const candidates = ref<DoseView[]>([]);

const maxValue = toDatetimeLocal(new Date().toISOString());

const form = reactive<FormState>({
  treatment_id: props.treatmentId ?? null,
  target: UNSCHEDULED,
  taken_at: maxValue,
  member_id: props.memberId ?? null,
  note: "",
});

const groups = computed(() =>
  dogs.value
    .map((dog) => ({
      dog,
      items: treatments.value.filter((t) => t.dog_id === dog.id),
    }))
    .filter((g) => g.items.length),
);

async function load() {
  if (settings.householdId == null) return;
  loading.value = true;
  try {
    const api = settings.api();
    const [dogList, allTreatments, memberList] = await Promise.all([
      api.listDogs(settings.householdId),
      api.listTreatments(),
      api.listMembers(settings.householdId),
    ]);
    dogs.value = dogList;
    const dogIds = new Set(dogList.map((d) => d.id));
    treatments.value = allTreatments.filter((t) => dogIds.has(t.dog_id));
    members.value = memberList;
    // Смена назначения сама подгрузит кандидатов (watch ниже).
    if (form.treatment_id == null) {
      form.treatment_id = treatments.value[0]?.id ?? null;
    } else {
      loadCandidates();
    }
  } catch (e) {
    toast.error(`Не удалось загрузить назначения: ${(e as Error).message}`);
  } finally {
    loading.value = false;
  }
}

// Номер последнего запроса: ответ на устаревший выбор назначения отбрасываем.
let candidatesReq = 0;

/** Неотмеченные плановые приёмы назначения, срок которых уже наступил. */
async function loadCandidates() {
  const treatmentId = form.treatment_id;
  const req = ++candidatesReq;
  candidates.value = [];
  pick(null);
  if (treatmentId == null || settings.householdId == null) {
    loadingCandidates.value = false;
    return;
  }
  loadingCandidates.value = true;
  try {
    const doses = await settings.api().listDoses({
      household_id: settings.householdId,
      treatment_id: treatmentId,
      to: new Date().toISOString(),
    });
    if (req !== candidatesReq) return;
    candidates.value = doses
      .filter((d) => d.status !== "taken")
      .sort((a, b) => new Date(b.due_at).getTime() - new Date(a.due_at).getTime())
      .slice(0, MAX_CANDIDATES);
    // Скорее всего, отмечают самый свежий пропуск — выбираем его сразу.
    pick(candidates.value[0] ?? null);
  } catch (e) {
    if (req === candidatesReq) {
      toast.error(`Не удалось загрузить приёмы: ${(e as Error).message}`);
    }
  } finally {
    if (req === candidatesReq) loadingCandidates.value = false;
  }
}

function pick(dose: DoseView | null) {
  form.target = dose ? dose.id : UNSCHEDULED;
  // Плановый приём по умолчанию датируем его плановым временем, внеплановый — текущим.
  form.taken_at = toDatetimeLocal(dose ? dose.due_at : new Date().toISOString());
}

async function submit() {
  if (form.treatment_id == null) {
    toast.error("Выберите назначение");
    return;
  }
  const takenAt = new Date(form.taken_at);
  if (Number.isNaN(takenAt.getTime())) {
    toast.error("Укажите дату и время приёма");
    return;
  }
  if (takenAt > new Date()) {
    toast.error("Время приёма не может быть в будущем");
    return;
  }
  saving.value = true;
  try {
    const api = settings.api();
    const common = {
      taken_at: takenAt.toISOString(),
      member_id: form.member_id,
      note: form.note.trim() || null,
    };
    if (form.target === UNSCHEDULED) {
      await api.createDose({ treatment_id: form.treatment_id, ...common });
      toast.success("Приём добавлен");
    } else {
      await api.setDoseStatus(form.target, { status: "taken", ...common });
      toast.success("Плановый приём отмечен принятым");
    }
    emit("saved");
  } catch (e) {
    toast.error(`Ошибка сохранения: ${(e as Error).message}`);
  } finally {
    saving.value = false;
  }
}

watch(() => form.treatment_id, loadCandidates);
onMounted(load);
</script>

<template>
  <ModalDialog
    title="Добавить пропущенный приём"
    :icon="CalendarPlus"
    @close="emit('close')"
  >
    <p v-if="loading" class="muted">Загружаю назначения…</p>
    <p v-else-if="!treatments.length" class="muted">
      Назначений пока нет — сначала добавьте лечение.
    </p>

    <div v-else class="form">
      <div class="field">
        <label>Назначение</label>
        <select v-model.number="form.treatment_id" class="select">
          <optgroup v-for="g in groups" :key="g.dog.id" :label="g.dog.name">
            <option v-for="t in g.items" :key="t.id" :value="t.id">
              {{ t.name }}{{ t.active ? "" : " (выключено)" }}
            </option>
          </optgroup>
        </select>
      </div>

      <div class="field">
        <label>Какой приём</label>
        <div class="options">
          <label
            v-for="c in candidates"
            :key="c.id"
            class="option"
            :class="{ on: form.target === c.id }"
          >
            <input
              type="radio"
              name="dose-target"
              :checked="form.target === c.id"
              @change="pick(c)"
            />
            <span class="option-text">
              <strong>По расписанию</strong>
              <span class="muted small">{{ formatDateTime(c.due_at) }}</span>
            </span>
            <StatusBadge :status="c.status" />
          </label>

          <label class="option" :class="{ on: form.target === UNSCHEDULED }">
            <input
              type="radio"
              name="dose-target"
              :checked="form.target === UNSCHEDULED"
              @change="pick(null)"
            />
            <span class="option-text">
              <strong>Вне расписания</strong>
              <span class="muted small">Приём, которого не было в расписании</span>
            </span>
          </label>
        </div>
        <span v-if="loadingCandidates" class="hint muted">
          Ищу неотмеченные приёмы…
        </span>
      </div>

      <div :class="{ two: members.length }">
        <div class="field">
          <label>Когда дали</label>
          <input
            v-model="form.taken_at"
            class="input"
            type="datetime-local"
            :max="maxValue"
          />
        </div>
        <div v-if="members.length" class="field">
          <label>Кто дал</label>
          <select v-model.number="form.member_id" class="select">
            <option :value="null">—</option>
            <option v-for="m in members" :key="m.id" :value="m.id">
              {{ m.display_name }}
            </option>
          </select>
        </div>
      </div>

      <div class="field">
        <label>Заметка <span class="optional">(необязательно)</span></label>
        <textarea
          v-model="form.note"
          class="textarea"
          placeholder="Напр. Дали в клинике после осмотра"
        />
        <span class="hint muted">Принятый приём попадёт в Веткнигу и отчёт.</span>
      </div>
    </div>

    <template #footer>
      <button class="btn btn-ghost" @click="emit('close')">Отмена</button>
      <button
        class="btn btn-primary"
        :disabled="saving || loading || form.treatment_id == null"
        @click="submit"
      >
        {{ saving ? "Сохраняю…" : "Добавить" }}
      </button>
    </template>
  </ModalDialog>
</template>

<style scoped>
.form {
  display: flex;
  flex-direction: column;
  gap: 0.85rem;
}
.two {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 0.85rem;
}
.options {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
}
.option {
  display: flex;
  align-items: center;
  gap: 0.7rem;
  padding: 0.55rem 0.8rem;
  border: 1.5px solid var(--corgi-wash);
  border-radius: var(--r-md);
  background: var(--surface);
  cursor: pointer;
  transition:
    border-color 0.12s ease,
    background 0.12s ease;
}
.option:hover {
  border-color: var(--corgi-soft);
}
.option.on {
  border-color: var(--corgi);
  background: var(--surface-2);
}
.option input {
  width: 17px;
  height: 17px;
  accent-color: var(--corgi);
  flex-shrink: 0;
}
.option-text {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
}
.optional {
  font-weight: 400;
  color: var(--ink-faint);
}
.hint {
  font-size: 0.78rem;
}
.small {
  font-size: 0.82rem;
}
</style>
