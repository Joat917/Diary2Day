<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { mdiFolderOpenOutline, mdiRestore, mdiTrashCanOutline } from "@mdi/js";
import {
  dataInfo,
  journalEmptyTrash,
  journalPurge,
  journalRestore,
  journalTrashList,
  onDataChanged,
  openDataDir,
} from "../ipc/client";
import type { DataInfo, JournalSummary, Theme } from "../ipc/types";
import { t, type MessageKey } from "../i18n";
import { useSession } from "../stores/session";
import { formatBytes, formatCardDate } from "../utils/date";
import ConfirmDialog from "../components/ConfirmDialog.vue";
import Icon from "../components/Icon.vue";
import StateBlock from "../components/StateBlock.vue";

const session = useSession();
const info = ref<DataInfo | null>(null);
const trash = ref<JournalSummary[]>([]);
const trashLoading = ref(true);
const failure = ref(false);

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
  } catch {
    failure.value = true;
  }
}

async function loadTrash() {
  trashLoading.value = true;
  try {
    const page = await journalTrashList(50);
    trash.value = page.items;
  } catch {
    trash.value = [];
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

async function openFolder() {
  try {
    await openDataDir();
  } catch {
    failure.value = true;
  }
}

async function restoreEntry(id: string) {
  await journalRestore(id);
  await Promise.all([loadInfo(), loadTrash()]);
}

type PendingAction = { kind: "purge"; id: string } | { kind: "empty" };

const pending = ref<PendingAction | null>(null);

const confirmTitle = computed(() =>
  pending.value?.kind === "empty" ? t("me.emptyTrashTitle") : t("me.purgeTitle"),
);

const confirmMessage = computed(() =>
  pending.value?.kind === "empty" ? t("me.emptyTrashConfirm") : t("me.purgeConfirm"),
);

const confirmLabel = computed(() =>
  pending.value?.kind === "empty" ? t("me.emptyTrash") : t("common.purge"),
);

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
  if (action.kind === "empty") await journalEmptyTrash();
  else await journalPurge(action.id);
  await Promise.all([loadInfo(), loadTrash()]);
}

let unlisten: (() => void) | null = null;

onMounted(async () => {
  await Promise.all([loadInfo(), loadTrash()]);
  unlisten = await onDataChanged(() => {
    void loadInfo();
    void loadTrash();
  });
});

onBeforeUnmount(() => unlisten?.());
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
            <select
              class="select"
              :value="session.settings.language"
              @change="setLanguage(($event.target as HTMLSelectElement).value)"
            >
              <option value="zh-CN">中文</option>
              <option value="en">English</option>
            </select>
          </div>
        </div>
      </section>

      <section class="group">
        <h2 class="group-title">{{ t("me.calendar") }}</h2>
        <div class="rows">
          <div class="row">
            <span class="label">{{ t("me.weekStart") }}</span>
            <select
              class="select"
              :value="session.settings.weekStart"
              @change="setWeekStart(Number(($event.target as HTMLSelectElement).value))"
            >
              <option v-for="option in weekdayOptions" :key="option.value" :value="option.value">
                {{ option.label }}
              </option>
            </select>
          </div>
          <div class="row">
            <span class="label">{{ t("me.dateFormat") }}</span>
            <select
              class="select"
              :value="session.settings.dateFormat"
              @change="setDateFormat(($event.target as HTMLSelectElement).value)"
            >
              <option value="slash">{{ t("me.dateFormat.slash") }}</option>
              <option value="short">{{ t("me.dateFormat.short") }}</option>
              <option value="zh">{{ t("me.dateFormat.zh") }}</option>
            </select>
          </div>
        </div>
      </section>

      <section class="group">
        <h2 class="group-title">{{ t("me.recording") }}</h2>
        <div class="rows">
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
        <StateBlock v-else-if="!trash.length" kind="empty" :title="t('me.trashEmpty')" />
        <div v-else class="trash-list">
          <div v-for="item in trash" :key="item.id" class="trash-item">
            <div class="trash-main">
              <p class="trash-title">{{ item.title || t("common.untitled") }}</p>
              <p class="trash-meta">
                {{ formatCardDate(item.recorded_at, item.recorded_off) }}
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
          <div class="row">
            <span class="value muted">{{ t("me.exportSoon") }}</span>
          </div>
          <StateBlock
            v-if="failure"
            kind="error"
            :title="t('common.error')"
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

.select {
  border: none;
  background: transparent;
  color: var(--c-primary);
  font-size: var(--t-small);
  text-align: end;
  padding: var(--s-1);
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