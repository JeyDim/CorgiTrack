<script setup lang="ts">
import { computed } from "vue";
import type { DoseStatus } from "../api/types";
import { DOSE_STATUS_LABEL } from "../api/types";

const props = defineProps<{ status: DoseStatus }>();

const tone = computed(() => {
  switch (props.status) {
    case "taken":
      return "ok";
    case "missed":
    case "skipped":
      return "danger";
    case "reminded":
      return "warn";
    default:
      return "calm";
  }
});

const icon = computed(() => {
  switch (props.status) {
    case "taken":
      return "✅";
    case "missed":
      return "⛔";
    case "skipped":
      return "🚫";
    case "reminded":
      return "🔔";
    default:
      return "🕒";
  }
});
</script>

<template>
  <span class="badge" :class="tone">{{ icon }} {{ DOSE_STATUS_LABEL[status] }}</span>
</template>
