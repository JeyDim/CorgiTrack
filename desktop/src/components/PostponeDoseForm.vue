<script setup lang="ts">
import { reactive, ref } from "vue";
import { CalendarClock } from "@lucide/vue";

import type { DoseView } from "../api/types";
import { useSettingsStore } from "../stores/settings";
import { useToastStore } from "../stores/toast";
import { formatDateTime, toDatetimeLocal } from "../util/format";
import ModalDialog from "./ModalDialog.vue";

const props = defineProps<{ dose: DoseView }>();
const emit = defineEmits<{
  (e: "close"): void;
  (e: "saved", dose: DoseView): void;
}>();

const settings = useSettingsStore();
const toast = useToastStore();
const saving = ref(false);

const PRESETS = [
  { label: "+1 день", days: 1 },
  { label: "+3 дня", days: 3 },
  { label: "+1 неделя", days: 7 },
];

// Сдвиг на N дней с сохранением времени приёма. Просроченный приём сдвигаем от
// сегодняшнего дня — иначе «+1 день» мог бы так и остаться в прошлом.
function shifted(days: number): string {
  const base = new Date(props.dose.due_at);
  const now = new Date();
  if (base < now) {
    base.setFullYear(now.getFullYear(), now.getMonth(), now.getDate());
  }
  base.setDate(base.getDate() + days);
  return toDatetimeLocal(base.toISOString());
}

const minValue = toDatetimeLocal(new Date().toISOString());

const form = reactive({
  due_at: shifted(1),
  note: "",
});

async function submit() {
  const due = new Date(form.due_at);
  if (Number.isNaN(due.getTime())) {
    toast.error("Укажите дату и время");
    return;
  }
  if (due <= new Date()) {
    toast.error("Отложить приём можно только на будущее время");
    return;
  }
  saving.value = true;
  try {
    const updated = await settings.api().postponeDose(props.dose.id, {
      // datetime-local → RFC3339 (UTC).
      due_at: due.toISOString(),
      note: form.note.trim() || null,
    });
    toast.success(`Приём перенесён на ${formatDateTime(updated.due_at)}`);
    emit("saved", updated);
  } catch (e) {
    toast.error(`Не удалось отложить: ${(e as Error).message}`);
  } finally {
    saving.value = false;
  }
}
</script>

<template>
  <ModalDialog title="Отложить приём" :icon="CalendarClock" @close="emit('close')">
    <div class="form">
      <div class="dose-info">
        <div>
          <strong>{{ dose.treatment_name }}</strong>
          <span class="muted"> · {{ dose.dog_name }}</span>
        </div>
        <div class="muted small">
          Запланировано на {{ formatDateTime(dose.due_at) }}
          <template v-if="dose.original_due_at">
            · по расписанию было {{ formatDateTime(dose.original_due_at) }}
          </template>
        </div>
      </div>

      <div class="field">
        <label>Перенести на</label>
        <div class="presets">
          <button
            v-for="p in PRESETS"
            :key="p.days"
            type="button"
            class="preset"
            :class="{ on: form.due_at === shifted(p.days) }"
            @click="form.due_at = shifted(p.days)"
          >
            {{ p.label }}
          </button>
        </div>
        <input
          v-model="form.due_at"
          class="input"
          type="datetime-local"
          :min="minValue"
        />
        <span class="hint muted">
          Напоминание придёт заново — к новой дате. Остальные приёмы по
          расписанию не сдвигаются.
        </span>
      </div>

      <div class="field">
        <label>Причина <span class="optional">(необязательно)</span></label>
        <textarea
          v-model="form.note"
          class="textarea"
          placeholder="Напр. Собака приболела — перенесли по совету врача"
        />
      </div>
    </div>

    <template #footer>
      <button class="btn btn-ghost" @click="emit('close')">Отмена</button>
      <button class="btn btn-primary" :disabled="saving" @click="submit">
        {{ saving ? "Переношу…" : "Отложить" }}
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
.dose-info {
  display: flex;
  flex-direction: column;
  gap: 0.2rem;
  padding: 0.7rem 0.9rem;
  background: var(--corgi-wash);
  border-radius: var(--r-md);
}
.presets {
  display: flex;
  gap: 0.4rem;
  flex-wrap: wrap;
}
.preset {
  border: 1.5px solid var(--corgi-wash);
  background: var(--surface);
  font-family: var(--font-display);
  font-weight: 500;
  font-size: 0.85rem;
  color: var(--ink-soft);
  padding: 0.3rem 0.85rem;
  border-radius: var(--r-pill);
  cursor: pointer;
  transition: all 0.14s ease;
}
.preset:hover {
  border-color: var(--corgi-soft);
  color: var(--ink);
}
.preset.on {
  background: linear-gradient(135deg, var(--corgi), var(--corgi-deep));
  border-color: transparent;
  color: #fff;
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
