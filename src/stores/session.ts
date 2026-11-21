import { defineStore } from "pinia";
import { ref } from "vue";
import { settingsGetAll, settingsPatch } from "../ipc/client";
import type { Settings } from "../ipc/types";
import { setLocale } from "../i18n";

const defaults: Settings = {
  theme: "system",
  language: "zh-CN",
  dateFormat: "slash",
  weekStart: 1,
  indentFirstLine: false,
  imageLazy: true,
  showStatsCard: true,
  quickRecord: true,
  lockLeaveDelayS: 60,
  reduceMotion: false,
  dayBoundaryMinutes: 0,
};

export const useSession = defineStore("session", () => {
  const settings = ref<Settings>({ ...defaults });
  const ready = ref(false);
  const media = window.matchMedia("(prefers-color-scheme: dark)");

  function applyTheme() {
    const theme =
      settings.value.theme === "system"
        ? media.matches
          ? "dark"
          : "light"
        : settings.value.theme;
    document.documentElement.dataset.theme = theme;
    document.documentElement.dataset.motion = settings.value.reduceMotion ? "reduced" : "full";
    setLocale(settings.value.language);
  }

  media.addEventListener("change", () => {
    if (settings.value.theme === "system") applyTheme();
  });

  async function load() {
    try {
      settings.value = await settingsGetAll();
    } catch {
      settings.value = { ...defaults };
    } finally {
      ready.value = true;
      applyTheme();
    }
  }

  async function patch(next: Partial<Settings>) {
    settings.value = await settingsPatch(next);
    applyTheme();
  }

  return { settings, ready, load, patch, applyTheme };
});