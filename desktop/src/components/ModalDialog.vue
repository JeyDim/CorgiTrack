<script setup lang="ts">
import { onMounted, onUnmounted, type Component } from "vue";
import { X } from "@lucide/vue";

defineProps<{ title?: string; icon?: Component }>();
const emit = defineEmits<{ (e: "close"): void }>();

function onKey(e: KeyboardEvent) {
  if (e.key === "Escape") emit("close");
}
onMounted(() => window.addEventListener("keydown", onKey));
onUnmounted(() => window.removeEventListener("keydown", onKey));
</script>

<template>
  <Teleport to="body">
    <div class="modal-backdrop" @click.self="emit('close')">
      <div class="modal card">
        <div class="modal-head">
          <h3>
            <component :is="icon" v-if="icon" :size="19" />
            {{ title }}
          </h3>
          <button class="btn btn-ghost btn-sm" @click="emit('close')">
            <X :size="16" />
          </button>
        </div>
        <div class="modal-body">
          <slot />
        </div>
        <div v-if="$slots.footer" class="modal-foot">
          <slot name="footer" />
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.modal-backdrop {
  position: fixed;
  inset: 0;
  background: rgba(74, 52, 42, 0.4);
  backdrop-filter: blur(3px);
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 1.5rem;
  z-index: 900;
  animation: fade 0.18s ease;
}
.modal {
  width: 100%;
  max-width: 520px;
  max-height: 90vh;
  overflow: auto;
  box-shadow: var(--shadow-lg);
  animation: pop 0.22s cubic-bezier(0.2, 0.8, 0.3, 1.15);
}
.modal-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 1.1rem 1.3rem 0.6rem;
}
.modal-head h3 {
  display: inline-flex;
  align-items: center;
  gap: 0.5rem;
}
.modal-body {
  padding: 0.4rem 1.3rem 1rem;
}
.modal-foot {
  padding: 0.6rem 1.3rem 1.2rem;
  display: flex;
  justify-content: flex-end;
  gap: 0.6rem;
}
@keyframes fade {
  from {
    opacity: 0;
  }
}
@keyframes pop {
  from {
    opacity: 0;
    transform: translateY(14px) scale(0.96);
  }
}
</style>
