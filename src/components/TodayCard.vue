<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref } from "vue";
import { mdiCheck, mdiClose, mdiImagePlus, mdiSend } from "@mdi/js";
import { assetGc, describeError, journalSave } from "../ipc/client";
import type { Asset, Journal } from "../ipc/types";
import { t } from "../i18n";
import { assetFileUrl, assetMarkdown, referencedAssetNames, thumbUrl } from "../utils/assetUrl";
import { recordedAtNow } from "../utils/date";
import { importMediaFiles, insertLabelKey, isMediaFile } from "../utils/media";
import Icon from "./Icon.vue";

const emit = defineEmits<{ saved: [journal: Journal] }>();

const text = ref("");
const attachments = ref<Asset[]>([]);
const textarea = ref<HTMLTextAreaElement | null>(null);
const fileInput = ref<HTMLInputElement | null>(null);
const status = ref<"idle" | "saving" | "saved" | "error">("idle");
const dragging = ref(false);
const importing = ref(false);
const importLabel = ref("");
const importError = ref<string | null>(null);
let resetTimer: number | undefined;

const wordCount = computed(() => text.value.trim().length);

const content = computed(() => {
  const parts: string[] = [];
  if (text.value.trim()) parts.push(text.value.trimEnd());
  for (const asset of attachments.value) {
    parts.push(assetMarkdown(asset.name, asset.kind));
  }
  return parts.join("\n\n");
});

const canSave = computed(
  () =>
    (text.value.trim().length > 0 || attachments.value.length > 0) &&
    status.value !== "saving" &&
    !importing.value,
);

function resize() {
  const element = textarea.value;
  if (!element) return;
  element.style.height = "auto";
  const line = 26;
  element.style.height = `${Math.min(Math.max(element.scrollHeight, line), line * 6)}px`;
}

async function ingest(files: File[]) {
  if (importing.value) return;
  const media = files.filter(isMediaFile);
  if (!media.length) return;
  importing.value = true;
  importError.value = null;
  importLabel.value = t(insertLabelKey(media));
  try {
    const assets = await importMediaFiles(media);
    attachments.value = [...attachments.value, ...assets];
  } catch (err) {
    importError.value = describeError(err);
  } finally {
    importing.value = false;
    importLabel.value = "";
  }
}

function onPick(event: Event) {
  const input = event.target as HTMLInputElement;
  const files = Array.from(input.files ?? []);
  input.value = "";
  if (files.length) void ingest(files);
}

function onPaste(event: ClipboardEvent) {
  const files = Array.from(event.clipboardData?.files ?? []);
  if (!files.some(isMediaFile)) return;
  event.preventDefault();
  void ingest(files);
}

function onDragOver(event: DragEvent) {
  if (!event.dataTransfer?.types.includes("Files")) return;
  event.preventDefault();
  dragging.value = true;
}

function onDrop(event: DragEvent) {
  const files = Array.from(event.dataTransfer?.files ?? []);
  dragging.value = false;
  if (!files.some(isMediaFile)) return;
  event.preventDefault();
  void ingest(files);
}

function removeAttachment(name: string) {
  attachments.value = attachments.value.filter((asset) => asset.name !== name);
  // An in-flight save may already reference this asset.
  if (status.value === "saving") return;
  if (referencedAssetNames(content.value).includes(name)) return;
  void assetGc([name]).catch((err) => {
    console.error("[assets] cleanup failed", err);
  });
}

function onThumbError(event: Event, name: string) {
  const image = event.target as HTMLImageElement;
  const fallback = assetFileUrl(name);
  if (fallback && image.getAttribute("src") !== fallback) image.src = fallback;
}

let saveTask: Promise<void> | null = null;

function save(): Promise<void> {
  if (!canSave.value) return Promise.resolve();
  status.value = "saving";
  const task = runSave();
  saveTask = task;
  void task.finally(() => {
    if (saveTask === task) saveTask = null;
  });
  return task;
}

async function runSave() {
  try {
    const stamp = recordedAtNow();
    const journal = await journalSave({
      content: content.value,
      recordedAt: stamp.recordedAt,
      recordedOff: stamp.recordedOff,
    });
    text.value = "";
    attachments.value = [];
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

onBeforeUnmount(() => {
  window.clearTimeout(resetTimer);
  const pending = saveTask;
  void (async () => {
    if (pending) await pending;
    const keep = new Set(referencedAssetNames(content.value));
    const names = attachments.value.map((asset) => asset.name).filter((name) => !keep.has(name));
    if (names.length) {
      await assetGc(names).catch((err) => {
        console.error("[assets] cleanup failed", err);
      });
    }
  })();
});
</script>

<template>
  <section
    class="card"
    :class="{ dragging }"
    @dragover="onDragOver"
    @dragleave.self="dragging = false"
    @drop="onDrop"
  >
    <textarea
      ref="textarea"
      v-model="text"
      class="input"
      rows="1"
      :placeholder="t('today.placeholder')"
      @input="resize"
      @paste="onPaste"
      @blur="save"
    />

    <div v-if="attachments.length" class="attachments">
      <div v-for="asset in attachments" :key="asset.name" class="attachment">
        <img
          v-if="asset.kind === 'image'"
          :src="thumbUrl(asset.name) ?? ''"
          :alt="asset.name"
          loading="lazy"
          @error="onThumbError($event, asset.name)"
        />
        <span v-else class="attachment-label">{{ asset.kind }}</span>
        <button
          class="attachment-remove"
          type="button"
          :aria-label="t('common.close')"
          @mousedown.prevent
          @click="removeAttachment(asset.name)"
        >
          <Icon :path="mdiClose" :size="12" />
        </button>
      </div>
    </div>

    <div class="footer">
      <div class="footer-left">
        <button
          class="attach"
          type="button"
          :aria-label="t('editor.image')"
          :disabled="importing"
          @mousedown.prevent
          @click="fileInput?.click()"
        >
          <Icon :path="mdiImagePlus" :size="16" />
        </button>
        <span class="status" :class="status">
          <template v-if="importing">{{ importLabel }}</template>
          <template v-else-if="importError">
            <span class="import-error" :title="importError">{{ importError }}</span>
          </template>
          <template v-else-if="status === 'saving'">{{ t('detail.saving') }}</template>
          <template v-else-if="status === 'saved'">
            <Icon :path="mdiCheck" :size="14" />
            {{ t('today.saved') }}
          </template>
          <template v-else-if="status === 'error'">
            {{ t('detail.saveError') }} · {{ t('common.retry') }}
          </template>
          <template v-else>
            {{
              wordCount || attachments.length
                ? t("today.words", { n: wordCount })
                : t("today.saveHint")
            }}
          </template>
        </span>
      </div>
      <button
        v-if="text.trim() || attachments.length"
        class="save"
        type="button"
        :disabled="status === 'saving' || importing"
        @mousedown.prevent
        @click="save"
      >
        <Icon :path="mdiSend" :size="16" />
        {{ t("today.save") }}
      </button>
    </div>

    <input
      ref="fileInput"
      class="file-input"
      type="file"
      accept="image/*,audio/*,video/*"
      multiple
      @change="onPick"
    />
    <div v-if="dragging" class="drop-hint">{{ t("editor.dropHint") }}</div>
  </section>
</template>

<style scoped>
.card {
  position: relative;
  margin: var(--s-4);
  padding: var(--s-3) var(--s-4) var(--s-2);
  border-radius: var(--r-md);
  background: var(--c-surface-1);
  border: 1px solid var(--c-hairline);
  transition: border-color var(--m-1) var(--ease);
}

.card:focus-within,
.card.dragging {
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

.attachments {
  display: flex;
  flex-wrap: wrap;
  gap: var(--s-2);
  padding-block: var(--s-1);
}

.attachment {
  position: relative;
  width: 64px;
  height: 64px;
  border-radius: var(--r-sm);
  overflow: hidden;
  background: var(--c-surface-2);
}

.attachment img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.attachment-label {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100%;
  color: var(--c-on-surface-muted);
  font-size: var(--t-caption);
}

.attachment-remove {
  position: absolute;
  top: 2px;
  inset-inline-end: 2px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 20px;
  border-radius: var(--r-pill);
  background: var(--c-scrim);
  color: #fff;
}

.footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--s-2);
  min-height: 32px;
}

.footer-left {
  display: flex;
  align-items: center;
  gap: var(--s-1);
  min-width: 0;
}

.attach {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border-radius: var(--r-pill);
  color: var(--c-on-surface-muted);
  flex: none;
}

@media (hover: hover) {
  .attach:hover:not(:disabled) {
    background: var(--c-surface-2);
  }
}

.status {
  display: inline-flex;
  align-items: center;
  gap: var(--s-1);
  font-size: var(--t-caption);
  line-height: var(--t-caption-lh);
  color: var(--c-on-surface-muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.status.error {
  color: var(--c-danger);
}

.import-error {
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
  flex: none;
}

.file-input {
  display: none;
}

.drop-hint {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--r-md);
  background: color-mix(in srgb, var(--c-primary-soft) 80%, transparent);
  color: var(--c-primary);
  font-size: var(--t-small);
  pointer-events: none;
}
</style>