<script setup lang="ts">
import { useToastStore, type ToastKind } from "../stores/toast";

const toast = useToastStore();

function icon(kind: ToastKind): string {
  if (kind === "success") return "🐾";
  if (kind === "error") return "⚠️";
  return "ℹ️";
}
</script>

<template>
  <Teleport to="body">
    <div class="toast-host">
      <TransitionGroup name="toast">
        <div
          v-for="t in toast.toasts"
          :key="t.id"
          class="toast"
          :class="t.kind"
          @click="toast.dismiss(t.id)"
        >
          <span class="ico">{{ icon(t.kind) }}</span>
          <span>{{ t.message }}</span>
        </div>
      </TransitionGroup>
    </div>
  </Teleport>
</template>

<style scoped>
.toast-host {
  position: fixed;
  right: 18px;
  bottom: 18px;
  display: flex;
  flex-direction: column;
  gap: 0.55rem;
  z-index: 1000;
  pointer-events: none;
}
.toast {
  pointer-events: auto;
  cursor: pointer;
  display: flex;
  align-items: center;
  gap: 0.55rem;
  min-width: 240px;
  max-width: 360px;
  padding: 0.7rem 1rem;
  border-radius: var(--r-md);
  background: var(--surface);
  box-shadow: var(--shadow-md);
  border-left: 5px solid var(--ink-faint);
  font-weight: 600;
  font-size: 0.92rem;
}
.toast .ico {
  font-size: 1.1rem;
}
.toast.success {
  border-left-color: var(--ok);
}
.toast.error {
  border-left-color: var(--danger);
}
.toast.info {
  border-left-color: var(--corgi);
}

.toast-enter-active,
.toast-leave-active {
  transition:
    transform 0.28s cubic-bezier(0.2, 0.8, 0.3, 1.2),
    opacity 0.28s ease;
}
.toast-enter-from,
.toast-leave-to {
  transform: translateX(120%) scale(0.9);
  opacity: 0;
}
</style>
