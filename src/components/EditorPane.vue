<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from "vue";
import Vditor from "vditor";
import { mdiImagePlus } from "@mdi/js";
import { describeError } from "../ipc/client";
import { t } from "../i18n";
import type { MessageKey } from "../i18n";
import type { Asset } from "../ipc/types";
import {
  assetMarkdown,
  collapseAssetUrls,
  expandAssetUrls,
} from "../utils/assetUrl";
import { registerFileDrop } from "../utils/fileDrop";
import {
  importMediaPaths,
  insertLabelKey,
  isMediaFile,
  mediaKindOfPath,
  pasteMedia,
  pickMedia,
} from "../utils/media";

const props = defineProps<{ modelValue: string }>();
const emit = defineEmits<{
  "update:modelValue": [value: string];
  ready: [];
  "assets-imported": [names: string[]];
}>();

const root = ref<HTMLDivElement | null>(null);
const host = ref<HTMLDivElement | null>(null);
const dragging = ref(false);
const busy = ref(false);
const insertLabel = ref("");
const insertError = ref<string | null>(null);
let editor: Vditor | null = null;
let lastValue = props.modelValue;

function syncFromEditor() {
  if (!editor) return;
  const value = collapseAssetUrls(editor.getValue());
  lastValue = value;
  emit("update:modelValue", value);
}

watch(
  () => props.modelValue,
  (value) => {
    if (!editor) return;
    if (value === lastValue || value === collapseAssetUrls(editor.getValue())) return;
    lastValue = value;
    editor.setValue(expandAssetUrls(value), true);
  },
);

function patchToolbar(element: HTMLElement) {
  const toolbar = element.querySelector(".vditor-toolbar");
  if (!(toolbar instanceof HTMLElement)) return;
  toolbar.onmousedown = (event) => {
    if ((event.target as HTMLElement).closest(".keep-focus")) return;
    event.preventDefault();
  };
}

async function ingest(load: () => Promise<Asset[]>, labelKey: MessageKey) {
  if (busy.value) return;
  busy.value = true;
  insertError.value = null;
  insertLabel.value = t(labelKey);
  try {
    const assets = await load();
    if (!assets.length) return;
    emit(
      "assets-imported",
      assets.map((asset) => asset.name),
    );
    for (const asset of assets) {
      const markdown = assetMarkdown(asset.name, asset.kind);
      editor?.insertValue(`${expandAssetUrls(markdown)}\n`, true);
    }
    syncFromEditor();
  } catch (err) {
    insertError.value = describeError(err);
  } finally {
    busy.value = false;
    insertLabel.value = "";
  }
}

function pickImage() {
  void ingest(pickMedia, "editor.insertingMedia");
}

function onPaste(event: ClipboardEvent) {
  const files = Array.from(event.clipboardData?.files ?? []);
  if (!files.some(isMediaFile)) return;
  event.preventDefault();
  event.stopPropagation();
  void ingest(pasteMedia, "editor.insertingMedia");
}

function onDragEnter(paths: string[]) {
  if (paths.some((path) => mediaKindOfPath(path))) dragging.value = true;
}

function onDrop(paths: string[]) {
  dragging.value = false;
  const media = paths.filter((path) => mediaKindOfPath(path));
  if (!media.length) return;
  const kinds = media.map((path) => mediaKindOfPath(path)!);
  void ingest(() => importMediaPaths(media), insertLabelKey(kinds));
}

let unregisterDrop: (() => void) | null = null;

onMounted(() => {
  if (!root.value || !host.value) return;
  root.value.addEventListener("paste", onPaste, true);
  unregisterDrop = registerFileDrop(root.value, {
    enter: onDragEnter,
    leave: () => {
      dragging.value = false;
    },
    drop: onDrop,
  });
  editor = new Vditor(host.value, {
    mode: "ir",
    cdn: "/vditor",
    value: expandAssetUrls(props.modelValue),
    height: "100%",
    cache: { enable: false },
    placeholder: t("today.placeholder"),
    theme: document.documentElement.dataset.scheme === "dark" ? "dark" : "classic",
    toolbar: [
      {
        name: "dd-image",
        icon: `<svg viewBox="0 0 24 24"><path fill="currentColor" d="${mdiImagePlus}"/></svg>`,
        tip: t("editor.image"),
        tipPosition: "s",
        click: pickImage,
      },
      "headings",
      "bold",
      "italic",
      "strike",
      "|",
      "list",
      "ordered-list",
      "check",
      "|",
      "quote",
      "code",
      "inline-code",
      "link",
      "table",
      "|",
      "undo",
      "redo",
    ],
    toolbarConfig: { hide: false, pin: true },
    counter: { enable: false },
    resize: { enable: false },
    input: (value) => {
      lastValue = collapseAssetUrls(value);
      emit("update:modelValue", lastValue);
    },
    after: () => {
      if (host.value) patchToolbar(host.value);
      emit("ready");
    },
  });
});

onBeforeUnmount(() => {
  root.value?.removeEventListener("paste", onPaste, true);
  unregisterDrop?.();
  unregisterDrop = null;
  editor?.destroy();
  editor = null;
});

defineExpose({
  focus: () => editor?.focus(),
});
</script>

<template>
  <div ref="root" class="editor" :class="{ dragging }">
    <!-- Vditor clears its host element on init, so it gets a dedicated child. -->
    <div ref="host" class="editor-body" />
    <div v-if="dragging" class="drop-hint">{{ t("editor.dropHint") }}</div>
    <div v-if="busy" class="insert-hint" role="status" aria-live="polite">
      <span class="spinner" aria-hidden="true" />
      <span>{{ insertLabel }}</span>
    </div>
    <button
      v-else-if="insertError"
      class="insert-hint failed"
      type="button"
      :title="insertError"
      @click="insertError = null"
    >
      <span class="insert-error-text">{{ insertError }}</span>
    </button>
  </div>
</template>

<style scoped>
.editor {
  position: relative;
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
  background: var(--c-surface-1);
  border: 1px solid var(--c-hairline);
  border-radius: var(--r-md);
  overflow: hidden;
}

.editor.dragging {
  border-color: var(--c-primary);
}

.editor-body {
  flex: 1;
  min-height: 0;
}

.drop-hint {
  position: absolute;
  inset: 0;
  z-index: 5;
  display: flex;
  align-items: center;
  justify-content: center;
  background: color-mix(in srgb, var(--c-primary-soft) 80%, transparent);
  color: var(--c-primary);
  font-size: var(--t-small);
  pointer-events: none;
}

.insert-hint {
  position: absolute;
  inset-inline: 0;
  bottom: var(--s-3);
  z-index: 6;
  display: flex;
  align-items: center;
  gap: var(--s-2);
  width: fit-content;
  max-width: calc(100% - var(--s-6));
  margin-inline: auto;
  padding: var(--s-2) var(--s-3);
  border: 1px solid var(--c-hairline);
  border-radius: var(--r-pill);
  background: var(--c-surface-1);
  box-shadow: var(--e-1);
  color: var(--c-on-surface-muted);
  font-size: var(--t-small);
  line-height: var(--t-small-lh);
  pointer-events: none;
}

.insert-hint.failed {
  color: var(--c-danger);
  pointer-events: auto;
  text-align: start;
}

.insert-error-text {
  max-height: 4.5em;
  overflow: auto;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
  user-select: text;
}

.spinner {
  flex: none;
  width: 14px;
  height: 14px;
  border: 2px solid var(--c-hairline);
  border-top-color: var(--c-primary);
  border-radius: var(--r-pill);
  animation: insert-spin 0.8s linear infinite;
}

@keyframes insert-spin {
  to {
    transform: rotate(360deg);
  }
}

.editor :deep(.vditor) {
  height: 100%;
  min-height: 0;
}

.editor :deep(.vditor-content) {
  height: auto;
  flex: 1;
  min-height: 0;
}

.editor :deep(.vditor-ir) {
  height: 100%;
  overflow-y: auto;
  padding: var(--s-3) var(--s-4) var(--s-8);
}
</style>