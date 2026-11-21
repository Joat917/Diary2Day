<script setup lang="ts">
import { nextTick, ref, watch } from "vue";
import { mdiLockOutline } from "@mdi/js";
import { describeError, IpcError } from "../ipc/client";
import { t } from "../i18n";
import { useLock } from "../stores/lock";
import Icon from "./Icon.vue";

const lock = useLock();
const code = ref("");
const errorText = ref("");
const busy = ref(false);
const input = ref<HTMLInputElement | null>(null);

async function submit() {
  if (!code.value || busy.value) return;
  busy.value = true;
  errorText.value = "";
  try {
    await lock.unlock(code.value);
    code.value = "";
  } catch (err) {
    errorText.value =
      err instanceof IpcError && err.code === "bad_lock_code" ? t("lock.wrong") : describeError(err);
    code.value = "";
    await nextTick();
    input.value?.focus();
  } finally {
    busy.value = false;
  }
}

watch(
  () => lock.locked,
  async (locked) => {
    if (locked) {
      code.value = "";
      errorText.value = "";
      await nextTick();
      input.value?.focus();
    }
  },
  { immediate: true },
);
</script>

<template>
  <Transition name="lock">
    <div v-if="lock.locked" class="lock" role="dialog" aria-modal="true">
      <div class="panel">
        <span class="badge">
          <Icon :path="mdiLockOutline" :size="28" />
        </span>
        <h1 class="title">{{ t("lock.title") }}</h1>
        <p class="hint">{{ t("lock.hint") }}</p>
        <input
          ref="input"
          v-model="code"
          class="input"
          type="password"
          inputmode="numeric"
          autocomplete="off"
          :placeholder="t('lock.placeholder')"
          :class="{ error: !!errorText }"
          @keydown.enter.prevent="submit"
          @input="errorText = ''"
        />
        <p v-if="errorText" class="error-text">{{ errorText }}</p>
        <button class="submit" type="button" :disabled="!code || busy" @click="submit">
          {{ t("lock.unlock") }}
        </button>
      </div>
    </div>
  </Transition>
</template>

<style scoped>
.lock {
  position: fixed;
  inset: 0;
  z-index: 100;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: var(--s-4);
  background: var(--c-surface);
}

.panel {
  display: flex;
  flex-direction: column;
  align-items: center;
  width: min(360px, 100%);
  padding: var(--s-6) var(--s-5);
  border: 1px solid var(--c-hairline);
  border-radius: var(--r-lg);
  background: var(--c-surface-1);
  box-shadow: var(--e-2);
}

.badge {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 56px;
  height: 56px;
  margin-bottom: var(--s-3);
  border-radius: var(--r-pill);
  background: var(--c-primary-soft);
  color: var(--c-primary);
}

.title {
  font-size: var(--t-title);
  line-height: var(--t-title-lh);
  font-weight: 500;
}

.hint {
  margin-top: var(--s-1);
  color: var(--c-on-surface-muted);
  font-size: var(--t-small);
  text-align: center;
}

.input {
  width: 100%;
  height: 44px;
  margin-top: var(--s-4);
  padding-inline: var(--s-3);
  border: 1px solid var(--c-hairline);
  border-radius: var(--r-sm);
  background: var(--c-surface-2);
  color: var(--c-on-surface);
  font-size: var(--t-body);
  text-align: center;
  letter-spacing: 0.2em;
}

.input:focus-visible {
  border-color: var(--c-primary);
  outline: none;
}

.input.error {
  border-color: var(--c-danger);
}

.error-text {
  margin-top: var(--s-1);
  color: var(--c-danger);
  font-size: var(--t-caption);
  white-space: pre-wrap;
  overflow-wrap: anywhere;
  user-select: text;
}

.submit {
  width: 100%;
  min-height: 44px;
  margin-top: var(--s-4);
  border-radius: var(--r-pill);
  background: var(--c-primary);
  color: var(--c-on-primary);
  font-size: var(--t-small);
}

.submit:disabled {
  opacity: 0.5;
}

.lock-enter-active,
.lock-leave-active {
  transition: opacity var(--m-2) var(--ease);
}

.lock-enter-from,
.lock-leave-to {
  opacity: 0;
}
</style>