<script setup lang="ts">
import { mdiAlertCircleOutline, mdiBookOpenPageVariantOutline } from "@mdi/js";
import Icon from "./Icon.vue";

withDefaults(
  defineProps<{
    kind: "loading" | "empty" | "error";
    title?: string;
    hint?: string;
    actionLabel?: string;
  }>(),
  {},
);

defineEmits<{ action: [] }>();
</script>

<template>
  <div v-if="kind === 'loading'" class="state" aria-busy="true">
    <div class="skeleton" />
    <div class="skeleton" />
    <div class="skeleton short" />
  </div>

  <div v-else-if="kind === 'empty'" class="state centered">
    <Icon :path="mdiBookOpenPageVariantOutline" :size="32" class="icon" />
    <p class="title">{{ title }}</p>
    <p v-if="hint" class="hint">{{ hint }}</p>
    <button v-if="actionLabel" class="action" type="button" @click="$emit('action')">
      {{ actionLabel }}
    </button>
  </div>

  <div v-else class="state error">
    <Icon :path="mdiAlertCircleOutline" :size="24" class="icon error-icon" />
    <p class="title error-title">{{ title }}</p>
    <p v-if="hint" class="hint">{{ hint }}</p>
    <button v-if="actionLabel" class="action" type="button" @click="$emit('action')">
      {{ actionLabel }}
    </button>
  </div>
</template>

<style scoped>
.state {
  display: flex;
  flex-direction: column;
  gap: var(--s-2);
  padding: var(--s-6) var(--s-4);
}

.state.centered {
  align-items: center;
  text-align: center;
  padding: var(--s-8) var(--s-4);
}

.icon {
  color: var(--c-on-surface-faint);
}

.error {
  gap: var(--s-1);
  padding: var(--s-4);
  border: 1px solid var(--c-hairline);
  border-radius: var(--r-md);
  background: var(--c-surface-1);
}

.icon.error-icon,
.error-title {
  color: var(--c-danger);
}

.title {
  font-size: var(--t-body);
  color: var(--c-on-surface);
}

.hint {
  font-size: var(--t-small);
  line-height: var(--t-small-lh);
  color: var(--c-on-surface-muted);
}

.error .hint {
  white-space: pre-wrap;
  overflow-wrap: anywhere;
  user-select: text;
}

.action {
  margin-top: var(--s-3);
  align-self: center;
  padding: var(--s-2) var(--s-4);
  border-radius: var(--r-pill);
  background: var(--c-primary);
  color: var(--c-on-primary);
  font-size: var(--t-small);
  min-height: 44px;
}

.error .action {
  align-self: flex-start;
  background: var(--c-surface-2);
  color: var(--c-on-surface);
}

.skeleton {
  height: 84px;
  border-radius: var(--r-md);
  background: var(--c-surface-2);
  animation: shimmer 1.2s infinite;
}

.skeleton.short {
  height: 44px;
}

@keyframes shimmer {
  0%,
  100% {
    opacity: 0.55;
  }
  50% {
    opacity: 1;
  }
}
</style>