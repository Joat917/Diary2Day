import { defineStore } from "pinia";
import { ref } from "vue";
import { settingsGetAll, settingsPatch } from "../ipc/client";
import type { Settings, Theme } from "../ipc/types";
import { isDarkTheme } from "../constants/themes";
import { setLocale } from "../i18n";

const defaults: Settings = {
  theme: "light",
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
  cardTime: "recorded",
  autoEdit: true,
  exportWidth: 640,
};

export const useSession = defineStore("session", () => {
  const settings = ref<Settings>({ ...defaults });
  const ready = ref(false);
  const media = window.matchMedia("(prefers-color-scheme: dark)");

  function autoTheme(): Theme {
    return media.matches ? "dark" : "light";
  }

  // 库里的 "system" 只是首次启动的默认值：按系统明暗落成浅色/深色第一套后写回。
  function normalized(next: Settings): Settings {
    return (next.theme as string) === "system" ? { ...next, theme: autoTheme() } : next;
  }

  function applyTheme() {
    const theme = settings.value.theme;
    document.documentElement.dataset.theme = theme;
    document.documentElement.dataset.scheme = isDarkTheme(theme) ? "dark" : "light";
    document.documentElement.dataset.motion = settings.value.reduceMotion ? "reduced" : "full";
    setLocale(settings.value.language);
  }

  async function load() {
    let firstRun = false;
    try {
      const stored = await settingsGetAll();
      firstRun = (stored.theme as string) === "system";
      settings.value = normalized(stored);
    } catch (error) {
      console.error("[session] settings load failed", error);
      settings.value = { ...defaults, theme: autoTheme() };
    } finally {
      ready.value = true;
      applyTheme();
    }
    if (firstRun) {
      try {
        settings.value = normalized(await settingsPatch({ theme: settings.value.theme }));
      } catch (error) {
        console.error("[session] theme persist failed", error);
      }
    }
  }

  async function patch(next: Partial<Settings>) {
    settings.value = normalized(await settingsPatch(next));
    applyTheme();
  }

  return { settings, ready, load, patch, applyTheme };
});