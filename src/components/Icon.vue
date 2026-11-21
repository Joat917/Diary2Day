<script setup lang="ts">
withDefaults(
  defineProps<{
    path: string;
    size?: number;
    label?: string;
  }>(),
  { size: 20 },
);
</script>

<template>
  <svg
    :width="size"
    :height="size"
    viewBox="0 0 24 24"
    fill="currentColor"
    :aria-hidden="label ? undefined : 'true'"
    :aria-label="label"
    :role="label ? 'img' : undefined"
    focusable="false"
  >
    <path :d="path" />
  </svg>
</template>