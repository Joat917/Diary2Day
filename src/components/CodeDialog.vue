<script setup lang="ts">
import { nextTick, ref, watch } from "vue";
import { t } from "../i18n";

const props = withDefaults(
  defineProps<{
    open: boolean;
    title: string;
    hint: string;
    confirmLabel: string;
    needsConfirm?: boolean;
    busy?: boolean;
    error?: string | null;
  }>(),
  { needsConfirm: false, busy: false, error: null },
);

const emit = defineEmits<{ submit: [code: string]; cancel: [] }>();

const code = ref("");
const confirmCode = ref("");
const localError = ref<string | null>(null);
const firstInput = ref<HTMLInputElement | null>(null);

watch(
  () => props.open,
  async (open) => {
    if (!open) return;
    code.value = "";
    confirmCode.value = "";
    localError.value = null;
    await nextTick();
    firstInput.value?.focus();
  },
);

function submit() {
  if (props.busy) return;
  localError.value = null;
  const value = code.value;
  if (value.length < 4) {
    localError.value = t("me.lockSetHint");
    return;
  }
  if (value !== value.trim()) {
    localError.value = t("me.lockSetHint");
    return;
  }
  if (props.needsConfirm && value !== confirmCode.value) {
    localError.value = t("me.lockMismatch");
    return;
  }
  emit("submit", value);
}

function onKeydown(event: KeyboardEvent) {
  if (!props.open) return;
  if (event.key === "Escape") {
    event.preventDefault();
    emit("cancel");
  }
}
</script>

<template>
  <Teleport to="body">
    <Transition name="dialog">
      <div v-if="open" class="scrim" @click.self="emit('cancel')">
        <form class="dialog" role="dialog" aria-modal="true" :aria-label="title" @submit.prevent="submit">
          <h2 class="title">{{ title }}</h2>
          <p class="hint">{{ hint }}</p>
          <label class="field">
            <span class="field-label">{{ needsConfirm ? t("me.lockCode") : t("me.lockDisableHint") }}</span>
            <input
              ref="firstInput"
              v-model="code"
              class="input"
              type="password"
              autocomplete="off"
              spellcheck="false"
              @input="localError = null"
              @keydown="onKeydown"
            />
          </label>
          <label v-if="needsConfirm" class="field">
            <span class="field-label">{{ t("me.lockConfirmCode") }}</span>
            <input
              v-model="confirmCode"
              class="input"
              type="password"
              autocomplete="off"
              spellcheck="false"
              @input="localError = null"
              @keydown="onKeydown"
            />
          </label>
          <p v-if="localError || error" class="error-text">{{ localError || error }}</p>
          <div class="actions">
            <button class="button" type="button" @click="emit('cancel')">
              {{ t("common.cancel") }}
            </button>
            <button class="button solid" type="submit" :disabled="busy">
              {{ confirmLabel }}
            </button>
          </div>
        </form>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.scrim {
  position: fixed;
  inset: 0;
  z-index: 100;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: var(--s-6);
  background: var(--c-scrim);
}

.dialog {
  width: min(100%, 360px);
  padding: var(--s-5);
  border-radius: var(--r-lg);
  background: var(--c-surface-1);
  border: 1px solid var(--c-hairline);
  box-shadow: var(--e-2);
}

.title {
  font-size: var(--t-title);
  line-height: var(--t-title-lh);
  font-weight: 500;
  color: var(--c-on-surface);
}

.hint {
  margin-top: var(--s-1);
  font-size: var(--t-small);
  line-height: var(--t-small-lh);
  color: var(--c-on-surface-muted);
}

.field {
  display: flex;
  flex-direction: column;
  gap: var(--s-1);
  margin-top: var(--s-3);
}

.field-label {
  font-size: var(--t-caption);
  line-height: var(--t-caption-lh);
  color: var(--c-on-surface-muted);
}

.input {
  height: 42px;
  padding-inline: var(--s-3);
  border: 1px solid var(--c-hairline);
  border-radius: var(--r-sm);
  background: var(--c-surface-2);
  color: var(--c-on-surface);
  font-size: var(--t-body);
  letter-spacing: 0.12em;
}

.input:focus-visible {
  border-color: var(--c-primary);
  outline: none;
}

.error-text {
  margin-top: var(--s-2);
  color: var(--c-danger);
  font-size: var(--t-caption);
  line-height: var(--t-caption-lh);
}

.actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--s-2);
  margin-top: var(--s-5);
}

.button {
  min-height: 38px;
  padding-inline: var(--s-4);
  border-radius: var(--r-pill);
  font-size: var(--t-small);
  color: var(--c-on-surface-muted);
  background: var(--c-surface-2);
}

.button.solid {
  background: var(--c-primary);
  color: var(--c-on-primary);
}

.button:disabled {
  opacity: 0.5;
}

.dialog-enter-active,
.dialog-leave-active {
  transition: opacity var(--m-2) var(--ease);
}

.dialog-enter-active .dialog,
.dialog-leave-active .dialog {
  transition: transform var(--m-2) var(--ease);
}

.dialog-enter-from,
.dialog-leave-to {
  opacity: 0;
}

.dialog-enter-from .dialog,
.dialog-leave-to .dialog {
  transform: scale(0.96);
}
</style>