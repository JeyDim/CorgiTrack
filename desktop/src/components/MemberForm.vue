<script setup lang="ts">
import { reactive, ref } from "vue";
import type { CreateMember, FamilyMember } from "../api/types";
import { Pencil, Plus } from "@lucide/vue";
import { useSettingsStore } from "../stores/settings";
import { useToastStore } from "../stores/toast";
import ModalDialog from "./ModalDialog.vue";

const props = defineProps<{
  householdId: number;
  member?: FamilyMember | null;
}>();
const emit = defineEmits<{ (e: "close"): void; (e: "saved"): void }>();

const settings = useSettingsStore();
const toast = useToastStore();

const isEdit = !!props.member;
const saving = ref(false);

const form = reactive({
  display_name: props.member?.display_name ?? "",
  // Telegram ID храним строкой — пустая строка означает «не привязан».
  telegram_user_id: props.member?.telegram_user_id?.toString() ?? "",
  escalation_order: props.member?.escalation_order ?? 0,
  notify: props.member?.notify ?? true,
});

async function submit() {
  if (!form.display_name.trim()) {
    toast.error("Введите имя участника");
    return;
  }
  // Telegram ID опционален, но если задан — должен быть числом.
  const tgRaw = form.telegram_user_id.trim();
  let telegramId: number | null = null;
  if (tgRaw) {
    const parsed = Number(tgRaw);
    if (!Number.isInteger(parsed)) {
      toast.error("Telegram ID должен быть числом");
      return;
    }
    telegramId = parsed;
  }

  saving.value = true;
  try {
    const api = settings.api();
    if (isEdit && props.member) {
      await api.updateMember(props.member.id, {
        display_name: form.display_name.trim(),
        telegram_user_id: telegramId,
        escalation_order: form.escalation_order,
        notify: form.notify,
      });
      toast.success("Участник обновлён");
    } else {
      const payload: CreateMember = {
        household_id: props.householdId,
        display_name: form.display_name.trim(),
        telegram_user_id: telegramId,
        escalation_order: form.escalation_order,
        notify: form.notify,
      };
      await api.createMember(payload);
      toast.success("Участник добавлен");
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
    :title="isEdit ? 'Изменить участника' : 'Новый участник'"
    :icon="isEdit ? Pencil : Plus"
    @close="emit('close')"
  >
    <div class="form">
      <div class="field">
        <label>Имя</label>
        <input
          v-model="form.display_name"
          class="input"
          placeholder="Напр. Мама"
        />
      </div>

      <div class="field">
        <label>Telegram ID</label>
        <input
          v-model="form.telegram_user_id"
          class="input"
          inputmode="numeric"
          placeholder="Напр. 123456789"
        />
        <span class="hint muted">
          Обычно привязывается автоматически через бота. Можно оставить пустым.
        </span>
      </div>

      <div class="field">
        <label>Порядок напоминаний</label>
        <input
          v-model.number="form.escalation_order"
          class="input"
          type="number"
          min="0"
        />
        <span class="hint muted">
          0 уведомляется первым, затем 1, 2, … при эскалации.
        </span>
      </div>

      <label class="check">
        <input v-model="form.notify" type="checkbox" />
        <span>Уведомлять (участвует в напоминаниях)</span>
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
