<script setup lang="ts">
import { RouterLink, RouterView } from "vue-router";
import { mdiAccountCircleOutline, mdiCalendarToday, mdiMagnify } from "@mdi/js";
import { useBreakpoint } from "./composables/useBreakpoint";
import { t, type MessageKey } from "./i18n";
import Icon from "./components/Icon.vue";

const { isWide } = useBreakpoint();

const navItems: { to: string; label: MessageKey; icon: string }[] = [
  { to: "/today", label: "nav.today", icon: mdiCalendarToday },
  { to: "/discover", label: "nav.discover", icon: mdiMagnify },
  { to: "/me", label: "nav.me", icon: mdiAccountCircleOutline },
];
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
      <div class="rail-foot">M0 · 0.1.0</div>
    </aside>

    <main class="content">
      <RouterView />
    </main>

    <nav v-if="!isWide" class="bottom">
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
  </div>
</template>

<style scoped>
.app {
  display: flex;
  flex-direction: column;
  height: 100dvh;
  background: var(--c-surface);
}

.app.wide {
  display: grid;
  grid-template-columns: 220px minmax(0, 1fr);
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

.bottom {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  height: calc(var(--nav-h) + env(safe-area-inset-bottom, 0px));
  padding-bottom: env(safe-area-inset-bottom, 0px);
  border-top: 1px solid var(--c-hairline);
  background: var(--c-surface-1);
  flex: none;
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