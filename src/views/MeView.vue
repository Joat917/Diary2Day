<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { open as openDialog, save as saveDialog } from "@tauri-apps/plugin-dialog";
import {
  mdiChevronDown,
  mdiCloudDownloadOutline,
  mdiCloudUploadOutline,
  mdiFileExportOutline,
  mdiFileImportOutline,
  mdiFolderOpenOutline,
  mdiRestore,
  mdiTrashCanOutline,
} from "@mdi/js";
import {
  dataBackup,
  dataExportDir,
  dataImportMd,
  dataInfo,
  dataRestore,
  describeError,
  journalEmptyTrash,
  journalPurge,
  journalRestore,
  journalTrashList,
  onDataChanged,
  onOpProgress,
  openDataDir,
} from "../ipc/client";
import type { DataInfo, JournalSummary, OpProgress, Theme } from "../ipc/types";
import { t, type MessageKey } from "../i18n";
import { useLock } from "../stores/lock";
import { useSession } from "../stores/session";
import {
  formatBoundary,
  formatBytes,
  formatCardDate,
  parseBoundary,
} from "../utils/date";
import CodeDialog from "../components/CodeDialog.vue";
import ConfirmDialog from "../components/ConfirmDialog.vue";
import Icon from "../components/Icon.vue";
import StateBlock from "../components/StateBlock.vue";

const session = useSession();
const lock = useLock();
const info = ref<DataInfo | null>(null);
const trash = ref<JournalSummary[]>([]);
const trashLoading = ref(true);
const trashError = ref<string | null>(null);
const failure = ref<string | null>(null);

const themes: Theme[] = ["system", "light", "dark"];

const weekdayOptions = computed(() =>
  Array.from({ length: 7 }, (_, index) => ({
    value: index,
    label: t(`weekday.${index}` as MessageKey),
  })),
);

async function loadInfo() {
  try {
    info.value = await dataInfo();
    failure.value = null;
  } catch (err) {
    failure.value = describeError(err);
  }
}

async function loadTrash() {
  trashLoading.value = true;
  trashError.value = null;
  try {
    const page = await journalTrashList(50);
    trash.value = page.items;
  } catch (err) {
    trash.value = [];
    trashError.value = describeError(err);
  } finally {
    trashLoading.value = false;
  }
}

async function setTheme(theme: Theme) {
  await session.patch({ theme });
}

async function setLanguage(value: string) {
  await session.patch({ language: value === "en" ? "en" : "zh-CN" });
}

async function setWeekStart(value: number) {
  await session.patch({ weekStart: value });
}

async function setDateFormat(value: string) {
  if (value === "slash" || value === "short" || value === "zh") {
    await session.patch({ dateFormat: value });
  }
}

async function toggleIndent() {
  await session.patch({ indentFirstLine: !session.settings.indentFirstLine });
}

const boundaryValue = computed(() =>
  formatBoundary(session.settings.dayBoundaryMinutes),
);

async function setDayBoundary(value: string) {
  const minutes = parseBoundary(value);
  if (minutes === session.settings.dayBoundaryMinutes) return;
  await session.patch({ dayBoundaryMinutes: minutes });
}

async function toggleReduceMotion() {
  await session.patch({ reduceMotion: !session.settings.reduceMotion });
}

async function openFolder() {
  try {
    await openDataDir();
  } catch (err) {
    failure.value = describeError(err);
  }
}

async function restoreEntry(id: string) {
  opError.value = null;
  try {
    await journalRestore(id);
    await Promise.all([loadInfo(), loadTrash()]);
  } catch (err) {
    opError.value = describeError(err);
  }
}

type PendingAction =
  | { kind: "purge"; id: string }
  | { kind: "empty" }
  | { kind: "restore"; path: string };

const pending = ref<PendingAction | null>(null);

const confirmTitle = computed(() => {
  if (pending.value?.kind === "empty") return t("me.emptyTrashTitle");
  if (pending.value?.kind === "restore") return t("me.restore");
  return t("me.purgeTitle");
});

const confirmMessage = computed(() => {
  if (pending.value?.kind === "empty") return t("me.emptyTrashConfirm");
  if (pending.value?.kind === "restore") return t("me.restoreConfirm");
  return t("me.purgeConfirm");
});

const confirmLabel = computed(() => {
  if (pending.value?.kind === "empty") return t("me.emptyTrash");
  if (pending.value?.kind === "restore") return t("me.restore");
  return t("common.purge");
});

const opBusy = ref(false);
const opLabel = ref<string | null>(null);
const progress = ref<OpProgress | null>(null);
const notice = ref<string | null>(null);
const opError = ref<string | null>(null);

async function runOp(label: string, task: () => Promise<void>) {
  if (opBusy.value) return;
  opBusy.value = true;
  opLabel.value = label;
  progress.value = null;
  notice.value = null;
  opError.value = null;
  try {
    await task();
  } catch (err) {
    opError.value = describeError(err);
  } finally {
    opBusy.value = false;
    opLabel.value = null;
    progress.value = null;
  }
}

async function exportEntries() {
  if (opBusy.value) return;
  const target = await openDialog({
    directory: true,
    multiple: false,
    title: t("me.export"),
  });
  if (typeof target !== "string") return;
  await runOp(t("me.export"), async () => {
    const report = await dataExportDir(target);
    notice.value = t("me.exportDone", { n: report.journals, dir: report.dir });
  });
}

async function importEntries() {
  if (opBusy.value) return;
  const source = await openDialog({
    directory: true,
    multiple: false,
    title: t("me.import"),
  });
  if (typeof source !== "string") return;
  await runOp(t("me.import"), async () => {
    const report = await dataImportMd(source);
    notice.value = t("me.importDone", { n: report.journals });
    await Promise.all([loadInfo(), loadTrash()]);
  });
}

async function backupData() {
  if (opBusy.value) return;
  const target = await saveDialog({
    title: t("me.backup"),
    defaultPath: "Diary2Day-Backup.zip",
    filters: [{ name: "Diary2Day", extensions: ["zip"] }],
  });
  if (!target) return;
  await runOp(t("me.backup"), async () => {
    const report = await dataBackup(target);
    notice.value = t("me.backupDone", { size: formatBytes(report.size) });
  });
}

async function pickRestore() {
  if (opBusy.value) return;
  const source = await openDialog({
    multiple: false,
    title: t("me.restore"),
    filters: [{ name: "Diary2Day", extensions: ["zip"] }],
  });
  if (typeof source !== "string") return;
  pending.value = { kind: "restore", path: source };
}

type LockDialogState = "set" | "disable" | null;

const lockDialog = ref<LockDialogState>(null);
const lockBusy = ref(false);
const lockError = ref<string | null>(null);

function openLockSet() {
  lockError.value = null;
  lockDialog.value = "set";
}

function openLockDisable() {
  lockError.value = null;
  lockDialog.value = "disable";
}

async function submitLockCode(code: string) {
  if (lockBusy.value) return;
  lockBusy.value = true;
  lockError.value = null;
  try {
    if (lockDialog.value === "set") await lock.setCode(code);
    else await lock.clear(code);
    lockDialog.value = null;
  } catch (err) {
    lockError.value = describeError(err);
  } finally {
    lockBusy.value = false;
  }
}

const lockDelays = [0, 30, 60, 300, 600];

function lockDelayLabel(seconds: number): string {
  if (seconds <= 0) return t("me.lockDelayOff");
  if (seconds < 60) return t("me.lockDelaySeconds", { n: seconds });
  return t("me.lockDelayMinutes", { n: Math.round(seconds / 60) });
}

async function setLockDelay(value: number) {
  await session.patch({ lockLeaveDelayS: value });
}

function purgeEntry(id: string) {
  pending.value = { kind: "purge", id };
}

function emptyTrash() {
  if (!trash.value.length) return;
  pending.value = { kind: "empty" };
}

async function runPending() {
  const action = pending.value;
  pending.value = null;
  if (!action) return;
  if (action.kind === "restore") {
    await runOp(t("me.restore"), async () => {
      const report = await dataRestore(action.path);
      notice.value = t("me.restoreDone", { n: report.journals });
      await Promise.all([session.load(), lock.load()]);
      await Promise.all([loadInfo(), loadTrash()]);
    });
    return;
  }
  opError.value = null;
  try {
    if (action.kind === "empty") await journalEmptyTrash();
    else await journalPurge(action.id);
    await Promise.all([loadInfo(), loadTrash()]);
  } catch (err) {
    opError.value = describeError(err);
  }
}

let unlisten: (() => void) | null = null;
let unlistenProgress: (() => void) | null = null;

onMounted(async () => {
  await Promise.all([loadInfo(), loadTrash()]);
  unlisten = await onDataChanged(() => {
    void loadInfo();
    void loadTrash();
  });
  unlistenProgress = await onOpProgress((payload) => {
    progress.value = payload;
  });
});

onBeforeUnmount(() => {
  unlisten?.();
  unlistenProgress?.();
});
</script>

<template>
  <div class="view">
    <header class="topbar">
      <h1 class="heading">{{ t("me.title") }}</h1>
    </header>

    <div class="scroll">
      <section class="group">
        <h2 class="group-title">{{ t("me.appearance") }}</h2>
        <div class="rows">
          <div class="row">
            <span class="label">{{ t("me.theme") }}</span>
            <div class="segmented">
              <button
                v-for="theme in themes"
                :key="theme"
                type="button"
                :class="{ active: session.settings.theme === theme }"
                @click="setTheme(theme)"
              >
                {{ t(`me.theme.${theme}` as MessageKey) }}
              </button>
            </div>
          </div>
          <div class="row">
            <span class="label">{{ t("me.language") }}</span>
            <span class="select-wrap">
              <select
                class="select"
                :value="session.settings.language"
                @change="setLanguage(($event.target as HTMLSelectElement).value)"
              >
                <option value="zh-CN">中文</option>
                <option value="en">English</option>
              </select>
              <Icon class="select-chevron" :path="mdiChevronDown" :size="16" />
            </span>
          </div>
          <div class="row">
            <span class="label">{{ t("me.reduceMotion") }}</span>
            <button
              class="switch"
              type="button"
              role="switch"
              :aria-checked="session.settings.reduceMotion"
              :class="{ on: session.settings.reduceMotion }"
              @click="toggleReduceMotion"
            >
              <span class="knob" />
            </button>
          </div>
        </div>
      </section>

      <section class="group">
        <h2 class="group-title">{{ t("me.calendar") }}</h2>
        <div class="rows">
          <div class="row">
            <span class="label">{{ t("me.weekStart") }}</span>
            <span class="select-wrap">
              <select
                class="select"
                :value="session.settings.weekStart"
                @change="setWeekStart(Number(($event.target as HTMLSelectElement).value))"
              >
                <option
                  v-for="option in weekdayOptions"
                  :key="option.value"
                  :value="option.value"
                >
                  {{ option.label }}
                </option>
              </select>
              <Icon class="select-chevron" :path="mdiChevronDown" :size="16" />
            </span>
          </div>
          <div class="row">
            <span class="label">{{ t("me.dateFormat") }}</span>
            <span class="select-wrap">
              <select
                class="select"
                :value="session.settings.dateFormat"
                @change="setDateFormat(($event.target as HTMLSelectElement).value)"
              >
                <option value="slash">{{ t("me.dateFormat.slash") }}</option>
                <option value="short">{{ t("me.dateFormat.short") }}</option>
                <option value="zh">{{ t("me.dateFormat.zh") }}</option>
              </select>
              <Icon class="select-chevron" :path="mdiChevronDown" :size="16" />
            </span>
          </div>
        </div>
      </section>

      <section class="group">
        <h2 class="group-title">{{ t("me.recording") }}</h2>
        <div class="rows">
          <div class="row">
            <div class="row-main">
              <span class="label">{{ t("me.dayBoundary") }}</span>
              <span class="sub">{{ t("me.dayBoundaryHint") }}</span>
            </div>
            <input
              class="select time-input"
              type="time"
              min="00:00"
              max="07:59"
              step="60"
              :value="boundaryValue"
              @change="setDayBoundary(($event.target as HTMLInputElement).value)"
            />
          </div>
          <div class="row">
            <span class="label">{{ t("me.indent") }}</span>
            <button
              class="switch"
              type="button"
              role="switch"
              :aria-checked="session.settings.indentFirstLine"
              :class="{ on: session.settings.indentFirstLine }"
              @click="toggleIndent"
            >
              <span class="knob" />
            </button>
          </div>
        </div>
      </section>

      <section class="group">
        <h2 class="group-title">{{ t("me.lock") }}</h2>
        <div class="rows">
          <div class="row">
            <div class="row-main">
              <span class="label">{{ t("me.lock") }}</span>
              <span class="sub">{{ t("me.lockHint") }}</span>
            </div>
            <button
              v-if="!lock.enabled"
              class="text-btn"
              type="button"
              @click="openLockSet"
            >
              {{ t("me.lockEnable") }}
            </button>
            <button
              v-else
              class="text-btn danger"
              type="button"
              @click="openLockDisable"
            >
              {{ t("me.lockDisable") }}
            </button>
          </div>
          <div v-if="lock.enabled" class="row">
            <span class="label">{{ t("me.lockDelay") }}</span>
            <span class="select-wrap">
              <select
                class="select"
                :value="session.settings.lockLeaveDelayS"
                @change="setLockDelay(Number(($event.target as HTMLSelectElement).value))"
              >
                <option v-for="value in lockDelays" :key="value" :value="value">
                  {{ lockDelayLabel(value) }}
                </option>
              </select>
              <Icon class="select-chevron" :path="mdiChevronDown" :size="16" />
            </span>
          </div>
        </div>
      </section>

      <section class="group">
        <h2 class="group-title">{{ t("me.data") }}</h2>
        <div class="rows">
          <div class="row stack">
            <span class="label">{{ t("me.dataDir") }}</span>
            <code class="path">{{ info?.data_dir ?? "—" }}</code>
          </div>
          <div class="row">
            <span class="label">{{ t("me.database") }}</span>
            <span class="value">{{ formatBytes(info?.db_size ?? 0) }}</span>
          </div>
          <div class="row">
            <span class="label">{{ t("me.assets") }}</span>
            <span class="value">{{ formatBytes(info?.assets_size ?? 0) }}</span>
          </div>
          <div v-if="info" class="row">
            <span class="label">{{ t("me.counts", {
              journals: info.counts.journals,
              trash: info.counts.trash,
              words: info.counts.words,
            }) }}</span>
          </div>
          <div class="row">
            <button class="text-btn" type="button" @click="openFolder">
              <Icon :path="mdiFolderOpenOutline" :size="18" />
              {{ t("me.openDataDir") }}
            </button>
          </div>
          <button
            class="row action-row"
            type="button"
            :disabled="opBusy"
            @click="exportEntries"
          >
            <span class="action-main">
              <span class="label">{{ t("me.export") }}</span>
              <span class="sub">{{ t("me.exportHint") }}</span>
            </span>
            <Icon :path="mdiFileExportOutline" :size="20" />
          </button>
          <button
            class="row action-row"
            type="button"
            :disabled="opBusy"
            @click="importEntries"
          >
            <span class="action-main">
              <span class="label">{{ t("me.import") }}</span>
            </span>
            <Icon :path="mdiFileImportOutline" :size="20" />
          </button>
          <button
            class="row action-row"
            type="button"
            :disabled="opBusy"
            @click="backupData"
          >
            <span class="action-main">
              <span class="label">{{ t("me.backup") }}</span>
              <span class="sub">{{ t("me.backupHint") }}</span>
            </span>
            <Icon :path="mdiCloudUploadOutline" :size="20" />
          </button>
          <button
            class="row action-row"
            type="button"
            :disabled="opBusy"
            @click="pickRestore"
          >
            <span class="action-main">
              <span class="label">{{ t("me.restore") }}</span>
            </span>
            <Icon :path="mdiCloudDownloadOutline" :size="20" />
          </button>
          <div v-if="opBusy" class="row">
            <span class="label">{{ opLabel }}</span>
            <span class="value">
              {{
                progress && progress.total > 0
                  ? t("me.progress", { done: progress.done, total: progress.total })
                  : "…"
              }}
            </span>
          </div>
          <div v-else-if="opError" class="row">
            <span class="label">{{ t("me.opFailed") }}</span>
            <span class="value warn">{{ opError }}</span>
          </div>
          <div v-else-if="notice" class="row">
            <span class="value notice">{{ notice }}</span>
          </div>
        </div>
      </section>

      <section class="group">
        <div class="group-head">
          <h2 class="group-title">{{ t("me.trash") }}</h2>
          <button
            v-if="trash.length"
            class="text-btn danger"
            type="button"
            @click="emptyTrash"
          >
            {{ t("me.emptyTrash") }}
          </button>
        </div>
        <StateBlock v-if="trashLoading" kind="loading" />
        <StateBlock
          v-else-if="trashError"
          kind="error"
          :title="t('common.error')"
          :hint="trashError ?? undefined"
          :action-label="t('common.retry')"
          @action="loadTrash"
        />
        <StateBlock v-else-if="!trash.length" kind="empty" :title="t('me.trashEmpty')" />
        <div v-else class="trash-list">
          <div v-for="item in trash" :key="item.id" class="trash-item">
            <div class="trash-main">
              <p class="trash-title">{{ item.title || t("common.untitled") }}</p>
              <p class="trash-meta">
                {{
                  formatCardDate(
                    item.recorded_at,
                    item.recorded_off,
                    session.settings.dayBoundaryMinutes,
                  )
                }}
              </p>
            </div>
            <div class="trash-actions">
              <button
                class="icon-btn"
                type="button"
                :aria-label="t('common.restore')"
                @click="restoreEntry(item.id)"
              >
                <Icon :path="mdiRestore" :size="20" />
              </button>
              <button
                class="icon-btn danger"
                type="button"
                :aria-label="t('common.purge')"
                @click="purgeEntry(item.id)"
              >
                <Icon :path="mdiTrashCanOutline" :size="20" />
              </button>
            </div>
          </div>
        </div>
      </section>

      <section class="group">
        <h2 class="group-title">{{ t("me.about") }}</h2>
        <div class="rows">
          <div class="row">
            <span class="value">{{ t("me.version") }}</span>
          </div>
          <StateBlock
            v-if="failure"
            kind="error"
            :title="t('common.error')"
            :hint="failure ?? undefined"
            :action-label="t('common.retry')"
            @action="loadInfo"
          />
        </div>
      </section>
    </div>

    <ConfirmDialog
      :open="pending !== null"
      :title="confirmTitle"
      :message="confirmMessage"
      :confirm-label="confirmLabel"
      danger
      @confirm="runPending"
      @cancel="pending = null"
    />

    <CodeDialog
      :open="lockDialog !== null"
      :title="lockDialog === 'disable' ? t('me.lockDisableTitle') : t('me.lockSetTitle')"
      :hint="lockDialog === 'disable' ? t('me.lockDisableHint') : t('me.lockSetHint')"
      :confirm-label="lockDialog === 'disable' ? t('me.lockDisable') : t('me.lockEnable')"
      :needs-confirm="lockDialog === 'set'"
      :busy="lockBusy"
      :error="lockError"
      @submit="submitLockCode"
      @cancel="lockDialog = null"
    />
  </div>
</template>

<style scoped>
.view {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
}

.topbar {
  padding: var(--s-3) var(--s-4) var(--s-2);
}

.heading {
  font-size: var(--t-display);
  line-height: var(--t-display-lh);
  font-weight: 500;
}

.scroll {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 0 var(--s-4) var(--s-8);
}

.group {
  margin-top: var(--s-5);
}

.group-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--s-3);
}

.group-title {
  font-size: var(--t-small);
  line-height: var(--t-small-lh);
  color: var(--c-on-surface-muted);
  font-weight: 500;
  letter-spacing: 0.04em;
  margin-bottom: var(--s-2);
}

.rows {
  border-radius: var(--r-md);
  background: var(--c-surface-1);
  border: 1px solid var(--c-hairline);
  overflow: hidden;
}

.row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--s-3);
  min-height: 48px;
  padding: var(--s-2) var(--s-4);
  border-bottom: 1px solid var(--c-hairline);
  font-size: var(--t-small);
}

.row:last-child {
  border-bottom: none;
}

.row.stack {
  flex-direction: column;
  align-items: flex-start;
  gap: var(--s-1);
  padding-block: var(--s-3);
}

.label {
  color: var(--c-on-surface);
}

.row-main {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}

.sub {
  font-size: var(--t-caption);
  line-height: var(--t-caption-lh);
  color: var(--c-on-surface-muted);
}

.value {
  color: var(--c-on-surface-muted);
  font-size: var(--t-small);
}

.value.muted {
  color: var(--c-on-surface-faint);
}

.path {
  font-family: var(--font-mono);
  font-size: 11px;
  line-height: 1.5;
  color: var(--c-on-surface-muted);
  word-break: break-all;
}

.segmented {
  display: inline-flex;
  padding: 2px;
  border-radius: var(--r-pill);
  background: var(--c-surface-2);
}

.segmented button {
  padding: var(--s-1) var(--s-3);
  border-radius: var(--r-pill);
  font-size: var(--t-small);
  color: var(--c-on-surface-muted);
  min-height: 32px;
}

.segmented button.active {
  background: var(--c-surface-1);
  color: var(--c-primary);
  box-shadow: var(--e-1);
}

.select-wrap {
  position: relative;
  display: inline-flex;
  align-items: center;
  flex: none;
}

.select {
  appearance: none;
  -webkit-appearance: none;
  min-height: 34px;
  padding-block: var(--s-1);
  padding-inline: var(--s-3) var(--s-7);
  border: 1px solid var(--c-hairline);
  border-radius: var(--r-sm);
  background: var(--c-surface-2);
  color: var(--c-on-surface);
  font-size: var(--t-small);
  cursor: pointer;
}

.select:focus-visible {
  border-color: var(--c-primary);
  outline: none;
}

.time-input {
  padding-inline: var(--s-3);
  font-variant-numeric: tabular-nums;
  color-scheme: light dark;
}

.select-chevron {
  position: absolute;
  inset-inline-end: var(--s-2);
  color: var(--c-on-surface-muted);
  pointer-events: none;
}

.switch {
  width: 44px;
  height: 26px;
  border-radius: var(--r-pill);
  background: var(--c-surface-2);
  position: relative;
  transition: background var(--m-2) var(--ease);
  flex: none;
}

.switch.on {
  background: var(--c-primary);
}

.knob {
  position: absolute;
  top: 3px;
  inset-inline-start: 3px;
  width: 20px;
  height: 20px;
  border-radius: var(--r-pill);
  background: var(--c-surface-1);
  transition: transform var(--m-2) var(--ease);
}

.switch.on .knob {
  transform: translateX(18px);
}

html[dir="rtl"] .switch.on .knob {
  transform: translateX(-18px);
}

.text-btn {
  display: inline-flex;
  align-items: center;
  gap: var(--s-1);
  color: var(--c-primary);
  font-size: var(--t-small);
  min-height: 36px;
}

.action-row {
  width: 100%;
  background: transparent;
  text-align: start;
  color: var(--c-on-surface-muted);
  cursor: pointer;
  transition: background var(--m-1) var(--ease);
}

.action-row:hover:not(:disabled) {
  background: var(--c-surface-2);
}

.action-row:disabled {
  opacity: 0.5;
  cursor: default;
}

.action-main {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}

.value.warn {
  color: var(--c-danger);
  max-width: 60%;
  text-align: end;
  word-break: break-word;
}

.value.notice {
  color: var(--c-primary);
  word-break: break-word;
}

.text-btn.danger {
  color: var(--c-danger);
  min-height: 32px;
}

.trash-list {
  border-radius: var(--r-md);
  background: var(--c-surface-1);
  border: 1px solid var(--c-hairline);
  overflow: hidden;
}

.trash-item {
  display: flex;
  align-items: center;
  gap: var(--s-2);
  padding: var(--s-2) var(--s-2) var(--s-2) var(--s-4);
  border-bottom: 1px solid var(--c-hairline);
}

.trash-item:last-child {
  border-bottom: none;
}

.trash-main {
  flex: 1;
  min-width: 0;
}

.trash-title {
  font-size: var(--t-small);
  line-height: 1.5;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.trash-meta {
  font-size: var(--t-caption);
  line-height: var(--t-caption-lh);
  color: var(--c-on-surface-muted);
}

.trash-actions {
  display: flex;
  flex: none;
}

.icon-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 44px;
  height: 44px;
  border-radius: var(--r-pill);
  color: var(--c-on-surface-muted);
}

.icon-btn.danger {
  color: var(--c-danger);
}
</style>