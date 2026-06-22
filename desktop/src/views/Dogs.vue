<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useRouter } from "vue-router";
import { Dog as DogIcon, PawPrint, Pencil, Plus, Trash2 } from "@lucide/vue";

import type { Dog } from "../api/types";
import { formatDate } from "../util/format";
import { useSettingsStore } from "../stores/settings";
import { useToastStore } from "../stores/toast";
import DogForm from "../components/DogForm.vue";
import ModalDialog from "../components/ModalDialog.vue";

const router = useRouter();
const settings = useSettingsStore();
const toast = useToastStore();

const loading = ref(false);
const dogs = ref<Dog[]>([]);

const showForm = ref(false);
const editing = ref<Dog | null>(null);
const deleteTarget = ref<Dog | null>(null);
const busyId = ref<number | null>(null);

const hasHousehold = computed(() => settings.householdId != null);

async function reload() {
  if (settings.householdId == null) return;
  loading.value = true;
  try {
    const list = await settings.api().listDogs(settings.householdId);
    dogs.value = list.sort((a, b) => a.id - b.id);
  } catch (e) {
    toast.error(`Не удалось загрузить: ${(e as Error).message}`);
  } finally {
    loading.value = false;
  }
}

function openCreate() {
  editing.value = null;
  showForm.value = true;
}

function openEdit(d: Dog) {
  editing.value = d;
  showForm.value = true;
}

function onSaved() {
  showForm.value = false;
  editing.value = null;
  reload();
}

async function confirmDelete() {
  if (!deleteTarget.value) return;
  const target = deleteTarget.value;
  busyId.value = target.id;
  try {
    await settings.api().deleteDog(target.id);
    toast.success("Собака удалена");
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
        <h1><DogIcon :size="24" /> Собаки</h1>
        <p class="muted">Питомцы семьи — к ним привязаны лечения и напоминания.</p>
      </div>
      <button v-if="hasHousehold" class="btn btn-primary" @click="openCreate">
        <Plus :size="18" /> Добавить
      </button>
    </header>

    <div v-if="!hasHousehold" class="empty card">
      <div class="empty-emoji"><DogIcon :size="52" /></div>
      <h3>Семья не выбрана</h3>
      <p class="muted">Выберите семью в настройках, чтобы управлять собаками.</p>
      <button class="btn btn-primary" @click="router.push({ name: 'settings' })">
        Перейти в настройки
      </button>
    </div>

    <div v-else-if="loading" class="loading">
      <span class="paw-loader">
        <span><PawPrint :size="22" /></span>
        <span><PawPrint :size="22" /></span>
        <span><PawPrint :size="22" /></span>
      </span>
    </div>

    <template v-else>
      <div v-if="!dogs.length" class="empty card">
        <div class="empty-emoji"><DogIcon :size="52" /></div>
        <h3>Пока нет собак</h3>
        <p class="muted">
          Добавьте питомца — затем можно будет заводить для него лечения.
        </p>
        <button class="btn btn-primary" @click="openCreate">
          <Plus :size="18" /> Добавить собаку
        </button>
      </div>

      <div v-else class="d-list">
        <article v-for="d in dogs" :key="d.id" class="d-row card">
          <div class="d-avatar"><DogIcon :size="22" /></div>

          <div class="d-main">
            <strong>{{ d.name }}</strong>
            <div class="d-meta muted small">
              Добавлена {{ formatDate(d.created_at) }}
            </div>
          </div>

          <div class="d-actions">
            <button
              class="btn btn-ghost btn-sm"
              title="Изменить"
              @click="openEdit(d)"
            >
              <Pencil :size="16" />
            </button>
            <button
              class="btn btn-ghost btn-sm"
              title="Удалить"
              @click="deleteTarget = d"
            >
              <Trash2 :size="16" />
            </button>
          </div>
        </article>
      </div>
    </template>

    <!-- форма создания/редактирования -->
    <DogForm
      v-if="showForm && settings.householdId != null"
      :household-id="settings.householdId"
      :dog="editing"
      @close="showForm = false"
      @saved="onSaved"
    />

    <!-- подтверждение удаления -->
    <ModalDialog
      v-if="deleteTarget"
      title="Удалить собаку?"
      @close="deleteTarget = null"
    >
      <p>
        Собака <strong>«{{ deleteTarget.name }}»</strong> будет удалена вместе со
        всеми её лечениями и историей приёмов. Действие необратимо.
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
  display: inline-flex;
  align-items: center;
  gap: 0.55rem;
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
  color: var(--corgi);
  line-height: 0;
}
.small {
  font-size: 0.82rem;
}

.d-list {
  display: flex;
  flex-direction: column;
  gap: 0.6rem;
}
.d-row {
  display: flex;
  align-items: center;
  gap: 0.9rem;
  padding: 0.8rem 1rem;
  transition:
    transform 0.14s ease,
    box-shadow 0.14s ease;
}
.d-row:hover {
  transform: translateY(-1px);
  box-shadow: var(--shadow-md);
}
.d-avatar {
  width: 2.8rem;
  height: 2.8rem;
  flex: none;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border-radius: 50%;
  color: var(--corgi-deep);
  background: var(--corgi-wash);
}
.d-main {
  flex: 1;
  min-width: 0;
}
.d-main strong {
  font-size: 1rem;
}
.d-meta {
  margin-top: 0.2rem;
}
.d-actions {
  display: flex;
  gap: 0.2rem;
}
</style>
