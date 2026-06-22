<script setup lang="ts">
import { reactive, ref } from "vue";
import type { CreateDog, Dog } from "../api/types";
import { Pencil, Plus } from "@lucide/vue";
import { useSettingsStore } from "../stores/settings";
import { useToastStore } from "../stores/toast";
import ModalDialog from "./ModalDialog.vue";

const props = defineProps<{
  householdId: number;
  dog?: Dog | null;
}>();
const emit = defineEmits<{ (e: "close"): void; (e: "saved"): void }>();

const settings = useSettingsStore();
const toast = useToastStore();

const isEdit = !!props.dog;
const saving = ref(false);

const form = reactive({
  name: props.dog?.name ?? "",
});

async function submit() {
  const name = form.name.trim();
  if (!name) {
    toast.error("Введите кличку собаки");
    return;
  }

  saving.value = true;
  try {
    const api = settings.api();
    if (isEdit && props.dog) {
      await api.updateDog(props.dog.id, { name });
      toast.success("Собака обновлена");
    } else {
      const payload: CreateDog = {
        household_id: props.householdId,
        name,
      };
      await api.createDog(payload);
      toast.success("Собака добавлена");
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
    :title="isEdit ? 'Изменить собаку' : 'Новая собака'"
    :icon="isEdit ? Pencil : Plus"
    @close="emit('close')"
  >
    <div class="form">
      <div class="field">
        <label>Кличка</label>
        <input
          v-model="form.name"
          class="input"
          placeholder="Напр. Рекс"
          @keyup.enter="submit"
        />
      </div>
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
</style>
