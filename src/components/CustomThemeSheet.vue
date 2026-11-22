<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { mdiArrowLeft } from "@mdi/js";
import {
  describeError,
  fontImport,
  fontRemove,
  themeBgImport,
  themeBgRemove,
} from "../ipc/client";
import type { CustomTheme } from "../ipc/types";
import { t, type MessageKey } from "../i18n";
import { cloneCustomTheme, LIGHT_THEMES, seedCustomTheme } from "../constants/themes";
import { themeBgUrl } from "../utils/assetUrl";
import { useBackLayer } from "../composables/useBackLayer";
import { useSession } from "../stores/session";
import ConfirmDialog from "../components/ConfirmDialog.vue";
import Icon from "../components/Icon.vue";

type ModuleKey = "surface" | "card" | "inset" | "scrim";
type TextKey = "onSurface" | "onSurfaceMuted" | "primary";

const props = defineProps<{ themeId: string }>();
const emit = defineEmits<{ close: []; delete: [id: string] }>();
const session = useSession();

const MODULES: { key: ModuleKey; label: MessageKey }[] = [
  { key: "surface", label: "theme.module.surface" },
  { key: "card", label: "theme.module.card" },
  { key: "inset", label: "theme.module.inset" },
  { key: "scrim", label: "theme.module.scrim" },
];

const TEXTS: { key: TextKey; label: MessageKey }[] = [
  { key: "onSurface", label: "theme.text" },
  { key: "onSurfaceMuted", label: "theme.textMuted" },
  { key: "primary", label: "theme.accent" },
];

const initial = session.settings.customThemes.find((theme) => theme.id === props.themeId);
const draft = ref<CustomTheme>(
  initial
    ? cloneCustomTheme(initial, initial.id)
    : seedCustomTheme(LIGHT_THEMES[0], props.themeId),
);
const busy = ref(false);
const error = ref<string | null>(null);
const confirmDelete = ref(false);
const backgroundBroken = ref(false);
const backgroundUrl = computed(() => themeBgUrl(draft.value.background));

const sheetStyle = computed<Record<string, string>>(() => ({
  "--c-theme-bg": backgroundUrl.value ? `url("${backgroundUrl.value}")` : "none",
}));
const checkerStyle: Record<string, string> = { "--c-theme-bg": "none" };

watch(
  () => draft.value.background,
  () => {
    backgroundBroken.value = false;
  },
);

let chain: Promise<unknown> = Promise.resolve();

function queuePatch(next: CustomTheme): Promise<void> {
  const snapshot = cloneCustomTheme(next, next.id);
  const task = chain.then(async () => {
    const customThemes = session.settings.customThemes.map((theme) =>
      theme.id === snapshot.id ? snapshot : theme,
    );
    await session.patch({ customThemes });
  });
  chain = task.catch(() => undefined);
  return task.then(() => undefined);
}

function backgroundShared(name: string): boolean {
  return session.settings.customThemes.some(
    (theme) => theme.id !== draft.value.id && theme.background === name,
  );
}

function fontShared(name: string): boolean {
  return session.settings.customThemes.some(
    (theme) => theme.id !== draft.value.id && theme.font?.name === name,
  );
}

async function commitDraft() {
  try {
    await queuePatch(draft.value);
    error.value = null;
  } catch (err) {
    error.value = describeError(err);
  }
}

function preview() {
  session.previewCustom(draft.value);
}

function setColor(key: ModuleKey, input: HTMLInputElement) {
  draft.value[key].color = input.value;
  preview();
}

function setAlpha(key: ModuleKey, input: HTMLInputElement) {
  draft.value[key].alpha = Number(input.value);
  preview();
}

function setText(key: TextKey, input: HTMLInputElement) {
  draft.value[key] = input.value;
  preview();
}

function setHairlineColor(input: HTMLInputElement) {
  draft.value.hairline.color = input.value;
  preview();
}

function setHairlineAlpha(input: HTMLInputElement) {
  draft.value.hairline.alpha = Number(input.value);
  preview();
}

type SampleKey = "sampleTitle" | "sampleText";

function setSample(key: SampleKey, input: HTMLInputElement) {
  draft.value[key] = input.value === "" ? null : input.value;
}

async function commitSample(key: SampleKey) {
  const text = draft.value[key]?.trim();
  draft.value[key] = text ? text : null;
  await commitDraft();
}

async function pickBackground() {
  if (busy.value) return;
  busy.value = true;
  error.value = null;
  const previous = draft.value.background;
  try {
    const imported = await themeBgImport();
    if (!imported) return;
    draft.value.background = imported.name;
    preview();
    await queuePatch(draft.value);
    if (previous && previous !== imported.name && !backgroundShared(previous)) {
      try {
        await themeBgRemove(previous);
      } catch (err) {
        error.value = describeError(err);
      }
    }
  } catch (err) {
    error.value = describeError(err);
  } finally {
    busy.value = false;
  }
}

async function removeBackground() {
  if (busy.value) return;
  const previous = draft.value.background;
  if (!previous) return;
  busy.value = true;
  error.value = null;
  draft.value.background = null;
  preview();
  try {
    await queuePatch(draft.value);
    if (!backgroundShared(previous)) {
      try {
        await themeBgRemove(previous);
      } catch (err) {
        error.value = describeError(err);
      }
    }
  } catch (err) {
    error.value = describeError(err);
  } finally {
    busy.value = false;
  }
}

async function pickFont() {
  if (busy.value) return;
  busy.value = true;
  error.value = null;
  const previous = draft.value.font;
  try {
    const imported = await fontImport();
    if (!imported) return;
    draft.value.font = { name: imported.name, label: imported.label };
    preview();
    await queuePatch(draft.value);
    if (
      previous &&
      previous.name !== imported.name &&
      !fontShared(previous.name)
    ) {
      try {
        await fontRemove(previous.name);
      } catch (err) {
        error.value = describeError(err);
      }
    }
  } catch (err) {
    error.value = describeError(err);
  } finally {
    busy.value = false;
  }
}

async function resetFont() {
  if (busy.value) return;
  const previous = draft.value.font;
  if (!previous) return;
  busy.value = true;
  error.value = null;
  draft.value.font = null;
  preview();
  try {
    await queuePatch(draft.value);
    if (!fontShared(previous.name)) {
      try {
        await fontRemove(previous.name);
      } catch (err) {
        error.value = describeError(err);
      }
    }
  } catch (err) {
    error.value = describeError(err);
  } finally {
    busy.value = false;
  }
}

function askDelete() {
  if (!busy.value) confirmDelete.value = true;
}

async function deleteTheme() {
  confirmDelete.value = false;
  busy.value = true;
  try {
    // 等编辑器自己的写入全部落库，再交给主题页删除，避免迟到写入把它带回来。
    await chain;
    emit("delete", draft.value.id);
  } finally {
    busy.value = false;
  }
}

function onBackgroundError(event: Event) {
  backgroundBroken.value = true;
  const img = event.target as HTMLImageElement | null;
  console.warn("[theme] background image failed to load", img?.src);
}

// 文案只在 change 时落库；Esc/返回键关闭前补一次，避免把刚输入的文字丢掉。
async function close() {
  const title = draft.value.sampleTitle?.trim() || null;
  const text = draft.value.sampleText?.trim() || null;
  if (title !== draft.value.sampleTitle || text !== draft.value.sampleText) {
    draft.value.sampleTitle = title;
    draft.value.sampleText = text;
    try {
      await queuePatch(draft.value);
    } catch (err) {
      error.value = describeError(err);
      return;
    }
  }
  emit("close");
}

useBackLayer(() => true, close);

function onKeydown(event: KeyboardEvent) {
  if (event.key === "Escape") {
    if (confirmDelete.value) return;
    event.preventDefault();
    close();
  }
}

const backButton = ref<HTMLButtonElement | null>(null);

onMounted(() => {
  window.addEventListener("keydown", onKeydown);
  backButton.value?.focus();
});

onBeforeUnmount(() => {
  window.removeEventListener("keydown", onKeydown);
  // 关闭或路由变化时都回到已存设置，清掉拖动中的临时预览。
  session.applyTheme();
});
</script>

<template>
  <Teleport to="body">
    <div
      class="sheet"
      role="dialog"
      aria-modal="true"
      :aria-label="t('theme.sheetTitle')"
      :style="sheetStyle"
    >
      <div class="sheet-bg checker" aria-hidden="true" />
      <div class="sheet-tint" aria-hidden="true" />
      <header class="topbar">
        <button
          ref="backButton"
          class="icon-btn"
          type="button"
          :aria-label="t('common.back')"
          @click="close"
        >
          <Icon :path="mdiArrowLeft" :size="22" />
        </button>
        <h1 class="heading">{{ t("theme.sheetTitle") }}</h1>
        <span class="spacer" />
      </header>

      <div class="scroll">
        <section class="panel">
          <label class="row field">
            <span class="row-label">{{ t("theme.sampleTitle") }}</span>
            <input
              class="field-input"
              type="text"
              maxlength="40"
              :value="draft.sampleTitle ?? ''"
              :placeholder="t('theme.customSample.title')"
              @input="setSample('sampleTitle', $event.target as HTMLInputElement)"
              @change="commitSample('sampleTitle')"
            />
          </label>
          <label class="row field">
            <span class="row-label">{{ t("theme.sampleText") }}</span>
            <input
              class="field-input"
              type="text"
              maxlength="120"
              :value="draft.sampleText ?? ''"
              :placeholder="t('theme.customSample.text')"
              @input="setSample('sampleText', $event.target as HTMLInputElement)"
              @change="commitSample('sampleText')"
            />
          </label>
        </section>

        <section class="panel">
          <div v-for="entry in TEXTS" :key="entry.key" class="row">
            <span class="row-label">{{ t(entry.label) }}</span>
            <input
              class="swatch"
              type="color"
              :value="draft[entry.key].toLowerCase()"
              :aria-label="t(entry.label)"
              @input="setText(entry.key, $event.target as HTMLInputElement)"
              @change="commitDraft"
            />
          </div>
          <div class="row">
            <span class="row-label">{{ t("theme.hairline") }}</span>
            <input
              class="swatch"
              type="color"
              :value="draft.hairline.color.toLowerCase()"
              :aria-label="t('theme.hairline')"
              @input="setHairlineColor($event.target as HTMLInputElement)"
              @change="commitDraft"
            />
            <input
              class="alpha"
              type="range"
              min="0"
              max="100"
              step="1"
              :value="draft.hairline.alpha"
              :aria-label="`${t('theme.hairline')} ${draft.hairline.alpha}%`"
              @input="setHairlineAlpha($event.target as HTMLInputElement)"
              @change="commitDraft"
            />
            <span class="alpha-value">{{ draft.hairline.alpha }}%</span>
          </div>
        </section>

        <section class="panel">
          <div v-for="module in MODULES" :key="module.key" class="row">
            <span class="row-label">{{ t(module.label) }}</span>
            <input
              class="swatch"
              type="color"
              :value="draft[module.key].color.toLowerCase()"
              :aria-label="t(module.label)"
              @input="setColor(module.key, $event.target as HTMLInputElement)"
              @change="commitDraft"
            />
            <input
              class="alpha"
              type="range"
              min="0"
              max="100"
              step="1"
              :value="draft[module.key].alpha"
              :aria-label="`${t(module.label)} ${draft[module.key].alpha}%`"
              @input="setAlpha(module.key, $event.target as HTMLInputElement)"
              @change="commitDraft"
            />
            <span class="alpha-value">{{ draft[module.key].alpha }}%</span>
          </div>
        </section>

        <section class="panel">
          <div class="row bg-row">
            <span class="row-label">{{ t("theme.background") }}</span>
            <span class="thumb checker" :style="checkerStyle">
              <img
                v-if="backgroundUrl && !backgroundBroken"
                :src="backgroundUrl"
                alt=""
                @error="onBackgroundError"
              />
            </span>
            <span class="bg-actions">
              <button
                class="text-btn"
                type="button"
                :disabled="busy"
                @click="pickBackground"
              >
                {{ t("theme.backgroundPick") }}
              </button>
              <button
                v-if="draft.background"
                class="text-btn danger"
                type="button"
                :disabled="busy"
                @click="removeBackground"
              >
                {{ t("theme.backgroundRemove") }}
              </button>
            </span>
          </div>
        </section>

        <section class="panel">
          <div class="row font-row">
            <span class="row-label">{{ t("theme.font") }}</span>
            <span class="font-actions">
              <span class="font-name">
                {{ draft.font?.label ?? t("theme.fontDefault") }}
              </span>
              <span class="font-buttons">
                <button
                  class="text-btn"
                  type="button"
                  :disabled="busy"
                  @click="pickFont"
                >
                  {{ t("theme.fontPick") }}
                </button>
                <button
                  v-if="draft.font"
                  class="text-btn danger"
                  type="button"
                  :disabled="busy"
                  @click="resetFont"
                >
                  {{ t("theme.fontReset") }}
                </button>
              </span>
            </span>
          </div>
        </section>

        <p v-if="error" class="error">{{ error }}</p>

        <button class="delete" type="button" :disabled="busy" @click="askDelete">
          {{ t("theme.delete") }}
        </button>
      </div>

      <ConfirmDialog
        :open="confirmDelete"
        :title="t('theme.deleteTitle')"
        :message="t('theme.deleteConfirm')"
        :confirm-label="t('theme.delete')"
        danger
        @confirm="deleteTheme"
        @cancel="confirmDelete = false"
      />
    </div>
  </Teleport>
</template>

<style scoped>
.sheet {
  position: fixed;
  inset: 0;
  z-index: 90;
  display: flex;
  flex-direction: column;
  padding-top: var(--safe-top);
  padding-inline: var(--safe-left) var(--safe-right);
}

/* 编辑层自带方格/背景层与半透明外壳：盖住下面的主题页，透明度只透出最底层。 */
.sheet-bg,
.sheet-tint {
  position: fixed;
  inset: 0;
  pointer-events: none;
}

.sheet-bg {
  z-index: -2;
}

.sheet-tint {
  z-index: -1;
  background: var(--c-surface);
}

.topbar {
  display: flex;
  align-items: center;
  gap: var(--s-2);
  min-height: var(--topbar-h);
  padding: var(--s-2) var(--s-3);
  border-bottom: 1px solid var(--c-hairline);
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

@media (hover: hover) {
  .icon-btn:hover {
    background: var(--c-surface-2);
  }
}

.heading {
  flex: 1;
  min-width: 0;
  text-align: center;
  font-size: var(--t-title);
  line-height: var(--t-title-lh);
  font-weight: 500;
}

.spacer {
  width: 44px;
  flex: none;
}

.scroll {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: var(--s-4);
  padding-bottom: calc(var(--s-8) + var(--safe-bottom) + var(--ime));
}

.panel {
  margin-top: var(--s-4);
  border-radius: var(--r-md);
  background: var(--c-surface-1);
  border: 1px solid var(--c-hairline);
  overflow: hidden;
}

.panel:first-child {
  margin-top: 0;
}

.row {
  display: flex;
  align-items: center;
  gap: var(--s-2);
  min-height: 48px;
  padding: var(--s-2) var(--s-4);
  border-bottom: 1px solid var(--c-hairline);
  font-size: var(--t-small);
}

.row:last-child {
  border-bottom: none;
}

.row-label {
  flex: 1;
  min-width: 0;
  color: var(--c-on-surface);
}

.row.field {
  gap: var(--s-3);
}

.row.field .row-label {
  flex: 0 0 auto;
  min-width: 72px;
}

.field-input {
  flex: 1;
  min-width: 0;
  height: 34px;
  padding-inline: var(--s-2);
  border: 1px solid var(--c-hairline);
  border-radius: var(--r-sm);
  background: var(--c-surface-2);
  color: var(--c-on-surface);
  font-size: var(--t-small);
}

.field-input::placeholder {
  color: var(--c-on-surface-faint);
}

.field-input:focus-visible {
  outline: 2px solid var(--c-primary);
  outline-offset: 1px;
}

.swatch {
  flex: none;
  width: 32px;
  height: 32px;
  padding: 0;
  border: 1px solid var(--c-hairline);
  border-radius: var(--r-sm);
  background: none;
  appearance: none;
  cursor: pointer;
}

.swatch::-webkit-color-swatch-wrapper {
  padding: 0;
}

.swatch::-webkit-color-swatch {
  border: none;
  border-radius: calc(var(--r-sm) - 1px);
}

.swatch::-moz-color-swatch {
  border: none;
  border-radius: calc(var(--r-sm) - 1px);
}

.alpha {
  flex: 1;
  min-width: 64px;
  max-width: 120px;
  height: 4px;
  margin: 0;
  border-radius: var(--r-pill);
  background: var(--c-surface-2);
  appearance: none;
}

.alpha::-webkit-slider-thumb {
  appearance: none;
  width: 18px;
  height: 18px;
  border-radius: var(--r-pill);
  background: var(--c-primary);
  border: 2px solid var(--c-surface-1);
}

.alpha::-moz-range-thumb {
  width: 18px;
  height: 18px;
  border-radius: var(--r-pill);
  background: var(--c-primary);
  border: 2px solid var(--c-surface-1);
}

.alpha-value {
  flex: none;
  width: 44px;
  text-align: end;
  font-variant-numeric: tabular-nums;
  color: var(--c-on-surface-muted);
}

.bg-row {
  gap: var(--s-3);
}

.thumb {
  flex: none;
  width: 56px;
  height: 56px;
  border-radius: var(--r-sm);
  overflow: hidden;
  background-size: cover, 24px 24px;
}

.thumb img {
  display: block;
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.bg-actions {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: var(--s-1);
  flex: none;
}

.font-row {
  gap: var(--s-3);
}

.font-actions {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: var(--s-1);
  min-width: 0;
  flex: none;
  max-width: 60%;
}

.font-name {
  max-width: 100%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--c-on-surface-muted);
  font-size: var(--t-small);
}

.font-buttons {
  display: flex;
  gap: var(--s-3);
}

.text-btn {
  display: inline-flex;
  align-items: center;
  color: var(--c-primary);
  font-size: var(--t-small);
  min-height: 32px;
  padding-inline: var(--s-1);
}

.text-btn.danger {
  color: var(--c-danger);
}

.text-btn:disabled {
  color: var(--c-on-surface-faint);
  cursor: default;
}

.error {
  margin-top: var(--s-3);
  font-size: var(--t-small);
  line-height: var(--t-small-lh);
  color: var(--c-danger);
  word-break: break-word;
}

.delete {
  width: 100%;
  min-height: 44px;
  margin-top: var(--s-4);
  border-radius: var(--r-md);
  border: 1px solid var(--c-hairline);
  background: var(--c-surface-1);
  color: var(--c-danger);
  font-size: var(--t-small);
}

.delete:disabled {
  color: var(--c-on-surface-faint);
  cursor: default;
}
</style>