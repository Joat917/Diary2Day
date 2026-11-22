<script setup lang="ts">
import { computed, onBeforeUnmount, ref } from "vue";
import { useRouter } from "vue-router";
import { mdiArrowLeft, mdiPencil, mdiPlus } from "@mdi/js";
import {
  activeCustomTheme,
  cloneCustomTheme,
  customScheme,
  DARK_THEMES,
  LIGHT_THEMES,
  onPrimaryColor,
  seedCustomTheme,
  slotRgba,
  themeOption,
  type ThemeOption,
} from "../constants/themes";
import type { CustomTheme, Settings, Theme } from "../ipc/types";
import { describeError, fontRemove, themeBgRemove } from "../ipc/client";
import { t } from "../i18n";
import { useSession } from "../stores/session";
import { themeBgUrl } from "../utils/assetUrl";
import Icon from "../components/Icon.vue";
import CustomThemeSheet from "../components/CustomThemeSheet.vue";

const router = useRouter();
const session = useSession();

const editingId = ref<string | null>(null);
const customError = ref<string | null>(null);

const customThemes = computed(() => session.settings.customThemes);
const customActive = computed(() => session.settings.theme === "custom");

function isCustomActive(theme: CustomTheme): boolean {
  return customActive.value && session.settings.customThemeId === theme.id;
}

function previewStyle(theme: CustomTheme): Record<string, string> {
  const background = themeBgUrl(theme.background);
  return { "--c-theme-bg": background ? `url("${background}")` : "none" };
}

function surfaceStyle(theme: CustomTheme): Record<string, string> {
  return { background: slotRgba(theme.surface) };
}

function customCardStyle(theme: CustomTheme) {
  const active = isCustomActive(theme);
  return {
    background: slotRgba(theme.card),
    borderColor: active ? theme.primary : slotRgba(theme.hairline),
    boxShadow: active ? `0 0 0 1px ${theme.primary}` : "none",
  };
}

function customChipStyle(theme: CustomTheme) {
  return isCustomActive(theme)
    ? { background: theme.primary, color: onPrimaryColor(theme.primary) }
    : { background: slotRgba(theme.inset), color: theme.primary };
}

// 新卡片照着当前已生效的主题画，外观与旁边那张卡一致，也预示会拷贝什么。
const newCardLook = computed(() => {
  const settings = session.settings;
  const custom = settings.theme === "custom" ? activeCustomTheme(settings) : null;
  if (custom) {
    return {
      checker: true,
      preview: previewStyle(custom),
      surface: slotRgba(custom.surface),
      card: slotRgba(custom.card),
      border: slotRgba(custom.hairline),
      title: custom.onSurface,
      text: custom.onSurfaceMuted,
      btnBg: custom.primary,
      btnColor: onPrimaryColor(custom.primary),
    };
  }
  const option = themeOption(settings.theme);
  return {
    checker: false,
    preview: {} as Record<string, string>,
    surface: option.preview.surface,
    card: option.preview.surface1,
    border: option.preview.hairline,
    title: option.preview.onSurface,
    text: option.preview.onSurfaceMuted,
    btnBg: option.preview.primary,
    btnColor: option.preview.onPrimary,
  };
});

async function createTheme() {
  customError.value = null;
  const settings = session.settings;
  const current = activeCustomTheme(settings);
  const next =
    settings.theme === "custom"
      ? current
        ? cloneCustomTheme(current)
        : seedCustomTheme(LIGHT_THEMES[0])
      : seedCustomTheme(themeOption(settings.theme));
  try {
    await session.patch({
      theme: "custom",
      customThemeId: next.id,
      customThemes: [...settings.customThemes, next],
    });
    editingId.value = next.id;
  } catch (err) {
    customError.value = describeError(err);
  }
}

async function pickCustom(theme: CustomTheme) {
  if (isCustomActive(theme)) return;
  customError.value = null;
  try {
    await session.patch({ theme: "custom", customThemeId: theme.id });
  } catch (err) {
    customError.value = describeError(err);
  }
}

const LONG_PRESS_MS = 500;
let pressTimer: number | null = null;
let longPressed = false;

function clearPress() {
  if (pressTimer !== null) {
    window.clearTimeout(pressTimer);
    pressTimer = null;
  }
}

function startPress(theme: CustomTheme, event: PointerEvent) {
  if (event.pointerType === "mouse" && event.button !== 0) return;
  longPressed = false;
  clearPress();
  pressTimer = window.setTimeout(() => {
    pressTimer = null;
    longPressed = true;
    void openSheet(theme);
  }, LONG_PRESS_MS);
}

function onCustomClick(theme: CustomTheme) {
  if (longPressed) {
    longPressed = false;
    return;
  }
  if (isCustomActive(theme)) void openSheet(theme);
  else void pickCustom(theme);
}

async function openSheet(theme: CustomTheme) {
  customError.value = null;
  try {
    if (!isCustomActive(theme)) {
      await session.patch({ theme: "custom", customThemeId: theme.id });
    }
    editingId.value = theme.id;
  } catch (err) {
    customError.value = describeError(err);
  }
}

function closeSheet() {
  editingId.value = null;
}

async function deleteTheme(id: string) {
  editingId.value = null;
  customError.value = null;
  const current = session.settings;
  const removed = current.customThemes.find((theme) => theme.id === id);
  if (!removed) return;
  const remaining = current.customThemes.filter((theme) => theme.id !== id);
  const background = removed.background;
  const fontName = removed.font?.name;
  const patch: Partial<Settings> = { customThemes: remaining };
  if (current.customThemeId === id) {
    if (remaining.length > 0) patch.customThemeId = remaining[0].id;
    else {
      patch.customThemeId = null;
      if (current.theme === "custom") {
        patch.theme = customScheme(removed) === "dark" ? "dark" : "light";
      }
    }
  }
  try {
    await session.patch(patch);
  } catch (err) {
    customError.value = describeError(err);
    return;
  }
  if (background && !remaining.some((theme) => theme.background === background)) {
    try {
      await themeBgRemove(background);
    } catch (err) {
      customError.value = describeError(err);
    }
  }
  if (fontName && !remaining.some((theme) => theme.font?.name === fontName)) {
    try {
      await fontRemove(fontName);
    } catch (err) {
      customError.value = describeError(err);
    }
  }
}

function isActive(theme: ThemeOption): boolean {
  return session.settings.theme === theme.id;
}

function cardStyle(theme: ThemeOption) {
  const active = isActive(theme);
  return {
    background: theme.preview.surface1,
    borderColor: active ? theme.preview.primary : theme.preview.hairline,
    boxShadow: active ? `0 0 0 1px ${theme.preview.primary}` : "none",
  };
}

function chipStyle(theme: ThemeOption) {
  const active = isActive(theme);
  return {
    background: active ? theme.preview.primary : theme.preview.primarySoft,
    color: active ? theme.preview.onPrimary : theme.preview.primary,
  };
}

async function pick(theme: Theme) {
  if (session.settings.theme === theme) return;
  await session.patch({ theme });
}

function goBack() {
  if (window.history.state?.back) router.back();
  else void router.push("/me");
}

onBeforeUnmount(clearPress);
</script>

<template>
  <div class="view">
    <header class="topbar">
      <button class="icon-btn" type="button" :aria-label="t('common.back')" @click="goBack">
        <Icon :path="mdiArrowLeft" :size="22" />
      </button>
      <h1 class="heading">{{ t("me.theme") }}</h1>
      <span class="spacer" />
    </header>

    <div class="scroll">
      <section class="group">
        <h2 class="group-title">{{ t("theme.light") }}</h2>
        <div class="grid">
          <button
            v-for="theme in LIGHT_THEMES"
            :key="theme.id"
            class="theme-card"
            type="button"
            :aria-pressed="isActive(theme)"
            :aria-label="t(theme.labelKey)"
            @click="pick(theme.id)"
          >
            <span
              class="preview"
              :style="{ background: theme.preview.surface }"
              aria-hidden="true"
            >
              <span class="pv-card" :style="cardStyle(theme)">
                <span class="pv-title" :style="{ color: theme.preview.onSurface }">
                  {{ t(theme.titleKey) }}
                </span>
                <span class="pv-text" :style="{ color: theme.preview.onSurfaceMuted }">
                  {{ t(theme.textKey) }}
                </span>
                <span class="pv-foot">
                  <span class="pv-chip" :style="chipStyle(theme)">
                    {{ t(theme.labelKey) }}
                  </span>
                  <span
                    class="pv-btn"
                    :style="{
                      background: theme.preview.primary,
                      color: theme.preview.onPrimary,
                    }"
                  >
                    <Icon :path="mdiPencil" :size="10" />
                  </span>
                </span>
              </span>
            </span>
          </button>
        </div>
      </section>

      <section class="group">
        <h2 class="group-title">{{ t("theme.dark") }}</h2>
        <div class="grid">
          <button
            v-for="theme in DARK_THEMES"
            :key="theme.id"
            class="theme-card"
            type="button"
            :aria-pressed="isActive(theme)"
            :aria-label="t(theme.labelKey)"
            @click="pick(theme.id)"
          >
            <span
              class="preview"
              :style="{ background: theme.preview.surface }"
              aria-hidden="true"
            >
              <span class="pv-card" :style="cardStyle(theme)">
                <span class="pv-title" :style="{ color: theme.preview.onSurface }">
                  {{ t(theme.titleKey) }}
                </span>
                <span class="pv-text" :style="{ color: theme.preview.onSurfaceMuted }">
                  {{ t(theme.textKey) }}
                </span>
                <span class="pv-foot">
                  <span class="pv-chip" :style="chipStyle(theme)">
                    {{ t(theme.labelKey) }}
                  </span>
                  <span
                    class="pv-btn"
                    :style="{
                      background: theme.preview.primary,
                      color: theme.preview.onPrimary,
                    }"
                  >
                    <Icon :path="mdiPencil" :size="10" />
                  </span>
                </span>
              </span>
            </span>
          </button>
        </div>
      </section>

      <section class="group">
        <h2 class="group-title">{{ t("theme.custom") }}</h2>
        <div class="grid">
          <div
            v-for="(theme, index) in customThemes"
            :key="theme.id"
            class="custom-item"
          >
            <button
              class="theme-card custom-card"
              type="button"
              :aria-pressed="isCustomActive(theme)"
              :aria-label="t('theme.customN', { n: index + 1 })"
              @pointerdown="startPress(theme, $event)"
              @pointerup="clearPress"
              @pointerleave="clearPress"
              @pointercancel="clearPress"
              @contextmenu.prevent
              @click="onCustomClick(theme)"
            >
              <span class="preview checker" :style="previewStyle(theme)" aria-hidden="true">
                <span class="pv-surface" :style="surfaceStyle(theme)" />
                <span class="pv-card" :style="customCardStyle(theme)">
                  <span class="pv-title" :style="{ color: theme.onSurface }">
                    {{ theme.sampleTitle ?? t("theme.customSample.title") }}
                  </span>
                  <span class="pv-text" :style="{ color: theme.onSurfaceMuted }">
                    {{ theme.sampleText ?? t("theme.customSample.text") }}
                  </span>
                  <span class="pv-foot">
                    <span class="pv-chip" :style="customChipStyle(theme)">
                      {{ t("theme.customN", { n: index + 1 }) }}
                    </span>
                    <span
                      class="pv-btn"
                      :style="{
                        background: theme.primary,
                        color: onPrimaryColor(theme.primary),
                      }"
                    >
                      <Icon :path="mdiPencil" :size="10" />
                    </span>
                  </span>
                </span>
              </span>
            </button>
          </div>
          <button
            class="theme-card"
            type="button"
            :aria-label="t('theme.newCard.title')"
            @click="createTheme"
          >
            <span
              class="preview"
              :class="{ checker: newCardLook.checker }"
              :style="newCardLook.checker ? newCardLook.preview : { background: newCardLook.surface }"
              aria-hidden="true"
            >
              <span
                v-if="newCardLook.checker"
                class="pv-surface"
                :style="{ background: newCardLook.surface }"
              />
              <span
                class="pv-card"
                :style="{ background: newCardLook.card, borderColor: newCardLook.border }"
              >
                <span class="pv-title" :style="{ color: newCardLook.title }">
                  {{ t("theme.newCard.title") }}
                </span>
                <span class="pv-text" :style="{ color: newCardLook.text }">
                  {{ t("theme.newCard.text") }}
                </span>
                <span class="pv-foot new-foot">
                  <span
                    class="pv-btn"
                    :style="{ background: newCardLook.btnBg, color: newCardLook.btnColor }"
                  >
                    <Icon :path="mdiPlus" :size="10" />
                  </span>
                </span>
              </span>
            </span>
          </button>
        </div>
        <p v-if="customError" class="error">{{ customError }}</p>
      </section>
    </div>

    <CustomThemeSheet
      v-if="editingId"
      :theme-id="editingId"
      @close="closeSheet"
      @delete="deleteTheme"
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
  padding: var(--s-4) var(--s-4) var(--s-8);
}

.group {
  margin-top: var(--s-5);
}

.group:first-child {
  margin-top: 0;
}

.group-title {
  font-size: var(--t-small);
  line-height: var(--t-small-lh);
  color: var(--c-on-surface-muted);
  font-weight: 500;
  letter-spacing: 0.04em;
  margin-bottom: var(--s-2);
}

.grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(148px, 1fr));
  gap: var(--s-3);
}

.theme-card {
  display: block;
  padding: 0;
  border: 0;
  background: none;
  border-radius: var(--r-md);
  text-align: start;
}

.custom-item {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: var(--s-1);
  min-width: 0;
}

.custom-item .theme-card {
  width: 100%;
}

.custom-card {
  user-select: none;
  -webkit-touch-callout: none;
}

.pv-foot.new-foot {
  justify-content: flex-end;
}

.preview {
  display: block;
  padding: var(--s-2);
  border-radius: var(--r-md);
}

.preview.checker {
  position: relative;
}

.pv-surface {
  position: absolute;
  inset: 0;
  z-index: 0;
  border-radius: inherit;
}

.pv-card {
  position: relative;
  z-index: 1;
  display: flex;
  flex-direction: column;
  gap: 4px;
  height: 96px;
  padding: var(--s-2);
  border-radius: var(--r-sm);
  border: 1px solid transparent;
}

.pv-title {
  font-size: 12.5px;
  line-height: 1.35;
  font-weight: 600;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.pv-text {
  font-size: 10.5px;
  line-height: 1.45;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.pv-foot {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 4px;
  margin-top: auto;
  min-width: 0;
}

.pv-chip {
  padding: 2px 6px;
  border-radius: var(--r-pill);
  font-size: 10px;
  line-height: 1.4;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.pv-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 18px;
  height: 18px;
  border-radius: var(--r-pill);
  flex: none;
}

.error {
  margin-top: var(--s-2);
  font-size: var(--t-small);
  line-height: var(--t-small-lh);
  color: var(--c-danger);
  word-break: break-word;
}
</style>