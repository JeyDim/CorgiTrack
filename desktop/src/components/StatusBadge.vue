<script setup lang="ts">
import { computed, type Component } from "vue";
import { Ban, Bell, CircleCheck, CircleX, Clock } from "@lucide/vue";
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

const icon = computed<Component>(() => {
  switch (props.status) {
    case "taken":
      return CircleCheck;
    case "missed":
      return CircleX;
    case "skipped":
      return Ban;
    case "reminded":
      return Bell;
    default:
      return Clock;
  }
});
</script>

<template>
  <span class="badge" :class="tone">
    <component :is="icon" :size="14" />
    {{ DOSE_STATUS_LABEL[status] }}
  </span>
</template>
