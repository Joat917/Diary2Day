<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { RouterLink, RouterView, useRoute, useRouter } from "vue-router";
import { mdiAccountCircleOutline, mdiCalendarToday, mdiMagnify } from "@mdi/js";
import { useBreakpoint } from "./composables/useBreakpoint";
import { t, type MessageKey } from "./i18n";
import { version as appVersion } from "../package.json";
import { isTauri, shareDrain } from "./ipc/client";
import { useLock } from "./stores/lock";
import { useSession } from "./stores/session";
import Icon from "./components/Icon.vue";
import LockOverlay from "./components/LockOverlay.vue";

const route = useRoute();
const router = useRouter();
const { isWide, isThreePane } = useBreakpoint();
const broad = computed(() => isThreePane.value && route.path.startsWith("/today"));
const lock = useLock();
const session = useSession();

let lockTimer: number | undefined;
let shareTimer: number | undefined;
let unlistenLock: (() => void) | null = null;
let shareBusy = false;

async function drainShares() {
  if (!isTauri() || shareBusy || lock.locked) return;
  if (document.visibilityState === "hidden") return;
  shareBusy = true;
  try {
    const report = await shareDrain();
    if (report) await router.push(`/j/${report.id}`);
  } catch {
    // 锁定或暂时失败，等下次聚焦 / 解锁再试
  } finally {
    shareBusy = false;
  }
}

function scheduleLock() {
  window.clearTimeout(lockTimer);
  if (!lock.enabled || lock.locked) return;
  const delay = session.settings.lockLeaveDelayS;
  if (delay <= 0) return;
  lockTimer = window.setTimeout(() => {
    void lock.engage();
  }, delay * 1000);
}

function cancelLock() {
  window.clearTimeout(lockTimer);
}

function onVisibilityChange() {
  if (document.visibilityState === "hidden") {
    scheduleLock();
    return;
  }
  cancelLock();
  void drainShares();
}

function onFocus() {
  cancelLock();
  void drainShares();
}

const viewEpoch = ref(0);
// Locked startup must not mount views: their initial IPC calls are lock-gated.
const viewsReady = ref(!lock.locked);

watch(
  () => lock.locked,
  (locked, wasLocked) => {
    if (!locked) viewsReady.value = true;
    if (wasLocked && !locked) viewEpoch.value += 1;
    if (!locked) void drainShares();
  },
);

onMounted(async () => {
  unlistenLock = await lock.listen();
  window.addEventListener("blur", scheduleLock);
  window.addEventListener("focus", onFocus);
  document.addEventListener("visibilitychange", onVisibilityChange);
  void drainShares();
  if (/Android/i.test(navigator.userAgent)) {
    shareTimer = window.setInterval(() => void drainShares(), 1500);
  }
});

onBeforeUnmount(() => {
  unlistenLock?.();
  window.clearTimeout(lockTimer);
  window.clearInterval(shareTimer);
  window.removeEventListener("blur", scheduleLock);
  window.removeEventListener("focus", onFocus);
  document.removeEventListener("visibilitychange", onVisibilityChange);
});

const navItems: { to: string; label: MessageKey; icon: string }[] = [
  { to: "/today", label: "nav.today", icon: mdiCalendarToday },
  { to: "/discover", label: "nav.discover", icon: mdiMagnify },
  { to: "/me", label: "nav.me", icon: mdiAccountCircleOutline },
];

const activeIndex = computed(() =>
  navItems.findIndex((item) => route.path === item.to || route.path.startsWith(`${item.to}/`)),
);
</script>

<template>
  <div class="app" :class="{ wide: isWide }">
    <aside v-if="isWide" class="rail">
      <div class="brand">{{ t("app.name") }}</div>
      <nav class="rail-nav">
        <RouterLink
          v-for="item in navItems"
          :key="item.to"
          :to="item.to"
          class="rail-link"
          active-class="active"
        >
          <Icon :path="item.icon" :size="20" />
          <span>{{ t(item.label) }}</span>
        </RouterLink>
      </nav>
      <div class="rail-foot">M2 · {{ appVersion }}</div>
    </aside>

    <main class="content" :class="{ broad }">
      <RouterView v-slot="{ Component }">
        <Transition name="view" mode="out-in">
          <component v-if="viewsReady" :is="Component" :key="viewEpoch" />
        </Transition>
      </RouterView>
    </main>

    <nav v-if="!isWide" class="bottom">
      <span
        class="nav-pill"
        :class="{ hidden: activeIndex < 0 }"
        :style="{ transform: `translateX(${activeIndex * 100}%)` }"
        aria-hidden="true"
      />
      <RouterLink
        v-for="item in navItems"
        :key="item.to"
        :to="item.to"
        class="bottom-link"
        active-class="active"
      >
        <Icon :path="item.icon" :size="22" />
        <span>{{ t(item.label) }}</span>
      </RouterLink>
    </nav>

    <LockOverlay />
  </div>
</template>

<style scoped>
.app {
  display: flex;
  flex-direction: column;
  height: 100dvh;
  padding-top: var(--safe-top);
  background: var(--c-surface);
}

.app.wide {
  display: grid;
  grid-template-columns: 184px minmax(0, 1fr);
}

.rail {
  display: flex;
  flex-direction: column;
  gap: var(--s-6);
  padding: var(--s-5) var(--s-3);
  border-inline-end: 1px solid var(--c-hairline);
  background: var(--c-surface);
}

.brand {
  font-size: var(--t-title);
  line-height: var(--t-title-lh);
  font-weight: 600;
  padding-inline: var(--s-3);
  color: var(--c-on-surface);
}

.rail-nav {
  display: flex;
  flex-direction: column;
  gap: var(--s-1);
}

.rail-link {
  display: flex;
  align-items: center;
  gap: var(--s-2);
  min-height: 44px;
  padding-inline: var(--s-3);
  border-radius: var(--r-md);
  color: var(--c-on-surface-muted);
  font-size: var(--t-small);
  transition:
    background var(--m-1) var(--ease),
    color var(--m-1) var(--ease);
}

.rail-link.active {
  background: var(--c-primary-soft);
  color: var(--c-primary);
}

.rail-foot {
  margin-top: auto;
  padding-inline: var(--s-3);
  font-size: var(--t-caption);
  color: var(--c-on-surface-faint);
}

.content {
  flex: 1;
  min-height: 0;
  min-width: 0;
  width: 100%;
  max-width: 1040px;
  margin-inline: auto;
  overflow: hidden;
}

.app:not(.wide) .content {
  max-width: none;
}

.content.broad {
  max-width: 1440px;
}

.view-enter-active,
.view-leave-active {
  transition: opacity var(--m-1) var(--ease);
}

.view-enter-from,
.view-leave-to {
  opacity: 0;
}

.bottom {
  position: relative;
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  height: calc(var(--nav-h) + var(--safe-bottom));
  padding-bottom: var(--safe-bottom);
  border-top: 1px solid var(--c-hairline);
  background: var(--c-surface-1);
  flex: none;
}

.nav-pill {
  position: absolute;
  top: 0;
  inset-inline-start: 0;
  width: 33.3333%;
  height: 2px;
  transition:
    transform var(--m-2) var(--ease),
    opacity var(--m-2) var(--ease);
  pointer-events: none;
}

.nav-pill::after {
  content: "";
  display: block;
  width: 24px;
  height: 2px;
  margin-inline: auto;
  border-radius: var(--r-pill);
  background: var(--c-primary);
}

.nav-pill.hidden {
  opacity: 0;
}

.bottom-link {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 2px;
  color: var(--c-on-surface-muted);
  font-size: var(--t-caption);
  line-height: var(--t-caption-lh);
  min-height: 44px;
}

.bottom-link.active {
  color: var(--c-primary);
}
</style>