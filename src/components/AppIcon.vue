<script setup lang="ts">
import { computed } from 'vue';
const props = defineProps<{ name: string; size?: number }>();
const paths: Record<string, string> = {
  today: 'M8 2v4m8-4v4M3 10h18M5 4h14a2 2 0 0 1 2 2v14H3V6a2 2 0 0 1 2-2Zm3 10h3v3H8z',
  plan: 'M3 5h18v16H3zM7 2v6m10-6v6M3 11h18M7 15h2m4 0h2m2 3h1',
  inbox: 'm3 5-1 14h20L21 5H3Zm0 8h5l2 3h4l2-3h5',
  done: 'm8 12 3 3 5-6M21 12a9 9 0 1 1-5-8',
  plus: 'M12 5v14M5 12h14',
  arrow: 'M5 12h14m-5-5 5 5-5 5',
  close: 'm6 6 12 12M6 18 18 6',
  check: 'm5 12 4 4L19 6',
  lock: 'M8 10V7a4 4 0 0 1 8 0v3M6 10h12a1 1 0 0 1 1 1v9a1 1 0 0 1-1 1H6a1 1 0 0 1-1-1v-9a1 1 0 0 1 1-1ZM12 14v3',
  unlock:
    'M8 10V6a4 4 0 0 1 7.5-2M6 10h12a1 1 0 0 1 1 1v9a1 1 0 0 1-1 1H6a1 1 0 0 1-1-1v-9a1 1 0 0 1 1-1ZM12 14v3',
  widget: 'M3 3h18v18H3zM3 8h18M8 8v13',
  spark: 'm12 3 2.5 6.5L21 12l-6.5 2.5L12 21l-2.5-6.5L3 12l6.5-2.5L12 3Z',
  shield: 'm12 3 8 3v6c0 5-8 9-8 9s-8-4-8-9V6l8-3Zm-4 9 3 3 5-6',
  clock: 'M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0ZM12 7v5l3 2',
  folder: 'M3 5h7l2 3h9v12H3V5Z',
};
const path = computed(() => paths[props.name] ?? paths.today);
</script>
<template>
  <svg
    :width="size ?? 20"
    :height="size ?? 20"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="1.65"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
  >
    <path :d="path" />
  </svg>
</template>
