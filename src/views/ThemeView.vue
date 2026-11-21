<script setup lang="ts">
import { useRouter } from "vue-router";
import { mdiArrowLeft, mdiPencil } from "@mdi/js";
import { DARK_THEMES, LIGHT_THEMES, type ThemeOption } from "../constants/themes";
import type { Theme } from "../ipc/types";
import { t } from "../i18n";
import { useSession } from "../stores/session";
import Icon from "../components/Icon.vue";

const router = useRouter();
const session = useSession();

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
    </div>
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

.pv-card {
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
</style>