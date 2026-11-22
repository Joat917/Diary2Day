import { createApp } from "vue";
import { createPinia } from "pinia";
import "vditor/dist/index.css";
import "./styles/tokens.css";
import "./styles/base.css";
import "./styles/vditor-override.css";
import App from "./App.vue";
import router from "./router";
import { handleBackLayer } from "./composables/useBackLayer";
import { assetPaths, isTauri } from "./ipc/client";
import { useLock } from "./stores/lock";
import { useSession } from "./stores/session";
import { setAssetBase } from "./utils/assetUrl";

declare global {
  interface Window {
    ddApplyInsets?: (
      top: number,
      right: number,
      bottom: number,
      left: number,
      ime: number,
    ) => void;
    ddBack?: () => boolean;
    ddInsets?: {
      read: () => string;
      setDark: (dark: boolean) => void;
    };
  }
}

function revealActiveControl() {
  requestAnimationFrame(() => {
    const active = document.activeElement;
    if (!(active instanceof HTMLElement) || active === document.body) return;
    let target: HTMLElement = active;
    if (active.isContentEditable) {
      const node = window.getSelection()?.anchorNode ?? null;
      const element = node instanceof HTMLElement ? node : node?.parentElement;
      if (element) target = element;
    }
    target.scrollIntoView({ block: "nearest", inline: "nearest" });
  });
}

function applySafeInsets(top: number, right: number, bottom: number, left: number, ime: number) {
  const root = document.documentElement;
  const style = root.style;
  const px = (value: number) => `${Math.max(0, value || 0)}px`;
  style.setProperty("--safe-top", px(top));
  style.setProperty("--safe-right", px(right));
  style.setProperty("--safe-bottom", px(bottom));
  style.setProperty("--safe-left", px(left));
  style.setProperty("--ime", px(ime));
  const keyboardOpen = (ime || 0) > 1;
  root.dataset.keyboard = keyboardOpen ? "1" : "0";
  if (keyboardOpen) revealActiveControl();
}

function syncSystemBarAppearance() {
  const scheme = document.documentElement.dataset.scheme;
  if (!scheme) return;
  window.ddInsets?.setDark(scheme === "dark");
}

const app = createApp(App);
const pinia = createPinia();
app.use(pinia);
app.use(router);

const session = useSession(pinia);
const lock = useLock(pinia);

function initAndroidInsets() {
  const bridge = window.ddInsets;
  if (!bridge) return;
  window.ddApplyInsets = applySafeInsets;
  window.ddBack = () => {
    if (lock.locked) return false;
    if (handleBackLayer()) return true;
    if (router.options.history.state.back) {
      void router.back();
      return true;
    }
    return false;
  };
  try {
    const values = bridge.read().split(",");
    const value = (index: number) => Number.parseFloat(values[index]) || 0;
    applySafeInsets(value(0), value(1), value(2), value(3), value(4));
  } catch (error) {
    console.error("[insets] initial read failed", error);
  }
  new MutationObserver(syncSystemBarAppearance).observe(document.documentElement, {
    attributes: true,
    attributeFilter: ["data-scheme"],
  });
  syncSystemBarAppearance();
}

initAndroidInsets();

void (async () => {
  try {
    await session.load();
    await lock.load();
    if (isTauri()) {
      const paths = await assetPaths();
      setAssetBase(paths.assets_dir);
      // 自定义背景图与字体依赖资产地址，拿到目录后重跑一次外观。
      session.applyTheme();
    }
  } catch (error) {
    console.error("[boot] startup failed", error);
  } finally {
    app.mount("#app");
  }
})();