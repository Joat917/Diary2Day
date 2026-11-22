<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { mdiArrowLeft } from "@mdi/js";
import { describeError, themeBgImport, themeBgRemove } from "../ipc/client";
import type { CustomTheme, ThemeBase } from "../ipc/types";
import { t, type MessageKey } from "../i18n";
import {
  customBaseOption,
  defaultCustomTheme,
  slotRgba,
} from "../constants/themes";
import { themeBgUrl } from "../utils/assetUrl";
import { useBackLayer } from "../composables/useBackLayer";
import { useSession } from "../stores/session";
import Icon from "../components/Icon.vue";

type ModuleKey = "surface" | "card" | "inset" | "scrim";

const emit = defineEmits<{ close: [] }>();
const session = useSession();

const MODULES: { key: ModuleKey; label: MessageKey }[] = [
  { key: "surface", label: "theme.module.surface" },
  { key: "card", label: "theme.module.card" },
  { key: "inset", label: "theme.module.inset" },
  { key: "scrim", label: "theme.module.scrim" },
];

const BASES: { base: ThemeBase; label: MessageKey }[] = [
  { base: "light", label: "theme.light" },
  { base: "dark", label: "theme.dark" },
];

function cloneTheme(source: CustomTheme): CustomTheme {
  return {
    base: source.base,
    surface: { ...source.surface },
    card: { ...source.card },
    inset: { ...source.inset },
    scrim: { ...source.scrim },
    background: source.background,
  };
}

const draft = ref<CustomTheme>(
  cloneTheme(session.settings.customTheme ?? defaultCustomTheme("light")),
);
const base = computed(() => customBaseOption(draft.value.base).preview);
const busy = ref(false);
const error = ref<string | null>(null);
const backgroundBroken = ref(false);
const backgroundUrl = computed(() => themeBgUrl(draft.value.background));

const demoStyle = computed<Record<string, string>>(() => ({
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
  const snapshot = cloneTheme(next);
  const task = chain.then(() => session.patch({ customTheme: snapshot }));
  chain = task.catch(() => undefined);
  return task.then(() => undefined);
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

async function setBase(next: ThemeBase) {
  if (draft.value.base === next) return;
  draft.value.base = next;
  preview();
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
    if (previous && previous !== imported.name) {
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
    try {
      await themeBgRemove(previous);
    } catch (err) {
      error.value = describeError(err);
    }
  } catch (err) {
    error.value = describeError(err);
  } finally {
    busy.value = false;
  }
}

async function reset() {
  if (busy.value) return;
  const previous = draft.value.background;
  busy.value = true;
  error.value = null;
  draft.value = defaultCustomTheme(draft.value.base);
  preview();
  try {
    await queuePatch(draft.value);
    if (previous) {
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

function onBackgroundError(event: Event) {
  backgroundBroken.value = true;
  const img = event.target as HTMLImageElement | null;
  console.warn("[theme] background image failed to load", img?.src);
}

function close() {
  emit("close");
}

useBackLayer(() => true, close);

function onKeydown(event: KeyboardEvent) {
  if (event.key === "Escape") {
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
    <div class="sheet" role="dialog" aria-modal="true" :aria-label="t('theme.sheetTitle')">
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
        <div class="demo checker" :style="demoStyle" aria-hidden="true">
          <div class="demo-shell" :style="{ background: slotRgba(draft.surface) }" />
          <div
            class="demo-card"
            :style="{
              background: slotRgba(draft.card),
              borderColor: base.hairline,
            }"
          >
            <span class="demo-title" :style="{ color: base.onSurface }">
              {{ t("theme.customSample.title") }}
            </span>
            <span class="demo-text" :style="{ color: base.onSurfaceMuted }">
              {{ t("theme.customSample.text") }}
            </span>
            <span
              class="demo-chip"
              :style="{ background: slotRgba(draft.inset), color: base.primary }"
            >
              {{ t("theme.custom") }}
            </span>
          </div>
        </div>

        <section class="panel">
          <div class="row">
            <span class="row-label">{{ t("theme.base") }}</span>
            <div class="segmented" role="group" :aria-label="t('theme.base')">
              <button
                v-for="option in BASES"
                :key="option.base"
                class="segment"
                :class="{ on: draft.base === option.base }"
                type="button"
                :aria-pressed="draft.base === option.base"
                @click="setBase(option.base)"
              >
                {{ t(option.label) }}
              </button>
            </div>
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

        <p v-if="error" class="error">{{ error }}</p>

        <button class="reset" type="button" :disabled="busy" @click="reset">
          {{ t("theme.reset") }}
        </button>
      </div>
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
  background: var(--c-surface);
  padding-top: var(--safe-top);
  padding-inline: var(--safe-left) var(--safe-right);
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

.demo {
  position: relative;
  height: 132px;
  border-radius: var(--r-md);
  overflow: hidden;
}

.demo-shell {
  position: absolute;
  inset: 0;
}

.demo-card {
  position: relative;
  margin: var(--s-3);
  max-width: 220px;
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: var(--s-3);
  border: 1px solid;
  border-radius: var(--r-sm);
}

.demo-title {
  font-size: var(--t-small);
  line-height: var(--t-small-lh);
  font-weight: 600;
}

.demo-text {
  font-size: var(--t-caption);
  line-height: var(--t-caption-lh);
}

.demo-chip {
  align-self: flex-start;
  margin-top: var(--s-1);
  padding: 2px 6px;
  border-radius: var(--r-pill);
  font-size: var(--t-caption);
  line-height: var(--t-caption-lh);
}

.panel {
  margin-top: var(--s-4);
  border-radius: var(--r-md);
  background: var(--c-surface-1);
  border: 1px solid var(--c-hairline);
  overflow: hidden;
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

.segmented {
  display: inline-flex;
  padding: 2px;
  border-radius: var(--r-pill);
  background: var(--c-surface-2);
  flex: none;
}

.segment {
  min-height: 34px;
  padding-inline: var(--s-4);
  border-radius: var(--r-pill);
  font-size: var(--t-small);
  color: var(--c-on-surface-muted);
}

.segment.on {
  background: var(--c-primary);
  color: var(--c-on-primary);
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

.reset {
  width: 100%;
  min-height: 44px;
  margin-top: var(--s-4);
  border-radius: var(--r-md);
  border: 1px solid var(--c-hairline);
  background: var(--c-surface-1);
  color: var(--c-danger);
  font-size: var(--t-small);
}

.reset:disabled {
  color: var(--c-on-surface-faint);
  cursor: default;
}
</style>