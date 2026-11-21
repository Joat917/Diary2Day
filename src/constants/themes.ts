import type { MessageKey } from "../i18n";
import type { Theme } from "../ipc/types";

export interface ThemePreview {
  surface: string;
  surface1: string;
  onSurface: string;
  onSurfaceMuted: string;
  primary: string;
  onPrimary: string;
  primarySoft: string;
  hairline: string;
}

export interface ThemeOption {
  id: Theme;
  labelKey: MessageKey;
  titleKey: MessageKey;
  textKey: MessageKey;
  dark: boolean;
  preview: ThemePreview;
}

export const LIGHT_THEMES: ThemeOption[] = [
  {
    id: "light",
    labelKey: "me.theme.light",
    titleKey: "theme.sample.light.title",
    textKey: "theme.sample.light.text",
    dark: false,
    preview: {
      surface: "#f6f7f6",
      surface1: "#ffffff",
      onSurface: "#17181a",
      onSurfaceMuted: "#5f6467",
      primary: "#17695c",
      onPrimary: "#ffffff",
      primarySoft: "#e7f0ed",
      hairline: "#e4e7e6",
    },
  },
  {
    id: "warm",
    labelKey: "me.theme.warm",
    titleKey: "theme.sample.warm.title",
    textKey: "theme.sample.warm.text",
    dark: false,
    preview: {
      surface: "#faf6ef",
      surface1: "#fffdf8",
      onSurface: "#211b14",
      onSurfaceMuted: "#6b6155",
      primary: "#9a5b17",
      onPrimary: "#ffffff",
      primarySoft: "#f4e7d3",
      hairline: "#e9e0d1",
    },
  },
  {
    id: "mist",
    labelKey: "me.theme.mist",
    titleKey: "theme.sample.mist.title",
    textKey: "theme.sample.mist.text",
    dark: false,
    preview: {
      surface: "#f2f6f9",
      surface1: "#ffffff",
      onSurface: "#17202a",
      onSurfaceMuted: "#5a6875",
      primary: "#2f6491",
      onPrimary: "#ffffff",
      primarySoft: "#e1eef8",
      hairline: "#dfe7ee",
    },
  },
  {
    id: "blossom",
    labelKey: "me.theme.blossom",
    titleKey: "theme.sample.blossom.title",
    textKey: "theme.sample.blossom.text",
    dark: false,
    preview: {
      surface: "#fbf4f5",
      surface1: "#fffdfd",
      onSurface: "#221a1c",
      onSurfaceMuted: "#6d5c60",
      primary: "#9c475c",
      onPrimary: "#ffffff",
      primarySoft: "#f6e2e8",
      hairline: "#eedde1",
    },
  },
];

export const DARK_THEMES: ThemeOption[] = [
  {
    id: "dark",
    labelKey: "me.theme.dark",
    titleKey: "theme.sample.dark.title",
    textKey: "theme.sample.dark.text",
    dark: true,
    preview: {
      surface: "#101211",
      surface1: "#191c1b",
      onSurface: "#eceeed",
      onSurfaceMuted: "#9ba29f",
      primary: "#5fc9b4",
      onPrimary: "#052b24",
      primarySoft: "#1b3a34",
      hairline: "#2a2f2d",
    },
  },
  {
    id: "midnight",
    labelKey: "me.theme.midnight",
    titleKey: "theme.sample.midnight.title",
    textKey: "theme.sample.midnight.text",
    dark: true,
    preview: {
      surface: "#0d1219",
      surface1: "#151c25",
      onSurface: "#e7edf5",
      onSurfaceMuted: "#93a2b4",
      primary: "#7fb0e8",
      onPrimary: "#07182b",
      primarySoft: "#1b2d44",
      hairline: "#26313d",
    },
  },
  {
    id: "mocha",
    labelKey: "me.theme.mocha",
    titleKey: "theme.sample.mocha.title",
    textKey: "theme.sample.mocha.text",
    dark: true,
    preview: {
      surface: "#16110e",
      surface1: "#1f1814",
      onSurface: "#f0e8df",
      onSurfaceMuted: "#ab9c8d",
      primary: "#d9a05f",
      onPrimary: "#2a1a07",
      primarySoft: "#3a2a1a",
      hairline: "#312720",
    },
  },
  {
    id: "forest",
    labelKey: "me.theme.forest",
    titleKey: "theme.sample.forest.title",
    textKey: "theme.sample.forest.text",
    dark: true,
    preview: {
      surface: "#0e1410",
      surface1: "#161e18",
      onSurface: "#e6efe8",
      onSurfaceMuted: "#96a79a",
      primary: "#7fc79a",
      onPrimary: "#08210f",
      primarySoft: "#1c3a27",
      hairline: "#26332a",
    },
  },
];

export const THEMES: ThemeOption[] = [...LIGHT_THEMES, ...DARK_THEMES];

const BY_ID = new Map(THEMES.map((theme) => [theme.id, theme]));

export function themeOption(id: Theme): ThemeOption {
  return BY_ID.get(id) ?? LIGHT_THEMES[0];
}

export function isDarkTheme(id: Theme): boolean {
  return BY_ID.get(id)?.dark === true;
}