<script setup lang="ts">
import { reactive, ref } from "vue";
import type {
  CreateTreatment,
  Dog,
  PillCategory,
  Treatment,
  TreatmentKind,
} from "../api/types";
import { PILL_CATEGORY_LABEL, TREATMENT_KIND_LABEL } from "../api/types";
import { Pencil, Plus } from "@lucide/vue";
import { useSettingsStore } from "../stores/settings";
import { useToastStore } from "../stores/toast";
import { toDatetimeLocal } from "../util/format";
import ModalDialog from "./ModalDialog.vue";

const props = defineProps<{
  dogs: Dog[];
  treatment?: Treatment | null;
}>();
const emit = defineEmits<{ (e: "close"): void; (e: "saved"): void }>();

const settings = useSettingsStore();
const toast = useToastStore();

const isEdit = !!props.treatment;
const saving = ref(false);

// Дефолт старта — сейчас, время напоминания — 09:00.
const form = reactive({
  dog_id: props.treatment?.dog_id ?? props.dogs[0]?.id ?? null,
  name: props.treatment?.name ?? "",
  kind: (props.treatment?.kind ?? "pill") as TreatmentKind,
  // Категория осмысленна только для таблеток; дефолт — «от гельминтов».
  category: (props.treatment?.category ?? "worm") as PillCategory,
  dose_label: props.treatment?.dose_label ?? "",
  cycle_days: props.treatment?.cycle_days ?? 90,
  start_at: props.treatment
    ? toDatetimeLocal(props.treatment.start_at)
    : toDatetimeLocal(new Date().toISOString()),
  reminder_time: props.treatment?.reminder_time?.slice(0, 5) ?? "09:00",
  instructions: props.treatment?.instructions ?? "",
  clinic: props.treatment?.clinic ?? "",
  active: props.treatment?.active ?? true,
});

async function submit() {
  if (!form.name.trim()) {
    toast.error("Введите название");
    return;
  }
  if (form.dog_id == null) {
    toast.error("Выберите собаку");
    return;
  }
  saving.value = true;
  try {
    const payload: CreateTreatment = {
      dog_id: form.dog_id,
      name: form.name.trim(),
      kind: form.kind,
      // Категорию шлём только для таблеток; у прививок — null.
      category: form.kind === "pill" ? form.category : null,
      dose_label: form.dose_label.trim() || null,
      cycle_days: form.cycle_days,
      // datetime-local → RFC3339 (UTC).
      start_at: new Date(form.start_at).toISOString(),
      reminder_time: `${form.reminder_time}:00`,
      instructions: form.instructions.trim() || null,
      // Клиника осмысленна только для прививок — для таблеток не отправляем.
      clinic: form.kind === "vaccine" ? form.clinic.trim() || null : null,
      active: form.active,
    };

    const api = settings.api();
    if (isEdit && props.treatment) {
      const { dog_id: _omit, ...patch } = payload;
      await api.updateTreatment(props.treatment.id, patch);
      toast.success("Назначение обновлено");
    } else {
      await api.createTreatment(payload);
      toast.success("Назначение добавлено");
    }
    emit("saved");
  } catch (e) {
    toast.error(`Ошибка сохранения: ${(e as Error).message}`);
  } finally {
    saving.value = false;
  }
}
</script>

<template>
  <ModalDialog
    :title="isEdit ? 'Изменить назначение' : 'Новое назначение'"
    :icon="isEdit ? Pencil : Plus"
    @close="emit('close')"
  >
    <div class="form">
      <div class="field">
        <label>Собака</label>
        <select v-model.number="form.dog_id" class="select" :disabled="isEdit">
          <option v-for="d in dogs" :key="d.id" :value="d.id">{{ d.name }}</option>
        </select>
      </div>

      <div class="field">
        <label>Название</label>
        <input v-model="form.name" class="input" placeholder="Напр. Таблетка от паразитов" />
      </div>

      <div class="two">
        <div class="field">
          <label>Вид</label>
          <select v-model="form.kind" class="select">
            <option v-for="(label, value) in TREATMENT_KIND_LABEL" :key="value" :value="value">
              {{ label }}
            </option>
          </select>
        </div>
        <div class="field">
          <label>Доза</label>
          <input v-model="form.dose_label" class="input" placeholder="1 таблетка" />
        </div>
      </div>

      <div v-if="form.kind === 'pill'" class="field">
        <label>Тип таблетки</label>
        <select v-model="form.category" class="select">
          <option v-for="(label, value) in PILL_CATEGORY_LABEL" :key="value" :value="value">
            {{ label }}
          </option>
        </select>
        <span class="hint muted">Раздел в «Веткниге»: от клещей или от гельминтов.</span>
      </div>

      <div v-if="form.kind === 'vaccine'" class="field">
        <label>Ветклиника</label>
        <input
          v-model="form.clinic"
          class="input"
          placeholder="Напр. Беланта на Авиамоторной"
        />
        <span class="hint muted">Попадёт штампом в «Веткнигу» при отметке приёма.</span>
      </div>

      <div class="two">
        <div class="field">
          <label>Цикл, дней</label>
          <input v-model.number="form.cycle_days" class="input" type="number" min="1" />
        </div>
        <div class="field">
          <label>Время напоминания</label>
          <input v-model="form.reminder_time" class="input" type="time" />
        </div>
      </div>

      <div class="field">
        <label>Старт (первый приём)</label>
        <input v-model="form.start_at" class="input" type="datetime-local" />
      </div>

      <div class="field">
        <label>Инструкция</label>
        <textarea
          v-model="form.instructions"
          class="textarea"
          placeholder="Напр. Дать после еды"
        />
      </div>

      <label class="check">
        <input v-model="form.active" type="checkbox" />
        <span>Активно (генерировать дозы и напоминания)</span>
      </label>
    </div>

    <template #footer>
      <button class="btn btn-ghost" @click="emit('close')">Отмена</button>
      <button class="btn btn-primary" :disabled="saving" @click="submit">
        {{ saving ? "Сохраняю…" : isEdit ? "Сохранить" : "Добавить" }}
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
.hint {
  font-size: 0.78rem;
}
.check {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-size: 0.9rem;
  color: var(--ink);
  cursor: pointer;
}
.check input {
  width: 18px;
  height: 18px;
  accent-color: var(--corgi);
}
</style>
