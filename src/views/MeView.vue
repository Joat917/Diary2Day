<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { useRouter } from "vue-router";
import { open as openDialog, save as saveDialog } from "@tauri-apps/plugin-dialog";
import {
  mdiChevronDown,
  mdiChevronRight,
  mdiCloudDownloadOutline,
  mdiCloudUploadOutline,
  mdiContentCopy,
  mdiDeleteSweepOutline,
  mdiFileExportOutline,
  mdiFileImportOutline,
  mdiFolderOpenOutline,
  mdiRestore,
  mdiTrashCanOutline,
} from "@mdi/js";
import {
  assetCleanUnused,
  backupPasswordGet,
  backupPasswordSet,
  dataBackup,
  dataBackupAndroid,
  dataExportAndroid,
  dataExportDir,
  dataImportAndroid,
  dataImportMd,
  dataInfo,
  dataPickRestoreAndroid,
  dataRestore,
  dataRestoreAndroid,
  describeError,
  IpcError,
  journalEmptyTrash,
  journalPurge,
  journalRestore,
  journalTrashList,
  onDataChanged,
  onOpProgress,
  openDataDir,
} from "../ipc/client";
import type { DataInfo, JournalSummary, OpProgress } from "../ipc/types";
import { t, type MessageKey } from "../i18n";
import {
  activeCustomTheme,
  customThemeIndex,
  themeLabelKey,
} from "../constants/themes";
import { version as appVersion } from "../../package.json";
import { useLock } from "../stores/lock";
import { useSession } from "../stores/session";
import {
  formatBoundary,
  formatBytes,
  formatCardTime,
  parseBoundary,
} from "../utils/date";
import BackupDialog from "../components/BackupDialog.vue";
import CodeDialog from "../components/CodeDialog.vue";
import ConfirmDialog from "../components/ConfirmDialog.vue";
import Icon from "../components/Icon.vue";
import StateBlock from "../components/StateBlock.vue";

const session = useSession();
const router = useRouter();
const lock = useLock();
const info = ref<DataInfo | null>(null);
const mobile = computed(() => info.value?.mobile === true);
const trash = ref<JournalSummary[]>([]);
const trashLoading = ref(true);
const trashError = ref<string | null>(null);
const failure = ref<string | null>(null);

const themeLabel = computed(() => {
  const settings = session.settings;
  if (settings.theme !== "custom") return t(themeLabelKey(settings.theme));
  const index = customThemeIndex(settings);
  return activeCustomTheme(settings) && index >= 0
    ? t("theme.customN", { n: index + 1 })
    : t("theme.custom");
});

const stats = computed(() => {
  const data = info.value;
  return [
    { label: t("me.database"), value: data ? formatBytes(data.db_size) : "—" },
    { label: t("me.assets"), value: data ? formatBytes(data.assets_size) : "—" },
    { label: t("me.journals"), value: data ? String(data.counts.journals) : "—" },
    { label: t("me.trash"), value: data ? String(data.counts.trash) : "—" },
    { label: t("me.words"), value: data ? String(data.counts.words) : "—" },
  ];
});

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

async function toggleCardTime() {
  await session.patch({
    cardTime: session.settings.cardTime === "updated" ? "recorded" : "updated",
  });
}

async function toggleIndent() {
  await session.patch({ indentFirstLine: !session.settings.indentFirstLine });
}

async function toggleAutoEdit() {
  await session.patch({ autoEdit: !session.settings.autoEdit });
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

const EXPORT_WIDTH_MIN = 320;
const EXPORT_WIDTH_MAX = 1280;

async function setExportWidth(input: HTMLInputElement) {
  const raw = input.value.trim();
  const parsed = Number(raw);
  const value =
    raw && Number.isFinite(parsed)
      ? Math.min(Math.max(Math.round(parsed), EXPORT_WIDTH_MIN), EXPORT_WIDTH_MAX)
      : session.settings.exportWidth;
  // 清空或乱填时把显示恢复成当前设置，不发请求。
  input.value = String(value);
  if (value === session.settings.exportWidth) return;
  await session.patch({ exportWidth: value });
}

async function openFolder() {
  try {
    await openDataDir();
  } catch (err) {
    failure.value = describeError(err);
  }
}

async function copyPath() {
  const path = info.value?.data_dir;
  if (!path) return;
  opError.value = null;
  try {
    await navigator.clipboard.writeText(path);
  } catch (err) {
    opError.value = describeError(err);
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
  | { kind: "restore"; path: string }
  | { kind: "cleanup" };

const pending = ref<PendingAction | null>(null);

const confirmTitle = computed(() => {
  if (pending.value?.kind === "empty") return t("me.emptyTrashTitle");
  if (pending.value?.kind === "restore") return t("me.restore");
  if (pending.value?.kind === "cleanup") return t("me.cleanAssets");
  return t("me.purgeTitle");
});

const confirmMessage = computed(() => {
  if (pending.value?.kind === "empty") return t("me.emptyTrashConfirm");
  if (pending.value?.kind === "restore") return t("me.restoreConfirm");
  if (pending.value?.kind === "cleanup") return t("me.cleanAssetsConfirm");
  return t("me.purgeConfirm");
});

const confirmLabel = computed(() => {
  if (pending.value?.kind === "empty") return t("me.emptyTrash");
  if (pending.value?.kind === "restore") return t("me.restore");
  if (pending.value?.kind === "cleanup") return t("me.cleanAssets");
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
  if (mobile.value) {
    await runOp(t("me.export"), async () => {
      const report = await dataExportAndroid();
      if (!report) return;
      notice.value = t("me.exportDone", { n: report.journals, dir: report.dir });
    });
    return;
  }
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
  if (mobile.value) {
    await runOp(t("me.import"), async () => {
      const report = await dataImportAndroid();
      if (!report) return;
      notice.value = t("me.importDone", { n: report.journals });
      await Promise.all([loadInfo(), loadTrash()]);
    });
    return;
  }
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

type BackupDialogMode = "create" | "restore";

const backupDialog = ref<BackupDialogMode | null>(null);
const backupPath = ref<string | null>(null);
const backupBusy = ref(false);
const backupDialogError = ref<string | null>(null);
const rememberedPassword = ref<string | null>(null);

async function loadRememberedPassword() {
  try {
    rememberedPassword.value = await backupPasswordGet();
  } catch (err) {
    rememberedPassword.value = null;
    opError.value = describeError(err);
  }
}

async function persistRememberedPassword(password: string | null, remember: boolean) {
  const value = remember ? password : null;
  rememberedPassword.value = value;
  try {
    await backupPasswordSet(value);
  } catch (err) {
    opError.value = describeError(err);
  }
}

function isPasswordRequired(err: unknown): boolean {
  return err instanceof IpcError && err.code === "password_required";
}

function backupDialogMessage(err: unknown): string {
  if (err instanceof IpcError && err.code === "bad_password") {
    return t("me.backupPwWrong");
  }
  return describeError(err);
}

function backupFileName(encrypted: boolean): string {
  const now = new Date();
  const pad = (value: number) => String(value).padStart(2, "0");
  const stamp = `${now.getFullYear()}${pad(now.getMonth() + 1)}${pad(now.getDate())}-${pad(now.getHours())}${pad(now.getMinutes())}${pad(now.getSeconds())}`;
  return `Diary2Day-Backup-${stamp}.${encrypted ? "ddbak" : "zip"}`;
}

async function openBackup() {
  if (opBusy.value) return;
  backupPath.value = null;
  backupDialogError.value = null;
  await loadRememberedPassword();
  backupDialog.value = "create";
}

async function askRestorePassword(path: string) {
  backupPath.value = path;
  backupDialogError.value = null;
  await loadRememberedPassword();
  backupDialog.value = "restore";
}

function closeBackupDialog() {
  if (backupBusy.value) return;
  backupDialog.value = null;
  backupPath.value = null;
  backupDialogError.value = null;
}

async function restoreFrom(path: string, password?: string | null) {
  const report = mobile.value
    ? await dataRestoreAndroid(path, password)
    : await dataRestore(path, password);
  notice.value = t("me.restoreDone", { n: report.journals });
  await Promise.all([session.load(), lock.load()]);
  await Promise.all([loadInfo(), loadTrash()]);
}

async function submitBackup(password: string | null, remember: boolean) {
  if (backupBusy.value) return;
  const mode = backupDialog.value;
  if (!mode) return;
  if (mode === "restore") {
    const path = backupPath.value;
    if (!path) {
      closeBackupDialog();
      return;
    }
    backupBusy.value = true;
    backupDialogError.value = null;
    await runOp(t("me.restore"), async () => {
      try {
        await restoreFrom(path, password);
      } catch (err) {
        backupDialogError.value = isPasswordRequired(err)
          ? t("me.backupPwRequired")
          : backupDialogMessage(err);
        return;
      }
      backupDialog.value = null;
      backupPath.value = null;
      await persistRememberedPassword(password, remember);
    });
    backupBusy.value = false;
    return;
  }

  backupDialog.value = null;
  const encrypted = password !== null;
  if (mobile.value) {
    await runOp(t("me.backup"), async () => {
      const report = await dataBackupAndroid(password);
      if (!report) return;
      notice.value = t("me.backupDone", { size: formatBytes(report.size) });
      await persistRememberedPassword(password, remember);
    });
    return;
  }
  const target = await saveDialog({
    title: t("me.backup"),
    defaultPath: backupFileName(encrypted),
    filters: [{ name: "Diary2Day", extensions: [encrypted ? "ddbak" : "zip"] }],
  });
  if (!target) return;
  await runOp(t("me.backup"), async () => {
    const report = await dataBackup(target, password);
    notice.value = t("me.backupDone", { size: formatBytes(report.size) });
    await persistRememberedPassword(password, remember);
  });
}

async function pickRestore() {
  if (opBusy.value) return;
  if (mobile.value) {
    try {
      const picked = await dataPickRestoreAndroid();
      if (!picked) return;
      pending.value = { kind: "restore", path: picked.path };
    } catch (err) {
      opError.value = describeError(err);
    }
    return;
  }
  const source = await openDialog({
    multiple: false,
    title: t("me.restore"),
    filters: [{ name: "Diary2Day", extensions: ["zip", "ddbak"] }],
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
      try {
        await restoreFrom(action.path);
      } catch (err) {
        if (isPasswordRequired(err)) {
          await askRestorePassword(action.path);
          return;
        }
        throw err;
      }
    });
    return;
  }
  if (action.kind === "cleanup") {
    await runOp(t("me.cleanAssets"), async () => {
      const report = await assetCleanUnused();
      notice.value = report.removed
        ? t("me.cleanAssetsDone", {
            n: report.removed,
            size: formatBytes(report.freed),
          })
        : t("me.cleanAssetsNone");
      await loadInfo();
    });
    return;
  }
  opError.value = null;
  try {
    if (action.kind === "empty") await journalEmptyTrash();
    else if (action.kind === "purge") await journalPurge(action.id);
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
        <h2 class="group-title">{{ t("me.data") }}</h2>
        <div class="stats">
          <div v-for="stat in stats" :key="stat.label" class="stat">
            <span class="stat-label">{{ stat.label }}</span>
            <span class="stat-value">{{ stat.value }}</span>
          </div>
        </div>
        <div class="rows">
          <div v-if="info && !mobile" class="row">
            <div class="row-main">
              <span class="label">{{ t("me.dataDir") }}</span>
              <code class="path">{{ info?.data_dir ?? "—" }}</code>
            </div>
            <div class="dir-actions">
              <button
                class="icon-btn"
                type="button"
                :aria-label="t('me.openDataDir')"
                @click="openFolder"
              >
                <Icon :path="mdiFolderOpenOutline" :size="20" />
              </button>
              <button
                class="icon-btn"
                type="button"
                :aria-label="t('me.copyPath')"
                @click="copyPath"
              >
                <Icon :path="mdiContentCopy" :size="18" />
              </button>
            </div>
          </div>
          <button
            class="row action-row"
            type="button"
            :disabled="opBusy"
            @click="exportEntries"
          >
            <span class="action-main">
              <span class="label">{{ t("me.export") }}</span>
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
            @click="openBackup"
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
          <button
            class="row action-row"
            type="button"
            :disabled="opBusy"
            @click="pending = { kind: 'cleanup' }"
          >
            <span class="action-main">
              <span class="label">{{ t("me.cleanAssets") }}</span>
              <span class="sub">{{ t("me.cleanAssetsHint") }}</span>
            </span>
            <Icon :path="mdiDeleteSweepOutline" :size="20" />
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
        <h2 class="group-title">{{ t("me.appearance") }}</h2>
        <div class="rows">
          <div class="row">
            <span class="label">{{ t("me.theme") }}</span>
            <button class="theme-link" type="button" @click="router.push('/theme')">
              <span class="value">{{ themeLabel }}</span>
              <Icon :path="mdiChevronRight" :size="18" />
            </button>
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
          <div class="row">
            <span class="label">{{ t("me.exportWidth") }}</span>
            <span class="number-wrap">
              <input
                class="number-input"
                type="number"
                inputmode="numeric"
                :min="EXPORT_WIDTH_MIN"
                :max="EXPORT_WIDTH_MAX"
                step="1"
                :value="session.settings.exportWidth"
                @change="setExportWidth($event.target as HTMLInputElement)"
              />
              <span class="number-unit">px</span>
            </span>
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
          <div class="row">
            <span class="label">{{ t("me.cardTime") }}</span>
            <button
              class="switch"
              type="button"
              role="switch"
              :aria-checked="session.settings.cardTime === 'updated'"
              :class="{ on: session.settings.cardTime === 'updated' }"
              @click="toggleCardTime"
            >
              <span class="knob" />
            </button>
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
          <div class="row">
            <span class="label">{{ t("me.autoEdit") }}</span>
            <button
              class="switch"
              type="button"
              role="switch"
              :aria-checked="session.settings.autoEdit"
              :class="{ on: session.settings.autoEdit }"
              @click="toggleAutoEdit"
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
                  formatCardTime(
                    item.recorded_at,
                    item.recorded_off,
                    item.updated_at,
                    session.settings.cardTime === "updated",
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
            <span class="value">{{ t("me.version", { version: appVersion }) }}</span>
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

    <BackupDialog
      :open="backupDialog !== null"
      :mode="backupDialog === 'restore' ? 'restore' : 'create'"
      :busy="backupBusy"
      :error="backupDialogError"
      :initial-password="rememberedPassword"
      @submit="submitBackup"
      @cancel="closeBackupDialog"
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

.stats {
  display: grid;
  grid-template-columns: repeat(5, minmax(0, 1fr));
  margin-bottom: var(--s-3);
  border: 1px solid var(--c-hairline);
  border-radius: var(--r-md);
  background: var(--c-surface-1);
  overflow: hidden;
}

.stat {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 2px;
  min-width: 0;
  padding: var(--s-3) var(--s-2);
}

.stat:nth-child(-n + 2) {
  min-width: 100px;
}

.stat-label {
  font-size: var(--t-caption);
  line-height: var(--t-caption-lh);
  color: var(--c-on-surface-muted);
}

.stat-value {
  font-size: var(--t-title);
  line-height: var(--t-title-lh);
  font-weight: 600;
  font-variant-numeric: tabular-nums;
  white-space: nowrap;
}

@media (max-width: 559px) {
  .stats {
    grid-template-columns: repeat(6, minmax(0, 1fr));
  }

  .stat:nth-child(-n + 2) {
    grid-column: span 3;
  }

  .stat:nth-child(n + 3) {
    grid-column: span 2;
  }
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

.dir-actions {
  display: flex;
  flex: none;
}

.path {
  font-family: var(--font-mono);
  font-size: 11px;
  line-height: 1.5;
  color: var(--c-on-surface-muted);
  word-break: break-all;
}

.theme-link {
  display: inline-flex;
  align-items: center;
  gap: var(--s-1);
  color: var(--c-on-surface-muted);
  flex: none;
}

@media (hover: hover) {
  .theme-link:hover {
    color: var(--c-primary);
  }
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
}

.number-wrap {
  display: inline-flex;
  align-items: center;
  gap: var(--s-1);
  flex: none;
}

.number-input {
  appearance: textfield;
  -moz-appearance: textfield;
  width: 76px;
  min-height: 34px;
  padding-inline: var(--s-3);
  border: 1px solid var(--c-hairline);
  border-radius: var(--r-sm);
  background: var(--c-surface-2);
  color: var(--c-on-surface);
  font-size: var(--t-small);
  font-variant-numeric: tabular-nums;
  text-align: end;
}

.number-input::-webkit-outer-spin-button,
.number-input::-webkit-inner-spin-button {
  appearance: none;
  -webkit-appearance: none;
  margin: 0;
}

.number-input:focus-visible {
  border-color: var(--c-primary);
  outline: none;
}

.number-unit {
  color: var(--c-on-surface-muted);
  font-size: var(--t-small);
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

.text-btn:disabled {
  color: var(--c-on-surface-faint);
  cursor: default;
}

.action-row {
  width: 100%;
  background: transparent;
  text-align: start;
  color: var(--c-on-surface-muted);
  cursor: pointer;
  transition: background var(--m-1) var(--ease);
}

@media (hover: hover) {
  .action-row:hover:not(:disabled) {
    background: var(--c-surface-2);
  }
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