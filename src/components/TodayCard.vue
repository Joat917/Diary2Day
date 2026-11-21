<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref } from "vue";
import { mdiCheck, mdiSend } from "@mdi/js";
import { journalSave } from "../ipc/client";
import type { Journal } from "../ipc/types";
import { t } from "../i18n";
import { recordedAtForDay, todayKey } from "../utils/date";
import Icon from "./Icon.vue";

const props = defineProps<{ dayKey: string }>();
const emit = defineEmits<{ saved: [journal: Journal] }>();

const text = ref("");
const textarea = ref<HTMLTextAreaElement | null>(null);
const status = ref<"idle" | "saving" | "saved" | "error">("idle");
let resetTimer: number | undefined;

const wordCount = computed(() => text.value.trim().length);
const canSave = computed(() => text.value.trim().length > 0 && status.value !== "saving");

function resize() {
  const element = textarea.value;
  if (!element) return;
  element.style.height = "auto";
  const line = 26;
  element.style.height = `${Math.min(Math.max(element.scrollHeight, line), line * 6)}px`;
}

async function save() {
  if (!canSave.value) return;
  status.value = "saving";
  try {
    const stamp = recordedAtForDay(props.dayKey);
    const journal = await journalSave({
      content: text.value,
      recordedAt: stamp.recordedAt,
      recordedOff: stamp.recordedOff,
    });
    text.value = "";
    status.value = "saved";
    emit("saved", journal);
    await nextTick();
    resize();
    window.clearTimeout(resetTimer);
    resetTimer = window.setTimeout(() => {
      if (status.value === "saved") status.value = "idle";
    }, 2000);
  } catch {
    status.value = "error";
  }
}

function onKeydown(event: KeyboardEvent) {
  if (event.key === "Enter" && (event.ctrlKey || event.metaKey)) {
    event.preventDefault();
    void save();
  }
}

function focus() {
  textarea.value?.focus();
}

defineExpose({ focus });

onBeforeUnmount(() => window.clearTimeout(resetTimer));
</script>

<template>
  <section class="card">
    <textarea
      ref="textarea"
      v-model="text"
      class="input"
      rows="1"
      :placeholder="props.dayKey === todayKey() ? t('today.placeholder') : t('today.emptyAction') + '…'"
      @input="resize"
      @blur="save"
      @keydown="onKeydown"
    />
    <div class="footer">
      <span class="status" :class="status">
        <template v-if="status === 'saving'">{{ t('detail.saving') }}</template>
        <template v-else-if="status === 'saved'">
          <Icon :path="mdiCheck" :size="14" />
          {{ t('today.saved') }}
        </template>
        <template v-else-if="status === 'error'">
          {{ t('detail.saveError') }} · {{ t('common.retry') }}
        </template>
        <template v-else>
          {{ wordCount ? t("today.words", { n: wordCount }) : t("today.saveHint") }}
        </template>
      </span>
      <button
        v-if="text.trim()"
        class="save"
        type="button"
        :disabled="status === 'saving'"
        @mousedown.prevent
        @click="save"
      >
        <Icon :path="mdiSend" :size="16" />
        {{ t("today.save") }}
      </button>
    </div>
  </section>
</template>

<style scoped>
.card {
  margin: var(--s-4);
  padding: var(--s-3) var(--s-4) var(--s-2);
  border-radius: var(--r-md);
  background: var(--c-surface-1);
  border: 1px solid var(--c-hairline);
  transition: border-color var(--m-1) var(--ease);
}

.card:focus-within {
  border-color: var(--c-primary);
}

.input {
  display: block;
  width: 100%;
  min-height: 26px;
  max-height: 156px;
  resize: none;
  border: none;
  outline: none;
  background: transparent;
  font-size: var(--t-body);
  line-height: 26px;
  color: var(--c-on-surface);
  overflow-y: auto;
}

.input::placeholder {
  color: var(--c-on-surface-faint);
}

.footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--s-2);
  min-height: 32px;
}

.status {
  display: inline-flex;
  align-items: center;
  gap: var(--s-1);
  font-size: var(--t-caption);
  line-height: var(--t-caption-lh);
  color: var(--c-on-surface-muted);
}

.status.error {
  color: var(--c-danger);
}

.save {
  display: inline-flex;
  align-items: center;
  gap: var(--s-1);
  padding: var(--s-1) var(--s-3);
  border-radius: var(--r-pill);
  background: var(--c-primary);
  color: var(--c-on-primary);
  font-size: var(--t-small);
  min-height: 32px;
}
</style>