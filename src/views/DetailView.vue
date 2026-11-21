<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { onBeforeRouteLeave, useRoute, useRouter } from "vue-router";
import Vditor from "vditor";
import {
  mdiArrowLeft,
  mdiClose,
  mdiPencil,
  mdiPin,
  mdiPinOutline,
  mdiTrashCanOutline,
} from "@mdi/js";
import {
  assetGc,
  describeError,
  IpcError,
  journalDelete,
  journalGet,
  journalSetPinned,
  journalSave,
} from "../ipc/client";
import type { Journal, Tag } from "../ipc/types";
import { t } from "../i18n";
import { useBreakpoint } from "../composables/useBreakpoint";
import { useAutosave } from "../composables/useAutosave";
import { useSession } from "../stores/session";
import { diaryToday, formatRecordedAt, recordedAtNow } from "../utils/date";
import { expandAssetUrls, isAssetSrc, referencedAssetNames } from "../utils/assetUrl";
import AssetViewer from "../components/AssetViewer.vue";
import EditorPane from "../components/EditorPane.vue";
import Icon from "../components/Icon.vue";
import ConfirmDialog from "../components/ConfirmDialog.vue";
import MetaChips from "../components/MetaChips.vue";
import StateBlock from "../components/StateBlock.vue";

const props = defineProps<{ entryId?: string | null }>();
const emit = defineEmits<{ close: [] }>();

const route = useRoute();
const router = useRouter();
const session = useSession();
const { isNarrow, isWide } = useBreakpoint();

const journal = ref<Journal | null>(null);
const content = ref("");
const loading = ref(true);
const missing = ref(false);
const failReason = ref<string | null>(null);
const saveError = ref<string | null>(null);
const editing = ref(false);
const previewEl = ref<HTMLElement | null>(null);
const editorPane = ref<InstanceType<typeof EditorPane> | null>(null);
const viewerSrc = ref<string | null>(null);

const embedded = computed(() => props.entryId != null);
const entryId = computed(() => props.entryId ?? String(route.params.id ?? ""));
const metaSaving = ref(false);
const deleteOpen = ref(false);
const pendingFocus = ref(false);
let skipLoadId = "";
let leaving = false;
const sessionAssets = new Set<string>();
let loadedAssets: string[] = [];

function onAssetsImported(names: string[]) {
  for (const name of names) sessionAssets.add(name);
}

function gcDetachedAssets() {
  if (readOnly.value) return;
  const keep = new Set(referencedAssetNames(content.value));
  const candidates = [...new Set([...loadedAssets, ...sessionAssets])].filter(
    (name) => !keep.has(name),
  );
  if (!candidates.length) return;
  void assetGc(candidates).catch((err) => {
    console.error("[assets] cleanup failed", err);
  });
}

const readOnly = computed(() => {
  const item = journal.value;
  if (!item) return false;
  return item.day_key < diaryToday(session.settings.dayBoundaryMinutes);
});

async function persist(value: string) {
  try {
    await writeJournal(value);
    saveError.value = null;
  } catch (err) {
    saveError.value = describeError(err);
    throw err;
  }
}

async function writeJournal(value: string) {
  if (readOnly.value) return;
  const item = journal.value;
  if (!item) {
    if (!value.trim()) return;
    const stamp = recordedAtNow();
    const saved = await journalSave({
      content: value,
      recordedAt: stamp.recordedAt,
      recordedOff: stamp.recordedOff,
    });
    journal.value = saved;
    skipLoadId = saved.id;
    if (!leaving && !embedded.value) {
      await router.replace({ path: `/j/${saved.id}`, query: { edit: "1" } });
    }
    return;
  }
  const saved = await journalSave({
    id: item.id,
    content: value,
    format: item.format,
    mood: item.mood,
    weather: item.weather,
    tagIds: item.tags.map((tag) => tag.id),
    recordedAt: item.recorded_at,
    recordedOff: item.recorded_off,
    pinned: item.pinned,
    sensitive: item.sensitive,
    revision: item.revision,
  });
  journal.value = saved;
}

const {
  state: saveState,
  schedule: scheduleSave,
  flushQuiet: flushSave,
} = useAutosave(() => content.value, persist);

async function updateTags(tags: Tag[]) {
  const item = journal.value;
  if (!item) return;
  item.tags = tags;
  await saveMeta();
}

async function updateMood(code: string | null) {
  const item = journal.value;
  if (!item) return;
  item.mood = code;
  await saveMeta();
}

async function updateWeather(code: string | null) {
  const item = journal.value;
  if (!item) return;
  item.weather = code;
  await saveMeta();
}

let metaQueued = false;

async function saveMeta() {
  if (metaSaving.value) {
    metaQueued = true;
    return;
  }
  metaSaving.value = true;
  try {
    do {
      metaQueued = false;
      await flushSave();
      await persist(content.value);
    } while (metaQueued);
  } catch {
    saveState.value = "error";
  } finally {
    metaSaving.value = false;
  }
}

const dateText = computed(() => {
  const boundary = session.settings.dayBoundaryMinutes;
  const item = journal.value;
  if (!item) {
    const stamp = recordedAtNow();
    return formatRecordedAt(
      stamp.recordedAt,
      stamp.recordedOff,
      session.settings.dateFormat,
      boundary,
    );
  }
  return formatRecordedAt(
    item.recorded_at,
    item.recorded_off,
    session.settings.dateFormat,
    boundary,
  );
});

function isDark(): boolean {
  return document.documentElement.dataset.theme === "dark";
}

function onPreviewClick(event: MouseEvent) {
  const target = event.target as HTMLElement | null;
  const image = target?.closest("img") as HTMLImageElement | null;
  if (image && isAssetSrc(image.src)) viewerSrc.value = image.src;
}

function renderPreview() {
  const element = previewEl.value;
  if (!element) return;
  element.innerHTML = "";
  if (!content.value.trim()) return;
  const dark = isDark();
  element.classList.add("vditor-reset");
  Vditor.preview(element as HTMLDivElement, expandAssetUrls(content.value), {
    cdn: "/vditor",
    mode: dark ? "dark" : "light",
    theme: { current: dark ? "dark" : "light", path: "/vditor/dist/css/content-theme" },
    hljs: { style: dark ? "native" : "github", lineNumber: false },
    anchor: 0,
    ...(session.settings.imageLazy ? { lazyLoadImage: "lazy" } : {}),
  });
  requestAnimationFrame(() => {
    applyHighlight(queryText());
  });
}

function queryText(): string {
  if (embedded.value) return "";
  const value = route.query.q;
  return typeof value === "string" ? value.trim() : "";
}

function applyHighlight(needleRaw: string) {
  const rootEl = previewEl.value;
  const needle = needleRaw.trim();
  if (!rootEl || !needle) return;
  const target = needle.toLowerCase();
  const walker = document.createTreeWalker(rootEl, NodeFilter.SHOW_TEXT, {
    acceptNode(node) {
      if (!node.nodeValue?.trim()) return NodeFilter.FILTER_REJECT;
      const parent = node.parentElement;
      if (!parent) return NodeFilter.FILTER_REJECT;
      if (parent.closest("pre, code, script, style, mark")) return NodeFilter.FILTER_REJECT;
      return NodeFilter.FILTER_ACCEPT;
    },
  });
  const nodes: Text[] = [];
  while (walker.nextNode()) nodes.push(walker.currentNode as Text);

  let first: HTMLElement | null = null;
  let count = 0;
  for (const node of nodes) {
    if (count >= 200) break;
    const value = node.nodeValue ?? "";
    const lower = value.toLowerCase();
    if (!lower.includes(target)) continue;
    const fragment = document.createDocumentFragment();
    let start = 0;
    let index = lower.indexOf(target);
    while (index !== -1 && count < 200) {
      if (index > start) fragment.append(value.slice(start, index));
      const mark = document.createElement("mark");
      mark.className = "dd-hit";
      mark.textContent = value.slice(index, index + target.length);
      fragment.append(mark);
      first ??= mark;
      count += 1;
      start = index + target.length;
      index = lower.indexOf(target, start);
    }
    if (start < value.length) fragment.append(value.slice(start));
    node.parentNode?.replaceChild(fragment, node);
  }
  first?.scrollIntoView({ block: "center" });
}

async function load() {
  const id = entryId.value;
  if (!id) return;
  if (id === "new") {
    journal.value = null;
    content.value = "";
    loadedAssets = [];
    missing.value = false;
    failReason.value = null;
    loading.value = false;
    editing.value = true;
    await nextTick();
    pendingFocus.value = true;
    editorPane.value?.focus();
    return;
  }
  loading.value = true;
  missing.value = false;
  failReason.value = null;
  try {
    const item = await journalGet(id);
    journal.value = item;
    content.value = item.content;
    loadedAssets = referencedAssetNames(item.content);
    const locked = item.day_key < diaryToday(session.settings.dayBoundaryMinutes);
    editing.value =
      !locked && (embedded.value || route.query.edit === "1" || !queryText());
    await nextTick();
    if (editing.value) {
      pendingFocus.value = true;
      editorPane.value?.focus();
    } else {
      renderPreview();
    }
  } catch (error) {
    if (error instanceof IpcError && error.code === "journal_not_found") {
      missing.value = true;
    } else {
      failReason.value = describeError(error);
    }
  } finally {
    loading.value = false;
  }
}

function onEditorReady() {
  if (!pendingFocus.value) return;
  pendingFocus.value = false;
  editorPane.value?.focus();
}

watch(
  entryId,
  async (next, previous) => {
    if (!next) return;
    if (skipLoadId && next === skipLoadId) {
      skipLoadId = "";
      return;
    }
    skipLoadId = "";
    if (previous) {
      await flushSave();
      gcDetachedAssets();
    }
    sessionAssets.clear();
    loadedAssets = [];
    content.value = "";
    await load();
  },
  { immediate: true },
);

function onEditorInput(value: string) {
  content.value = value;
  scheduleSave();
}

async function startEdit() {
  if (editing.value || !journal.value || readOnly.value) return;
  editing.value = true;
  pendingFocus.value = true;
  await nextTick();
  editorPane.value?.focus();
}

async function togglePin() {
  const item = journal.value;
  if (!item) return;
  const next = !item.pinned;
  await flushSave();
  await journalSetPinned(item.id, next);
  item.pinned = next;
}

function remove() {
  if (!journal.value) return;
  deleteOpen.value = true;
}

async function confirmRemove() {
  deleteOpen.value = false;
  const item = journal.value;
  if (!item) return;
  await flushSave();
  try {
    await journalDelete(item.id);
    goBack();
  } catch (err) {
    failReason.value = describeError(err);
  }
}

async function requestClose() {
  await flushSave();
  emit("close");
}

function goBack() {
  if (embedded.value) {
    void requestClose();
    return;
  }
  if (window.history.state?.back) router.back();
  else void router.push("/today");
}

function onWindowBlur() {
  void flushSave();
}

function onVisibility() {
  if (document.visibilityState === "hidden") void flushSave();
}

function onBeforeUnload() {
  void flushSave();
}

onMounted(() => {
  window.addEventListener("blur", onWindowBlur);
  window.addEventListener("beforeunload", onBeforeUnload);
  document.addEventListener("visibilitychange", onVisibility);
});

onBeforeRouteLeave(async () => {
  leaving = true;
  await flushSave();
});

onBeforeUnmount(() => {
  void (async () => {
    await flushSave();
    gcDetachedAssets();
  })();
  window.removeEventListener("blur", onWindowBlur);
  window.removeEventListener("beforeunload", onBeforeUnload);
  document.removeEventListener("visibilitychange", onVisibility);
});

watch(
  () => session.settings.theme,
  async () => {
    if (!editing.value) {
      await nextTick();
      renderPreview();
    }
  },
);
</script>

<template>
  <div class="view" :class="{ indent: session.settings.indentFirstLine }">
    <header class="topbar">
      <button
        class="icon-btn"
        type="button"
        :aria-label="embedded ? t('common.close') : t('common.back')"
        @click="goBack"
      >
        <Icon :path="embedded ? mdiClose : mdiArrowLeft" :size="22" />
      </button>

      <div class="heading">
        <span class="date">{{ dateText }}</span>
        <span v-if="journal" class="meta">
          {{ t("detail.words", { n: journal.words }) }}
          <template v-if="journal.pinned"> · {{ t("detail.pinned") }}</template>
          <template v-if="readOnly"> · {{ t("detail.readOnly") }}</template>
          <template v-if="saveState === 'saving'">
            · {{ t("detail.saving") }}
          </template>
          <button
            v-else-if="saveState === 'error'"
            class="retry-save"
            type="button"
            @click="flushSave"
          >
            · {{ t("detail.saveError") }} · {{ t("detail.retrySave") }}
          </button>
        </span>
      </div>

      <div class="actions">
        <button
          v-if="journal && !editing && !readOnly && isWide"
          class="icon-btn"
          type="button"
          :aria-label="t('common.edit')"
          @click="startEdit"
        >
          <Icon :path="mdiPencil" :size="20" />
        </button>
        <button
          v-if="journal"
          class="icon-btn"
          type="button"
          :aria-label="journal.pinned ? t('detail.unpin') : t('detail.pin')"
          :class="{ active: journal.pinned }"
          @click="togglePin"
        >
          <Icon :path="journal.pinned ? mdiPin : mdiPinOutline" :size="20" />
        </button>
        <button
          v-if="journal"
          class="icon-btn danger"
          type="button"
          :aria-label="t('common.delete')"
          @click="remove"
        >
          <Icon :path="mdiTrashCanOutline" :size="20" />
        </button>
      </div>
    </header>

    <p v-if="saveState === 'error' && saveError" class="save-detail">{{ saveError }}</p>

    <MetaChips
      v-if="journal && !loading"
      :tags="journal.tags"
      :mood="journal.mood"
      :weather="journal.weather"
      :editable="editing"
      @update:tags="updateTags"
      @update:mood="updateMood"
      @update:weather="updateWeather"
    />

    <div class="body">
      <StateBlock v-if="loading" kind="loading" />
      <StateBlock
        v-else-if="missing"
        kind="empty"
        :title="t('detail.notFound')"
        :action-label="t('common.back')"
        @action="goBack"
      />
      <StateBlock
        v-else-if="failReason"
        kind="error"
        :title="t('common.error')"
        :hint="failReason ?? undefined"
        :action-label="t('common.retry')"
        @action="load"
      />
      <template v-else>
        <Transition name="mode" mode="out-in">
          <div v-if="editing" key="edit" class="editor-wrap">
            <EditorPane
              ref="editorPane"
              :model-value="content"
              @update:model-value="onEditorInput"
              @ready="onEditorReady"
              @assets-imported="onAssetsImported"
            />
          </div>
          <div v-else-if="journal" key="preview" class="preview-wrap">
            <article ref="previewEl" class="preview" @click="onPreviewClick" @dblclick="startEdit" />
            <p v-if="readOnly" class="hint">
              {{ t("detail.readOnlyHint") }}
            </p>
            <p v-else-if="!content.trim()" class="hint">
              {{ t("detail.readHint") }}
            </p>
          </div>
        </Transition>
      </template>
    </div>

    <button
      v-if="journal && !editing && !readOnly && isNarrow"
      class="fab"
      type="button"
      @click="startEdit"
    >
      <Icon :path="mdiPencil" :size="18" />
      {{ t("common.edit") }}
    </button>

    <AssetViewer :src="viewerSrc" @close="viewerSrc = null" />

    <ConfirmDialog
      :open="deleteOpen"
      :title="t('detail.deleteTitle')"
      :message="t('detail.deleteConfirm')"
      :confirm-label="t('common.delete')"
      danger
      @confirm="confirmRemove"
      @cancel="deleteOpen = false"
    />
  </div>
</template>

<style scoped>
.view {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
  position: relative;
}

.topbar {
  display: flex;
  align-items: center;
  gap: var(--s-2);
  min-height: var(--topbar-h);
  padding: var(--s-2) var(--s-3);
  border-bottom: 1px solid var(--c-hairline);
  background: var(--c-surface);
}

.icon-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 44px;
  height: 44px;
  border-radius: var(--r-pill);
  color: var(--c-on-surface-muted);
  flex: none;
}

.icon-btn.active {
  color: var(--c-accent);
}

.icon-btn.danger {
  color: var(--c-danger);
}

@media (hover: hover) {
  .icon-btn:hover {
    background: var(--c-surface-2);
  }
}

.heading {
  flex: 1;
  min-width: 0;
  text-align: center;
}

.date {
  display: block;
  font-size: var(--t-small);
  line-height: var(--t-small-lh);
  color: var(--c-on-surface);
}

.meta {
  display: block;
  font-size: var(--t-caption);
  line-height: var(--t-caption-lh);
  color: var(--c-on-surface-muted);
}

.retry-save {
  color: var(--c-danger);
  font-size: var(--t-caption);
  text-decoration: underline;
}

.save-detail {
  padding: var(--s-2) var(--s-3);
  border-bottom: 1px solid var(--c-hairline);
  background: var(--c-surface-2);
  color: var(--c-danger);
  font-size: var(--t-caption);
  line-height: var(--t-caption-lh);
  white-space: pre-wrap;
  overflow-wrap: anywhere;
  user-select: text;
}

.actions {
  display: flex;
  align-items: center;
  gap: var(--s-1);
  flex: none;
}

.body {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  scrollbar-gutter: stable;
}

.editor-wrap {
  width: min(calc(100% - var(--s-4) * 2), calc(var(--measure) + var(--s-5) * 2));
  height: 100%;
  min-height: 0;
  margin-inline: auto;
  padding: var(--s-3) 0 var(--s-4);
}

.mode-enter-active,
.mode-leave-active {
  transition: opacity var(--m-1) var(--ease);
}

.mode-enter-from,
.mode-leave-to {
  opacity: 0;
}

.preview {
  width: min(calc(100% - var(--s-4) * 2), calc(var(--measure) + var(--s-5) * 2));
  max-width: none;
  margin: var(--s-3) auto var(--s-6);
  padding: var(--s-5);
  min-height: 200px;
  background: var(--c-surface-1);
  border: 1px solid var(--c-hairline);
  border-radius: var(--r-md);
}

.preview :deep(.dd-hit) {
  background: var(--c-hit);
  color: inherit;
  border-radius: 2px;
}

.hint {
  padding: var(--s-4);
  font-size: var(--t-small);
  line-height: var(--t-small-lh);
  color: var(--c-on-surface-muted);
}

.indent :deep(.preview p) {
  text-indent: 2em;
}

.indent :deep(.vditor-reset p) {
  text-indent: 2em;
}

.fab {
  position: fixed;
  inset-inline-end: var(--s-4);
  bottom: calc(var(--nav-h) + var(--safe-bottom) + var(--s-4));
  display: inline-flex;
  align-items: center;
  gap: var(--s-1);
  height: 44px;
  padding-inline: var(--s-4);
  border-radius: var(--r-pill);
  background: var(--c-primary);
  color: var(--c-on-primary);
  box-shadow: var(--e-2);
  font-size: var(--t-small);
  transition: transform var(--m-1) var(--ease);
}

.fab:active {
  transform: scale(0.98);
}

@media (min-width: 960px) {
  .fab {
    bottom: var(--s-6);
  }
}

.not-found {
  padding: var(--s-6);
}
</style>