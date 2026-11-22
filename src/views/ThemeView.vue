<script setup lang="ts">
import { computed, ref } from "vue";
import { useRouter } from "vue-router";
import { mdiArrowLeft, mdiPencil } from "@mdi/js";
import {
  customBaseOption,
  DARK_THEMES,
  LIGHT_THEMES,
  seedCustomTheme,
  slotRgba,
  themeOption,
  type ThemeOption,
} from "../constants/themes";
import type { CustomTheme, Theme } from "../ipc/types";
import { describeError } from "../ipc/client";
import { t } from "../i18n";
import { useSession } from "../stores/session";
import { themeBgUrl } from "../utils/assetUrl";
import Icon from "../components/Icon.vue";
import CustomThemeSheet from "../components/CustomThemeSheet.vue";

const router = useRouter();
const session = useSession();

const sheetOpen = ref(false);
const customError = ref<string | null>(null);

const customActive = computed(() => session.settings.theme === "custom");
const custom = computed<CustomTheme>(
  () =>
    session.settings.customTheme ??
    seedCustomTheme(themeOption(session.settings.theme)),
);
const customBase = computed(() => customBaseOption(custom.value.base).preview);
const customBackground = computed(() => themeBgUrl(custom.value.background));

function previewStyle(): Record<string, string> {
  return {
    "--c-theme-bg": customBackground.value
      ? `url("${customBackground.value}")`
      : "none",
  };
}

function surfaceStyle(): Record<string, string> {
  return { background: slotRgba(custom.value.surface) };
}

function customCardStyle(): Record<string, string> {
  const base = customBase.value;
  return {
    background: slotRgba(custom.value.card),
    borderColor: customActive.value ? base.primary : base.hairline,
    boxShadow: customActive.value ? `0 0 0 1px ${base.primary}` : "none",
  };
}

function customChipStyle(): Record<string, string> {
  const base = customBase.value;
  return customActive.value
    ? { background: base.primary, color: base.onPrimary }
    : { background: slotRgba(custom.value.inset), color: base.primary };
}

async function pickCustom() {
  if (customActive.value) return;
  customError.value = null;
  try {
    const next =
      session.settings.customTheme ??
      seedCustomTheme(themeOption(session.settings.theme));
    await session.patch({ theme: "custom", customTheme: next });
  } catch (err) {
    customError.value = describeError(err);
  }
}

async function openSheet() {
  customError.value = null;
  try {
    if (!customActive.value || !session.settings.customTheme) {
      const next =
        session.settings.customTheme ??
        seedCustomTheme(themeOption(session.settings.theme));
      await session.patch({ theme: "custom", customTheme: next });
    }
    sheetOpen.value = true;
  } catch (err) {
    customError.value = describeError(err);
  }
}

function closeSheet() {
  sheetOpen.value = false;
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
        <div class="custom-row">
          <button
            class="theme-card custom-card"
            type="button"
            :aria-pressed="customActive"
            :aria-label="t('theme.custom')"
            @click="pickCustom"
          >
            <span class="preview checker" :style="previewStyle()" aria-hidden="true">
              <span class="pv-surface" :style="surfaceStyle()" />
              <span class="pv-card" :style="customCardStyle()">
                <span
                  class="pv-title"
                  :style="{ color: customBase.onSurface }"
                >
                  {{ t("theme.customSample.title") }}
                </span>
                <span
                  class="pv-text"
                  :style="{ color: customBase.onSurfaceMuted }"
                >
                  {{ t("theme.customSample.text") }}
                </span>
                <span class="pv-foot">
                  <span class="pv-chip" :style="customChipStyle()">
                    {{ t("theme.custom") }}
                  </span>
                  <span
                    class="pv-btn"
                    :style="{
                      background: customBase.primary,
                      color: customBase.onPrimary,
                    }"
                  >
                    <Icon :path="mdiPencil" :size="10" />
                  </span>
                </span>
              </span>
            </span>
          </button>
          <button class="text-btn" type="button" @click="openSheet">
            {{ t("theme.edit") }}
          </button>
        </div>
        <p v-if="customError" class="error">{{ customError }}</p>
      </section>
    </div>

    <CustomThemeSheet v-if="sheetOpen" @close="closeSheet" />
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

.custom-row {
  display: flex;
  align-items: center;
  gap: var(--s-3);
}

.custom-card {
  width: 180px;
  flex: none;
}

.text-btn {
  display: inline-flex;
  align-items: center;
  color: var(--c-primary);
  font-size: var(--t-small);
  min-height: 36px;
  padding-inline: var(--s-1);
}

.error {
  margin-top: var(--s-2);
  font-size: var(--t-small);
  line-height: var(--t-small-lh);
  color: var(--c-danger);
  word-break: break-word;
}
</style>