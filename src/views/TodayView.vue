<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { mdiCalendarMonthOutline, mdiPlus } from "@mdi/js";
import { journalList, journalSave, onDataChanged } from "../ipc/client";
import type { JournalSummary } from "../ipc/types";
import { t } from "../i18n";
import { useBreakpoint } from "../composables/useBreakpoint";
import { useSession } from "../stores/session";
import { formatDayTitle, greetingKey, recordedAtForDay, todayKey } from "../utils/date";
import Icon from "../components/Icon.vue";
import JournalCard from "../components/JournalCard.vue";
import MiniCalendar from "../components/MiniCalendar.vue";
import StateBlock from "../components/StateBlock.vue";
import TodayCard from "../components/TodayCard.vue";
import WeekStrip from "../components/WeekStrip.vue";

const route = useRoute();
const router = useRouter();
const session = useSession();
const { isWide } = useBreakpoint();
const sheetOpen = ref(false);

const selected = computed(() => {
  const value = route.query.d;
  return typeof value === "string" && /^\d{4}-\d{2}-\d{2}$/.test(value) ? value : todayKey();
});

const items = ref<JournalSummary[]>([]);
const nextCursor = ref<string | null>(null);
const loading = ref(true);
const error = ref(false);
const version = ref(0);
const card = ref<InstanceType<typeof TodayCard> | null>(null);

let loadToken = 0;

async function load() {
  const token = ++loadToken;
  loading.value = true;
  error.value = false;
  try {
    const page = await journalList({ dayKey: selected.value, limit: 50 });
    if (token !== loadToken) return;
    items.value = page.items;
    nextCursor.value = page.next_cursor;
  } catch {
    if (token === loadToken) error.value = true;
  } finally {
    if (token === loadToken) loading.value = false;
  }
}

async function loadMore() {
  if (!nextCursor.value) return;
  try {
    const page = await journalList({
      dayKey: selected.value,
      limit: 50,
      cursor: nextCursor.value,
    });
    items.value = [...items.value, ...page.items];
    nextCursor.value = page.next_cursor;
  } catch {
    error.value = true;
  }
}

watch(selected, load, { immediate: true });

function selectDay(dayKey: string) {
  if (dayKey === selected.value) return;
  void router.replace({ path: "/today", query: { d: dayKey } });
}

function openEntry(id: string) {
  void router.push(`/j/${id}`);
}

const creating = ref(false);

async function createEntry() {
  if (creating.value) return;
  creating.value = true;
  try {
    const stamp = recordedAtForDay(selected.value);
    const entry = await journalSave({
      content: "",
      recordedAt: stamp.recordedAt,
      recordedOff: stamp.recordedOff,
    });
    await router.push({ path: `/j/${entry.id}`, query: { edit: "1" } });
  } catch {
    error.value = true;
  } finally {
    creating.value = false;
  }
}

function pickDay(dayKey: string) {
  selectDay(dayKey);
  sheetOpen.value = false;
}

function onSheetKey(event: KeyboardEvent) {
  if (event.key === "Escape") sheetOpen.value = false;
}

watch(sheetOpen, (open) => {
  if (open) document.addEventListener("keydown", onSheetKey);
  else document.removeEventListener("keydown", onSheetKey);
});

function onSaved() {
  version.value += 1;
  void load();
}

let unlisten: (() => void) | null = null;

onMounted(async () => {
  unlisten = await onDataChanged((payload) => {
    version.value += 1;
    if (!payload.day_key || payload.day_key === selected.value) void load();
  });
});

onBeforeUnmount(() => {
  unlisten?.();
  document.removeEventListener("keydown", onSheetKey);
});
</script>

<template>
  <div class="view" :class="{ wide: isWide }">
    <aside v-if="isWide" class="cal-col">
      <MiniCalendar
        :selected="selected"
        :week-start="session.settings.weekStart"
        :reload-key="version"
        @select="selectDay"
      />
    </aside>

    <div class="main">
      <header class="topbar">
        <div>
          <p class="greeting">{{ t(greetingKey()) }}</p>
          <h1 class="day">{{ formatDayTitle(selected) }}</h1>
        </div>
        <div class="top-actions">
          <button
            v-if="isWide"
            class="new-btn"
            type="button"
            :disabled="creating"
            @click="createEntry"
          >
            <Icon :path="mdiPlus" :size="18" />
            {{ t("common.new") }}
          </button>
          <button
            v-if="selected !== todayKey()"
            class="today-btn"
            type="button"
            @click="selectDay(todayKey())"
          >
            {{ t("nav.today") }}
          </button>
          <button
            v-if="!isWide"
            class="icon-btn"
            type="button"
            :aria-label="t('calendar.open')"
            @click="sheetOpen = true"
          >
            <Icon :path="mdiCalendarMonthOutline" :size="20" />
          </button>
        </div>
      </header>

      <WeekStrip
        v-if="!isWide"
        :selected="selected"
        :week-start="session.settings.weekStart"
        :reload-key="version"
        @select="selectDay"
      />

      <div class="scroll">
        <TodayCard ref="card" :day-key="selected" @saved="onSaved" />

        <section class="day-list">
          <p v-if="items.length" class="count">
            {{ t("today.count", { n: items.length }) }}
          </p>

          <StateBlock v-if="loading && !items.length" kind="loading" />
          <StateBlock
            v-else-if="error"
            kind="error"
            :title="t('common.error')"
            :action-label="t('common.retry')"
            @action="load"
          />
          <StateBlock
            v-else-if="!items.length"
            kind="empty"
            :title="t('today.empty')"
            :action-label="t('today.emptyAction')"
            @action="card?.focus()"
          />
          <template v-else>
            <JournalCard
              v-for="item in items"
              :key="item.id"
              :item="item"
              @open="openEntry"
            />
          </template>

          <button
            v-if="nextCursor"
            class="more"
            type="button"
            :disabled="loading"
            @click="loadMore"
          >
            {{ t("today.loadMore") }}
          </button>
        </section>
      </div>
    </div>

    <button
      v-if="!isWide"
      class="fab"
      type="button"
      :disabled="creating"
      @click="createEntry"
    >
      <Icon :path="mdiPlus" :size="20" />
      {{ t("common.new") }}
    </button>

    <Transition name="sheet">
      <div v-if="sheetOpen" class="sheet-layer">
        <div class="scrim" @click="sheetOpen = false" />
        <div class="sheet" role="dialog" aria-modal="true">
          <MiniCalendar
            large
            :selected="selected"
            :week-start="session.settings.weekStart"
            :reload-key="version"
            @select="pickDay"
            @close="sheetOpen = false"
          />
        </div>
      </div>
    </Transition>
  </div>
</template>

<style scoped>
.view {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
  position: relative;
}

.view.wide {
  display: grid;
  grid-template-columns: 232px minmax(0, 1fr);
}

.cal-col {
  min-height: 0;
  overflow-y: auto;
  padding: var(--s-3) var(--s-2);
  border-inline-end: 1px solid var(--c-hairline);
  background: var(--c-surface-1);
}

.main {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-width: 0;
  min-height: 0;
}

.top-actions {
  display: flex;
  align-items: center;
  gap: var(--s-1);
}

.new-btn {
  display: inline-flex;
  align-items: center;
  gap: var(--s-1);
  height: 36px;
  padding-inline: var(--s-3);
  border-radius: var(--r-pill);
  background: var(--c-primary);
  color: var(--c-on-primary);
  font-size: var(--t-small);
  flex: none;
}

.fab {
  position: fixed;
  inset-inline-end: var(--s-4);
  bottom: calc(var(--nav-h) + env(safe-area-inset-bottom, 0px) + var(--s-4));
  display: inline-flex;
  align-items: center;
  gap: var(--s-1);
  height: 46px;
  padding-inline: var(--s-4);
  border-radius: var(--r-pill);
  background: var(--c-primary);
  color: var(--c-on-primary);
  box-shadow: var(--e-2);
  font-size: var(--t-small);
  z-index: 30;
}

.icon-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 40px;
  height: 40px;
  border-radius: var(--r-pill);
  color: var(--c-on-surface-muted);
}

@media (hover: hover) {
  .icon-btn:hover {
    background: var(--c-surface-2);
  }
}

.sheet-layer {
  position: fixed;
  inset: 0;
  z-index: 40;
}

.scrim {
  position: absolute;
  inset: 0;
  background: var(--c-scrim);
}

.sheet {
  position: absolute;
  inset-inline: 0;
  top: 0;
  padding: var(--s-2) var(--s-3) var(--s-3);
  padding-top: calc(var(--s-2) + env(safe-area-inset-top, 0px));
  background: var(--c-surface-1);
  border-end-start-radius: var(--r-lg);
  border-end-end-radius: var(--r-lg);
  box-shadow: var(--e-2);
}

.sheet-enter-active,
.sheet-leave-active {
  transition: opacity var(--m-3) var(--ease);
}

.sheet-enter-active .sheet,
.sheet-leave-active .sheet {
  transition: transform var(--m-3) var(--ease);
}

.sheet-enter-from,
.sheet-leave-to {
  opacity: 0;
}

.sheet-enter-from .sheet,
.sheet-leave-to .sheet {
  transform: translateY(-24px);
}

@media (prefers-reduced-motion: reduce) {
  .sheet-enter-active,
  .sheet-leave-active,
  .sheet-enter-active .sheet,
  .sheet-leave-active .sheet {
    transition: none;
  }
}

.topbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--s-3);
  min-height: var(--topbar-h);
  padding: var(--s-3) var(--s-4) var(--s-2);
  background: var(--c-surface);
}

@media (min-width: 960px) {
  .topbar {
    border-bottom: 1px solid var(--c-hairline);
  }
}

.topbar > div:first-child {
  min-width: 0;
}

.greeting {
  font-size: var(--t-caption);
  line-height: var(--t-caption-lh);
  color: var(--c-on-surface-muted);
  letter-spacing: 0.04em;
}

.day {
  font-size: var(--t-display);
  line-height: var(--t-display-lh);
  font-weight: 500;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.today-btn {
  padding: var(--s-1) var(--s-3);
  border-radius: var(--r-pill);
  background: var(--c-surface-2);
  color: var(--c-primary);
  font-size: var(--t-small);
  min-height: 36px;
}

.scroll {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding-bottom: 96px;
}

@media (min-width: 960px) {
  .scroll {
    padding-bottom: var(--s-6);
  }
}

.day-list {
  display: flex;
  flex-direction: column;
  gap: var(--s-3);
  padding: 0 var(--s-4);
}

.count {
  font-size: var(--t-caption);
  line-height: var(--t-caption-lh);
  color: var(--c-on-surface-muted);
  padding-top: var(--s-2);
}

.more {
  align-self: center;
  padding: var(--s-2) var(--s-5);
  border-radius: var(--r-pill);
  background: var(--c-surface-2);
  color: var(--c-on-surface-muted);
  font-size: var(--t-small);
  min-height: 44px;
  margin-block: var(--s-2);
}
</style>