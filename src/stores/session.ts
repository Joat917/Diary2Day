import { defineStore } from "pinia";
import { ref } from "vue";
import { settingsGetAll, settingsPatch } from "../ipc/client";
import type { CustomTheme, Settings, Theme, ThemeSlot, UiFont } from "../ipc/types";
import {
  activeCustomTheme,
  customScheme,
  isDarkTheme,
  onPrimaryColor,
  slotRgba,
} from "../constants/themes";
import { fontFileUrl, themeBgUrl } from "../utils/assetUrl";
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
  customThemes: [],
  customThemeId: null,
};

const FONT_STYLE_ID = "dd-ui-font";
const MODULE_VARS = ["--c-surface", "--c-surface-1", "--c-surface-2", "--c-scrim"] as const;
const TEXT_VARS = [
  "--c-on-surface",
  "--c-on-surface-muted",
  "--c-primary",
  "--c-hairline",
  "--c-on-primary",
] as const;

let baseFontBody: string | null = null;
let probedBackground: string | null = null;

// CSS 背景图加载失败不会抛错，这里探一次，缺失文件时留下带源地址的告警。
function probeBackground(url: string, name: string) {
  if (probedBackground === url) return;
  probedBackground = url;
  const probe = new Image();
  probe.onerror = () => {
    console.warn("[theme] background image failed to load", name, url);
  };
  probe.src = url;
}

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

  function applyCustomVars(custom: CustomTheme | null) {
    const style = document.documentElement.style;
    if (!custom) {
      for (const name of MODULE_VARS) {
        style.removeProperty(name);
        style.removeProperty(`${name}-flat`);
      }
      for (const name of TEXT_VARS) style.removeProperty(name);
      style.removeProperty("--c-theme-bg");
      return;
    }
    const slots: [string, ThemeSlot][] = [
      ["--c-surface", custom.surface],
      ["--c-surface-1", custom.card],
      ["--c-surface-2", custom.inset],
      ["--c-scrim", custom.scrim],
    ];
    for (const [name, slot] of slots) {
      style.setProperty(name, slotRgba(slot));
      style.setProperty(`${name}-flat`, slot.color);
    }
    style.setProperty("--c-on-surface", custom.onSurface);
    style.setProperty("--c-on-surface-muted", custom.onSurfaceMuted);
    style.setProperty("--c-primary", custom.primary);
    style.setProperty("--c-hairline", custom.hairline);
    style.setProperty("--c-on-primary", onPrimaryColor(custom.primary));
    const background = themeBgUrl(custom.background);
    if (background) {
      style.setProperty("--c-theme-bg", `url("${background}")`);
      probeBackground(background, custom.background ?? "");
    } else {
      style.removeProperty("--c-theme-bg");
    }
  }

  function applyFont(font: UiFont | null) {
    const root = document.documentElement;
    let element = document.getElementById(FONT_STYLE_ID) as HTMLStyleElement | null;
    if (!font) {
      element?.remove();
      root.style.removeProperty("--font-body");
      return;
    }
    // 资产目录尚未就绪时先跳过，setAssetBase 之后会再跑一次 applyTheme。
    const url = fontFileUrl(font.name);
    if (!url) return;
    if (baseFontBody === null) {
      baseFontBody =
        getComputedStyle(root).getPropertyValue("--font-body").trim() ||
        "system-ui, sans-serif";
    }
    if (!element) {
      element = document.createElement("style");
      element.id = FONT_STYLE_ID;
      document.head.append(element);
    }
    element.textContent = `@font-face{font-family:"DD-UI";src:url("${url}");font-display:swap;}`;
    root.style.setProperty("--font-body", `"DD-UI", ${baseFontBody}`);
    document.fonts.load('1em "DD-UI"').catch((error: unknown) => {
      console.error("[font] custom UI font failed to load", url, error);
    });
  }

  function applyTheme() {
    const current = settings.value;
    const theme = current.theme;
    const custom = theme === "custom" ? activeCustomTheme(current) : null;
    document.documentElement.dataset.theme = theme;
    document.documentElement.dataset.scheme = custom
      ? customScheme(custom)
      : isDarkTheme(theme)
        ? "dark"
        : "light";
    document.documentElement.dataset.motion = current.reduceMotion ? "reduced" : "full";
    applyCustomVars(custom);
    applyFont(custom?.font ?? null);
    setLocale(current.language);
  }

  // 编辑层拖动中的临时外观：只改根节点，不写库；关闭编辑层后再 applyTheme() 回到已存值。
  function previewCustom(custom: CustomTheme) {
    document.documentElement.dataset.scheme = customScheme(custom);
    applyCustomVars(custom);
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

  return { settings, ready, load, patch, applyTheme, previewCustom };
});