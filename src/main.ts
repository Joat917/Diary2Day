import { createApp } from "vue";
import { createPinia } from "pinia";
import "vditor/dist/index.css";
import "./styles/tokens.css";
import "./styles/base.css";
import "./styles/vditor-override.css";
import App from "./App.vue";
import router from "./router";
import { assetPaths, isTauri } from "./ipc/client";
import { useLock } from "./stores/lock";
import { useSession } from "./stores/session";
import { setAssetBase } from "./utils/assetUrl";

declare global {
  interface Window {
    ddApplyInsets?: (top: number, bottom: number) => void;
    ddInsets?: {
      read: () => string;
    };
  }
}

function applySafeInsets(top: number, bottom: number) {
  const style = document.documentElement.style;
  style.setProperty("--safe-top", `${Math.max(0, top)}px`);
  style.setProperty("--safe-bottom", `${Math.max(0, bottom)}px`);
}

function initAndroidInsets() {
  const bridge = window.ddInsets;
  if (!bridge) return;
  window.ddApplyInsets = applySafeInsets;
  try {
    const values = bridge.read().split(",");
    applySafeInsets(Number.parseFloat(values[0]) || 0, Number.parseFloat(values[1]) || 0);
  } catch (error) {
    console.error("[insets] initial read failed", error);
  }
}

initAndroidInsets();

const app = createApp(App);
const pinia = createPinia();
app.use(pinia);
app.use(router);

const session = useSession(pinia);
const lock = useLock(pinia);

void (async () => {
  try {
    await session.load();
    await lock.load();
    if (isTauri()) {
      const paths = await assetPaths();
      setAssetBase(paths.assets_dir);
    }
  } catch (error) {
    console.error("[boot] startup failed", error);
  } finally {
    app.mount("#app");
  }
})();