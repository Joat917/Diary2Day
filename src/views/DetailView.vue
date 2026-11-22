<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { onBeforeRouteLeave, useRoute, useRouter } from "vue-router";
import {
  mdiArrowLeft,
  mdiClose,
  mdiImageOutline,
  mdiPencil,
  mdiPin,
  mdiPinOutline,
  mdiTrashCanOutline,
} from "@mdi/js";
import { open as openDialog, save as saveDialog } from "@tauri-apps/plugin-dialog";
import {
  assetGc,
  describeError,
  imageExport,
  imageExportAndroid,
  imagesExportAndroid,
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
import { isAssetSrc, referencedAssetNames } from "../utils/assetUrl";
import { renderNotePreview } from "../utils/notePreview";
import {
  capturePng,
  inlineImages,
  prepareExportImages,
  replaceMediaElements,
  splitExportChunks,
  waitForStable,
} from "../utils/noteImage";
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
const draftTags = ref<Tag[]>([]);
const draftMood = ref<string | null>(null);
const draftWeather = ref<string | null>(null);
const loading = ref(true);
const missing = ref(false);
const failReason = ref<string | null>(null);
const saveError = ref<string | null>(null);
const editing = ref(false);
const previewEl = ref<HTMLElement | null>(null);
const editorWrapEl = ref<HTMLElement | null>(null);
const editorPane = ref<InstanceType<typeof EditorPane> | null>(null);
const viewerSrc = ref<string | null>(null);
const exportStageEl = ref<HTMLElement | null>(null);
const exportCardEl = ref<HTMLElement | null>(null);
const exportArticleEl = ref<HTMLElement | null>(null);
const exporting = ref(false);
const exportError = ref<string | null>(null);
const exportNotice = ref<string | null>(null);
const mobilePlatform = /Android|iPhone|iPad|iPod/i.test(navigator.userAgent);

const embedded = computed(() => props.entryId != null);
const entryId = computed(() => props.entryId ?? String(route.params.id ?? ""));
const isDraft = computed(
  () => !loading.value && !journal.value && entryId.value === "new",
);
const showMeta = computed(() => !loading.value && (!!journal.value || isDraft.value));
const metaTags = computed(() => (journal.value ? journal.value.tags : draftTags.value));
const metaMood = computed(() => (journal.value ? journal.value.mood : draftMood.value));
const metaWeather = computed(() =>
  journal.value ? journal.value.weather : draftWeather.value,
);
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

const canExport = computed(
  () => !!journal.value && !editing.value && content.value.trim() !== "",
);

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
      mood: draftMood.value,
      weather: draftWeather.value,
      tagIds: draftTags.value.map((tag) => tag.id),
      recordedAt: stamp.recordedAt,
      recordedOff: stamp.recordedOff,
    });
    journal.value = saved;
    draftTags.value = [];
    draftMood.value = null;
    draftWeather.value = null;
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
  if (!item) {
    draftTags.value = tags;
    return;
  }
  item.tags = tags;
  await saveMeta();
}

async function updateMood(code: string | null) {
  const item = journal.value;
  if (!item) {
    draftMood.value = code;
    return;
  }
  item.mood = code;
  await saveMeta();
}

async function updateWeather(code: string | null) {
  const item = journal.value;
  if (!item) {
    draftWeather.value = code;
    return;
  }
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
  return document.documentElement.dataset.scheme === "dark";
}

function onPreviewClick(event: MouseEvent) {
  const target = event.target as HTMLElement | null;
  const image = target?.closest("img") as HTMLImageElement | null;
  if (image && isAssetSrc(image.src)) viewerSrc.value = image.src;
}

function renderPreview() {
  const element = previewEl.value;
  if (!element) return;
  void renderNotePreview(element, content.value, {
    dark: isDark(),
    lazyImages: session.settings.imageLazy,
  })
    .then(() => {
      requestAnimationFrame(() => applyHighlight(queryText()));
    })
    .catch((err) => {
      console.error("[preview] render failed", err);
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
    draftTags.value = [];
    draftMood.value = null;
    draftWeather.value = null;
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
  exportError.value = null;
  exportNotice.value = null;
  draftTags.value = [];
  draftMood.value = null;
  draftWeather.value = null;
  try {
    const item = await journalGet(id);
    journal.value = item;
    content.value = item.content;
    loadedAssets = referencedAssetNames(item.content);
    const locked = item.day_key < diaryToday(session.settings.dayBoundaryMinutes);
    editing.value =
      !locked &&
      (route.query.edit === "1" ||
        (session.settings.autoEdit && !queryText()));
    await nextTick();
    if (editing.value) {
      pendingFocus.value = true;
      editorPane.value?.focus();
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
      await gcAfterConfirmedSave();
    }
    sessionAssets.clear();
    loadedAssets = [];
    content.value = "";
    await load();
  },
  { immediate: true },
);

// The mode transition is out-in, so the preview mounts after the editor leaves;
// rendering here instead of right after `editing` flips avoids an empty card.
watch(previewEl, (element) => {
  if (element && !editing.value) renderPreview();
});

function onEditorInput(value: string) {
  content.value = value;
  scheduleSave();
}

async function startEdit() {
  if (editing.value || !journal.value || readOnly.value) return;
  exportError.value = null;
  exportNotice.value = null;
  editing.value = true;
  pendingFocus.value = true;
  await nextTick();
  editorPane.value?.focus();
}

// Rust deletes whatever names it is handed, so running GC after a failed save would
// drop files the stored document still references. Flush first and only collect once
// the document in the database matches `content`.
async function gcAfterConfirmedSave() {
  await flushSave();
  if (saveState.value === "error") {
    console.warn(
      "[assets] cleanup skipped: content was not saved",
      saveError.value ?? "unknown error",
    );
    return;
  }
  gcDetachedAssets();
}

async function leaveEdit() {
  if (!editing.value) return;
  editing.value = false;
  await gcAfterConfirmedSave();
}

function surfaceColor(): string {
  const value = getComputedStyle(document.documentElement)
    .getPropertyValue("--c-surface")
    .trim();
  return value || (isDark() ? "black" : "white");
}

function exportFileBase(): string {
  const key =
    journal.value?.day_key ?? diaryToday(session.settings.dayBoundaryMinutes);
  return `Diary2Day-${key}`;
}

function exportFileName(): string {
  return `${exportFileBase()}.png`;
}

function dataOf(dataUrl: string): string {
  const data = dataUrl.slice(dataUrl.indexOf(",") + 1);
  if (!data) throw new IpcError("export_empty", "生成的图片为空");
  return data;
}

async function savePng(dataUrl: string, fileName: string): Promise<string | null> {
  const data = dataOf(dataUrl);
  if (mobilePlatform) return imageExportAndroid(fileName, data);
  const target = await saveDialog({
    title: t("detail.exportImage"),
    defaultPath: fileName,
    filters: [{ name: "PNG", extensions: ["png"] }],
  });
  if (!target) return null;
  await imageExport(target, data);
  return target.split(/[\\/]/).pop() ?? target;
}

async function savePngSet(images: string[], base: string): Promise<string | null> {
  const width = Math.max(2, String(images.length).length);
  const entries = images.map((dataUrl, index) => ({
    name: `${base}-${String(index + 1).padStart(width, "0")}.png`,
    data: dataOf(dataUrl),
  }));
  if (mobilePlatform) {
    return imagesExportAndroid(
      entries.map((entry) => entry.name),
      entries.map((entry) => entry.data),
    );
  }
  const picked = await openDialog({
    title: t("detail.exportImage"),
    directory: true,
    multiple: false,
  });
  if (!picked) return null;
  const dir = picked.replace(/[\\/]+$/, "");
  const sep = dir.includes("\\") ? "\\" : "/";
  for (const entry of entries) {
    await imageExport(`${dir}${sep}${entry.name}`, entry.data);
  }
  return dir.split(/[\\/]/).filter(Boolean).pop() ?? dir;
}

async function exportImage() {
  if (!canExport.value || exporting.value) return;
  exporting.value = true;
  exportError.value = null;
  exportNotice.value = null;
  try {
    await nextTick();
    const stage = exportStageEl.value;
    const card = exportCardEl.value;
    const article = exportArticleEl.value;
    if (!stage || !card || !article) {
      throw new IpcError("export_empty", "导出区域未就绪");
    }
    await renderNotePreview(article, content.value, {
      dark: isDark(),
      lazyImages: false,
    });
    await waitForStable(card);
    await replaceMediaElements(article, {
      audio: t("detail.exportAudio"),
      video: t("detail.exportVideo"),
      embed: t("detail.exportEmbed"),
    });
    await prepareExportImages(article);
    await inlineImages(article);
    const chunks = splitExportChunks(card, stage, (index, total) =>
      `${dateText.value} · ${t("detail.exportChunk", { index, total })}`,
    );
    const images: string[] = [];
    for (const chunk of chunks) {
      images.push(await capturePng(chunk, surfaceColor()));
    }
    // 探测音视频与逐张出图会拉长整个流程；页面若已离开，别再弹保存框。
    if (!chunks[0].isConnected) return;
    const saved =
      images.length === 1
        ? await savePng(images[0], exportFileName())
        : await savePngSet(images, exportFileBase());
    if (saved) {
      exportNotice.value =
        images.length === 1
          ? t("detail.exportImageDone", { name: saved })
          : t("detail.exportImagesDone", {
              count: images.length,
              name: saved,
            });
    }
  } catch (err) {
    console.error(`[export] image failed: ${describeError(err)}`, err);
    exportError.value = describeError(err);
  } finally {
    exporting.value = false;
  }
}

function onDocumentPointerDown(event: PointerEvent) {
  if (!editing.value || !journal.value) return;
  const target = event.target;
  if (!(target instanceof Element)) return;
  if (editorWrapEl.value?.contains(target)) return;
  if (target.closest(".meta-chips, [role='dialog'], [role='alertdialog']")) return;
  void leaveEdit();
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
  document.addEventListener("pointerdown", onDocumentPointerDown, true);
});

onBeforeRouteLeave(async () => {
  leaving = true;
  await flushSave();
});

onBeforeUnmount(() => {
  void gcAfterConfirmedSave();
  window.removeEventListener("blur", onWindowBlur);
  window.removeEventListener("beforeunload", onBeforeUnload);
  document.removeEventListener("visibilitychange", onVisibility);
  document.removeEventListener("pointerdown", onDocumentPointerDown, true);
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
  <div class="view" :class="{ indent: session.settings.indentFirstLine, wide: isWide }">
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
          v-if="canExport"
          class="icon-btn"
          type="button"
          :aria-label="t('detail.exportImage')"
          :disabled="exporting"
          @click="exportImage"
        >
          <Icon :path="mdiImageOutline" :size="20" />
        </button>
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
    <p v-if="exporting" class="export-note">{{ t("detail.exportImagePending") }}</p>
    <p v-else-if="exportError" class="save-detail">{{ exportError }}</p>
    <p v-else-if="exportNotice" class="export-note">{{ exportNotice }}</p>

    <MetaChips
      v-if="showMeta"
      :tags="metaTags"
      :mood="metaMood"
      :weather="metaWeather"
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
          <div v-if="editing" key="edit" ref="editorWrapEl" class="editor-wrap">
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

    <div v-if="exporting" ref="exportStageEl" class="export-stage" aria-hidden="true">
      <div ref="exportCardEl" class="export-card">
        <p class="export-date">{{ dateText }}</p>
        <article ref="exportArticleEl" class="preview" />
      </div>
    </div>

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
  .icon-btn:hover:not(:disabled) {
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

/* 离屏出图：固定在视口外，宽度固定，不参与滚动与交互。 */
.export-stage {
  position: fixed;
  inset-block-start: 0;
  inset-inline-start: -10000px;
  width: 640px;
  pointer-events: none;
}

.export-card {
  width: 640px;
  padding: var(--s-6);
  background: var(--c-surface);
  color: var(--c-on-surface);
}

.export-date {
  margin: 0 0 var(--s-4);
  font-size: var(--t-small);
  line-height: var(--t-small-lh);
  color: var(--c-on-surface-muted);
}

.export-card .preview {
  width: 100%;
  max-width: none;
  min-height: 0;
  margin: 0;
  padding: 0;
  background: transparent;
  border: 0;
  border-radius: 0;
}

.export-card :deep(.export-media) {
  margin: var(--s-3) 0;
  border: 1px solid var(--c-hairline);
  border-radius: var(--r-sm);
  overflow: hidden;
  background: var(--c-surface-2);
  color: var(--c-on-surface-muted);
  font-size: var(--t-small);
  line-height: var(--t-small-lh);
}

.export-card :deep(.export-embed) {
  padding: var(--s-2) var(--s-3);
}

.export-card :deep(.export-media__caption) {
  padding: var(--s-1) var(--s-3);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.export-card :deep(.export-media__frame) {
  position: relative;
  height: 360px;
  background: var(--c-surface-1);
}

.export-card :deep(.export-media__cover) {
  display: block;
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.export-card :deep(.export-media__play) {
  position: absolute;
  inset-block-start: 50%;
  inset-inline-start: 50%;
  transform: translate(-50%, -50%);
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 52px;
  height: 52px;
  border-radius: 50%;
  background: var(--c-on-surface);
  color: var(--c-surface);
}

.export-card :deep(.export-media__play svg) {
  width: 26px;
  height: 26px;
  fill: currentColor;
}

.export-card :deep(.export-media__duration) {
  position: absolute;
  inset-block-end: var(--s-2);
  inset-inline-end: var(--s-2);
  padding: 1px var(--s-1);
  border-radius: var(--r-pill);
  background: var(--c-on-surface);
  color: var(--c-surface);
  font-size: var(--t-caption);
  line-height: var(--t-caption-lh);
  font-variant-numeric: tabular-nums;
}

.export-card :deep(.export-audio) {
  padding-block-end: var(--s-2);
}

.export-card :deep(.export-audio .export-media__caption) {
  padding-block-end: var(--s-1);
}

.export-card :deep(.export-audio__row) {
  display: flex;
  align-items: center;
  gap: var(--s-2);
  padding-inline: var(--s-3);
}

.export-card :deep(.export-audio__bar) {
  position: relative;
  flex: 1;
  height: 4px;
  border-radius: var(--r-pill);
  background: var(--c-surface-1);
}

.export-card :deep(.export-audio__knob) {
  position: absolute;
  inset-block-start: 50%;
  inset-inline-start: 0;
  width: 10px;
  height: 10px;
  margin-block-start: -5px;
  border-radius: 50%;
  background: var(--c-primary);
}

.export-card :deep(.export-audio__time) {
  flex: none;
  font-size: var(--t-caption);
  line-height: var(--t-caption-lh);
  font-variant-numeric: tabular-nums;
}

.export-note {
  padding: var(--s-2) var(--s-3);
  border-bottom: 1px solid var(--c-hairline);
  background: var(--c-surface-2);
  color: var(--c-on-surface-muted);
  font-size: var(--t-caption);
  line-height: var(--t-caption-lh);
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
  inset-inline-end: calc(var(--s-4) + var(--safe-right));
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

.view.wide .fab {
  bottom: var(--s-6);
}

.not-found {
  padding: var(--s-6);
}
</style>