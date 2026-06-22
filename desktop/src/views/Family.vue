<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useRouter } from "vue-router";

import type { FamilyMember } from "../api/types";
import { useSettingsStore } from "../stores/settings";
import { useToastStore } from "../stores/toast";
import MemberForm from "../components/MemberForm.vue";
import ModalDialog from "../components/ModalDialog.vue";

const router = useRouter();
const settings = useSettingsStore();
const toast = useToastStore();

const loading = ref(false);
const members = ref<FamilyMember[]>([]);

const showForm = ref(false);
const editing = ref<FamilyMember | null>(null);
const deleteTarget = ref<FamilyMember | null>(null);
const busyId = ref<number | null>(null);

const hasHousehold = computed(() => settings.householdId != null);

async function reload() {
  if (settings.householdId == null) return;
  loading.value = true;
  try {
    const list = await settings.api().listMembers(settings.householdId);
    members.value = list.sort(
      (a, b) => a.escalation_order - b.escalation_order || a.id - b.id,
    );
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

function openEdit(m: FamilyMember) {
  editing.value = m;
  showForm.value = true;
}

function onSaved() {
  showForm.value = false;
  editing.value = null;
  reload();
}

async function toggleNotify(m: FamilyMember) {
  busyId.value = m.id;
  try {
    const updated = await settings
      .api()
      .updateMember(m.id, { notify: !m.notify });
    m.notify = updated.notify;
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
    await settings.api().deleteMember(target.id);
    toast.success("Участник удалён");
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
        <h1>👪 Семья</h1>
        <p class="muted">Участники, которым бот пишет напоминания о приёмах.</p>
      </div>
      <button v-if="hasHousehold" class="btn btn-primary" @click="openCreate">
        ➕ Добавить
      </button>
    </header>

    <div v-if="!hasHousehold" class="empty card">
      <div class="empty-emoji">🐕‍🦺</div>
      <h3>Семья не выбрана</h3>
      <p class="muted">Выберите семью в настройках, чтобы управлять участниками.</p>
      <button class="btn btn-primary" @click="router.push({ name: 'settings' })">
        Перейти в настройки
      </button>
    </div>

    <div v-else-if="loading" class="loading">
      <span class="paw-loader"><span>🐾</span><span>🐾</span><span>🐾</span></span>
    </div>

    <template v-else>
      <div class="card info">
        <span class="info-ico">📣</span>
        <p class="muted small">
          При напоминании сначала пишем участнику с наименьшим порядком. Если за
          отведённое время никто не нажал «Принято» — повтор тому же человеку,
          затем следующий по порядку. Учитываются только участники с галочкой
          «Уведомлять» и привязанным Telegram.
        </p>
      </div>

      <div v-if="!members.length" class="empty card">
        <div class="empty-emoji">🧑‍🤝‍🧑</div>
        <h3>Пока никого нет</h3>
        <p class="muted">
          Добавьте участников семьи — им бот будет присылать напоминания.
        </p>
        <button class="btn btn-primary" @click="openCreate">➕ Добавить участника</button>
      </div>

      <div v-else class="m-list">
        <article
          v-for="m in members"
          :key="m.id"
          class="m-row card"
          :class="{ 'muted-row': !m.notify }"
        >
          <div class="m-order" :title="`Порядок напоминаний: ${m.escalation_order}`">
            №{{ m.escalation_order }}
          </div>

          <div class="m-main">
            <strong>{{ m.display_name }}</strong>
            <div class="m-meta muted small">
              <span v-if="m.telegram_user_id" class="tg-tag">
                ✈️ {{ m.telegram_user_id }}
              </span>
              <span v-else class="tg-tag off">Telegram не привязан</span>
            </div>
          </div>

          <button
            class="toggle"
            :class="{ on: m.notify }"
            :disabled="busyId === m.id"
            :title="m.notify ? 'Уведомлять' : 'Не уведомлять'"
            @click="toggleNotify(m)"
          >
            <span class="knob" />
          </button>

          <div class="m-actions">
            <button class="btn btn-ghost btn-sm" @click="openEdit(m)">✏️</button>
            <button class="btn btn-ghost btn-sm" @click="deleteTarget = m">🗑</button>
          </div>
        </article>
      </div>
    </template>

    <!-- форма создания/редактирования -->
    <MemberForm
      v-if="showForm && settings.householdId != null"
      :household-id="settings.householdId"
      :member="editing"
      @close="showForm = false"
      @saved="onSaved"
    />

    <!-- подтверждение удаления -->
    <ModalDialog
      v-if="deleteTarget"
      title="Удалить участника?"
      @close="deleteTarget = null"
    >
      <p>
        Участник <strong>«{{ deleteTarget.display_name }}»</strong> будет удалён и
        перестанет получать напоминания. Действие необратимо.
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

.info {
  display: flex;
  align-items: flex-start;
  gap: 0.7rem;
  padding: 0.9rem 1.1rem;
  background: var(--corgi-wash);
}
.info-ico {
  font-size: 1.2rem;
  line-height: 1.4;
}
.info p {
  margin: 0;
}
.small {
  font-size: 0.82rem;
}

.m-list {
  display: flex;
  flex-direction: column;
  gap: 0.6rem;
}
.m-row {
  display: flex;
  align-items: center;
  gap: 0.9rem;
  padding: 0.8rem 1rem;
  transition:
    transform 0.14s ease,
    box-shadow 0.14s ease;
}
.m-row:hover {
  transform: translateY(-1px);
  box-shadow: var(--shadow-md);
}
.m-row.muted-row {
  opacity: 0.6;
}
.m-order {
  width: 2.6rem;
  flex: none;
  text-align: center;
  font-weight: 700;
  font-family: var(--font-display);
  color: var(--corgi-deep);
  background: var(--corgi-wash);
  padding: 0.3rem 0;
  border-radius: var(--r-md);
}
.m-main {
  flex: 1;
  min-width: 0;
}
.m-main strong {
  font-size: 1rem;
}
.m-meta {
  display: flex;
  gap: 0.35rem;
  flex-wrap: wrap;
  margin-top: 0.2rem;
}
.tg-tag {
  background: var(--corgi-wash);
  color: var(--ink);
  padding: 0.02rem 0.5rem;
  border-radius: var(--r-pill);
  font-weight: 600;
}
.tg-tag.off {
  background: var(--paper-deep);
  font-weight: 500;
}
.m-actions {
  display: flex;
  gap: 0.2rem;
}

/* переключатель уведомлений */
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
