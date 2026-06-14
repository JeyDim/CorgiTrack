<script setup lang="ts">
import { computed } from "vue";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useUpdaterStore } from "../stores/updater";
import { useToastStore } from "../stores/toast";

const updater = useUpdaterStore();
const toast = useToastStore();

const RELEASES_URL = "https://github.com/JeyDim/CorgiTrack/releases/latest";

const show = computed(() => updater.available && !updater.dismissed);

async function onUpdate() {
  try {
    await updater.install();
    // при успехе приложение перезапустится — код ниже уже не выполнится
  } catch {
    // авто-установка не удалась — предлагаем скачать вручную
    toast.error("Не удалось установить обновление, открываю страницу загрузки");
    try {
      await openUrl(RELEASES_URL);
    } catch {
      /* ничего не делаем — пользователь увидел тост */
    }
  }
}
</script>

<template>
  <Teleport to="body">
    <Transition name="update">
      <div v-if="show" class="update-banner" :class="{ busy: updater.installing }">
        <span class="ico">🐕</span>

        <div class="msg">
          <strong>Доступна версия {{ updater.version }}</strong>
          <span class="sub">
            {{
              updater.installing
                ? `Загрузка… ${updater.progress}%`
                : "Нажмите, чтобы обновить CorgiTrack"
            }}
          </span>
        </div>

        <button class="btn" :disabled="updater.installing" @click="onUpdate">
          {{ updater.installing ? "Обновляю…" : "Обновить" }}
        </button>

        <button
          class="close"
          :disabled="updater.installing"
          aria-label="Скрыть"
          @click="updater.dismiss()"
        >
          ✕
        </button>

        <div v-if="updater.installing" class="progress">
          <div class="fill" :style="{ width: updater.progress + '%' }" />
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.update-banner {
  position: fixed;
  top: 16px;
  left: 50%;
  transform: translateX(-50%);
  z-index: 1100;

  display: flex;
  align-items: center;
  gap: 0.7rem;
  max-width: min(92vw, 460px);
  padding: 0.6rem 0.7rem 0.6rem 0.9rem;

  background: var(--surface);
  border: 1px solid var(--corgi-soft);
  border-radius: var(--r-pill);
  box-shadow: var(--shadow-md);
  overflow: hidden;
}

.ico {
  font-size: 1.25rem;
  flex: none;
}

.msg {
  display: flex;
  flex-direction: column;
  line-height: 1.15;
  min-width: 0;
}
.msg strong {
  font-family: var(--font-display);
  font-size: 0.92rem;
  color: var(--ink);
}
.msg .sub {
  font-size: 0.74rem;
  color: var(--ink-soft);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.btn {
  flex: none;
  cursor: pointer;
  padding: 0.4rem 0.95rem;
  border: none;
  border-radius: var(--r-pill);
  background: linear-gradient(135deg, var(--corgi), var(--corgi-deep));
  color: #fff;
  font-family: var(--font-display);
  font-weight: 600;
  font-size: 0.85rem;
  box-shadow: 0 4px 12px rgba(201, 116, 43, 0.32);
  transition:
    transform 0.14s ease,
    filter 0.14s ease;
}
.btn:hover:not(:disabled) {
  transform: translateY(-1px);
  filter: brightness(1.04);
}
.btn:disabled {
  cursor: default;
  opacity: 0.75;
}

.close {
  flex: none;
  cursor: pointer;
  width: 1.7rem;
  height: 1.7rem;
  border: none;
  border-radius: 50%;
  background: transparent;
  color: var(--ink-soft);
  font-size: 0.9rem;
  transition:
    background 0.14s ease,
    color 0.14s ease;
}
.close:hover:not(:disabled) {
  background: var(--corgi-wash);
  color: var(--ink);
}
.close:disabled {
  opacity: 0.4;
  cursor: default;
}

.progress {
  position: absolute;
  left: 0;
  right: 0;
  bottom: 0;
  height: 3px;
  background: var(--corgi-wash);
}
.progress .fill {
  height: 100%;
  background: linear-gradient(90deg, var(--corgi), var(--corgi-deep));
  transition: width 0.2s ease;
}

/* плавное появление сверху */
.update-enter-active,
.update-leave-active {
  transition:
    transform 0.3s cubic-bezier(0.2, 0.8, 0.3, 1.2),
    opacity 0.3s ease;
}
.update-enter-from,
.update-leave-to {
  transform: translate(-50%, -120%);
  opacity: 0;
}
</style>
