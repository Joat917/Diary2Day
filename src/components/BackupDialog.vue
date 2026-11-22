<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import { t } from "../i18n";
import { useBackLayer } from "../composables/useBackLayer";

const props = withDefaults(
  defineProps<{
    open: boolean;
    mode: "create" | "restore";
    busy?: boolean;
    error?: string | null;
    initialPassword?: string | null;
  }>(),
  { busy: false, error: null, initialPassword: null },
);

const emit = defineEmits<{
  submit: [password: string | null, remember: boolean];
  cancel: [];
}>();

useBackLayer(
  () => props.open,
  () => emit("cancel"),
);

const password = ref("");
const confirmPassword = ref("");
const remember = ref(false);
const localError = ref<string | null>(null);
const firstInput = ref<HTMLInputElement | null>(null);

const isCreate = computed(() => props.mode === "create");
const title = computed(() =>
  isCreate.value ? t("me.backupTitle") : t("me.restorePasswordTitle"),
);
const hint = computed(() => {
  if (!isCreate.value) return t("me.restorePasswordHint");
  if (!password.value) return t("me.backupEncryptHintIdle");
  return Array.from(password.value).length < 8
    ? t("me.backupEncryptHintShort")
    : t("me.backupEncryptHintReady");
});
const confirmLabel = computed(() =>
  isCreate.value ? t("me.backupStart") : t("me.restore"),
);

watch(
  () => props.open,
  async (open) => {
    if (!open) return;
    password.value = props.initialPassword ?? "";
    confirmPassword.value = password.value;
    remember.value = password.value.length > 0;
    localError.value = null;
    await nextTick();
    firstInput.value?.focus();
  },
);

function submit() {
  if (props.busy) return;
  localError.value = null;
  const value = password.value;
  if (!isCreate.value) {
    if (!value) {
      localError.value = t("me.backupPwRequired");
      return;
    }
    emit("submit", value, remember.value);
    return;
  }
  if (!value && !confirmPassword.value) {
    emit("submit", null, remember.value);
    return;
  }
  if (value.trim() !== value) {
    localError.value = t("me.backupPwSpaces");
    return;
  }
  if (value !== confirmPassword.value) {
    localError.value = t("me.backupPwMismatch");
    return;
  }
  emit("submit", value, remember.value);
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
            <span class="field-label">
              {{ isCreate ? t("me.backupPassword") : t("me.restorePassword") }}
            </span>
            <input
              ref="firstInput"
              v-model="password"
              class="input"
              type="password"
              :autocomplete="isCreate ? 'new-password' : 'off'"
              spellcheck="false"
              @input="localError = null"
              @keydown="onKeydown"
            />
          </label>
          <label v-if="isCreate" class="field">
            <span class="field-label">{{ t("me.backupPasswordConfirm") }}</span>
            <input
              v-model="confirmPassword"
              class="input"
              type="password"
              autocomplete="new-password"
              spellcheck="false"
              @input="localError = null"
              @keydown="onKeydown"
            />
          </label>
          <label class="remember">
            <input v-model="remember" type="checkbox" />
            <span>{{ t("me.backupRemember") }}</span>
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
  padding-inline: calc(var(--s-6) + var(--safe-left)) calc(var(--s-6) + var(--safe-right));
  padding-bottom: calc(var(--s-6) + var(--ime));
  background: var(--c-scrim);
}

.dialog {
  width: min(100%, 360px);
  max-height: 100%;
  overflow-y: auto;
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

.remember {
  display: flex;
  align-items: center;
  gap: var(--s-2);
  margin-top: var(--s-3);
  color: var(--c-on-surface-muted);
  font-size: var(--t-small);
  line-height: var(--t-small-lh);
  cursor: pointer;
}

.remember input {
  width: 16px;
  height: 16px;
  accent-color: var(--c-primary);
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