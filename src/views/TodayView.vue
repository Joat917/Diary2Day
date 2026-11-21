<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import {
  mdiCalendarMonthOutline,
  mdiCrosshairsGps,
  mdiPencil,
  mdiPlus,
} from "@mdi/js";
import { describeError, journalList, onDataChanged } from "../ipc/client";
import type { JournalSummary } from "../ipc/types";
import { t } from "../i18n";
import { useBreakpoint } from "../composables/useBreakpoint";
import { useSession } from "../stores/session";
import { diaryToday, formatDayTitle, greetingKey } from "../utils/date";
import DetailView from "./DetailView.vue";
import Icon from "../components/Icon.vue";
import JournalCard from "../components/JournalCard.vue";
import MiniCalendar from "../components/MiniCalendar.vue";
import StateBlock from "../components/StateBlock.vue";
import TodayCard from "../components/TodayCard.vue";
import WeekStrip from "../components/WeekStrip.vue";

const route = useRoute();
const router = useRouter();
const session = useSession();
const { isWide, isThreePane } = useBreakpoint();
const sheetOpen = ref(false);

const today = computed(() => diaryToday(session.settings.dayBoundaryMinutes));
const selected = computed(() => {
  const value = route.query.d;
  return typeof value === "string" && /^\d{4}-\d{2}-\d{2}$/.test(value) ? value : today.value;
});
const isToday = computed(() => selected.value === today.value);

const items = ref<JournalSummary[]>([]);
const nextCursor = ref<string | null>(null);
const loading = ref(true);
const error = ref<string | null>(null);
const version = ref(0);
const listDir = ref<"next" | "prev">("next");
const loadedDay = ref("");

let loadToken = 0;

async function load() {
  const token = ++loadToken;
  const day = selected.value;
  const dayChanged = loadedDay.value !== day;
  if (dayChanged) {
    items.value = [];
    nextCursor.value = null;
  }
  loading.value = true;
  error.value = null;
  try {
    const page = await journalList({ dayKey: day, limit: 50 });
    if (token !== loadToken) return;
    items.value = page.items;
    nextCursor.value = page.next_cursor;
    loadedDay.value = day;
  } catch (err) {
    if (token === loadToken) error.value = describeError(err);
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
  } catch (err) {
    error.value = describeError(err);
  }
}

watch(
  selected,
  (day, previous) => {
    if (previous && day !== previous) listDir.value = day < previous ? "prev" : "next";
    void load();
  },
  { immediate: true },
);

function selectDay(dayKey: string) {
  if (dayKey === selected.value) return;
  void router.replace({ path: "/today", query: { d: dayKey } });
}

const openedId = computed(() => {
  const value = route.query.e;
  return typeof value === "string" && value ? value : null;
});

function openEntry(id: string) {
  if (!isThreePane.value) {
    void router.push(`/j/${id}`);
    return;
  }
  void router.replace({ path: "/today", query: { ...route.query, e: id } });
}

function closeEntry() {
  const query: Record<string, string> = {};
  for (const [key, value] of Object.entries(route.query)) {
    if (key !== "e" && typeof value === "string") query[key] = value;
  }
  void router.replace({ path: "/today", query });
}

const creating = ref(false);

async function createEntry() {
  if (creating.value) return;
  creating.value = true;
  try {
    await router.push({ path: "/j/new", query: { edit: "1" } });
  } finally {
    creating.value = false;
  }
}

function primaryAction() {
  if (isToday.value) void createEntry();
  else selectDay(today.value);
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

const emptyTitle = computed(() => {
  if (selected.value === today.value) return t("today.emptyToday");
  return selected.value < today.value ? t("today.emptyPast") : t("today.emptyFuture");
});

function onSaved() {
  version.value += 1;
  if (isToday.value) void load();
  else selectDay(today.value);
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
  <div class="view" :class="{ wide: isWide, three: isThreePane }">
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
            :class="{ secondary: !isToday }"
            type="button"
            :disabled="isToday && creating"
            @click="primaryAction"
          >
            <Icon :path="isToday ? mdiPlus : mdiCrosshairsGps" :size="18" />
            {{ isToday ? t("common.new") : t("nav.today") }}
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
        <TodayCard v-if="isToday" @saved="onSaved" />

        <div class="day-stack">
          <Transition :name="`day-${listDir}`">
            <section :key="selected" class="day-list">
              <p v-if="items.length" class="count">
                {{ t("today.count", { n: items.length }) }}
              </p>

              <StateBlock v-if="loading && !items.length" kind="loading" />
              <StateBlock
                v-else-if="error"
                kind="error"
                :title="t('common.error')"
                :hint="error ?? undefined"
                :action-label="t('common.retry')"
                @action="load"
              />
              <StateBlock v-else-if="!items.length" kind="empty" :title="emptyTitle" />
              <template v-else>
                <TransitionGroup name="card">
                  <JournalCard
                    v-for="item in items"
                    :key="item.id"
                    :item="item"
                    :active="item.id === openedId"
                    @open="openEntry"
                  />
                </TransitionGroup>
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
          </Transition>
        </div>
      </div>
    </div>

    <aside v-if="isThreePane" class="detail-col">
      <Transition name="detail-panel" mode="out-in">
        <DetailView
          v-if="openedId"
          :key="openedId"
          :entry-id="openedId"
          @close="closeEntry"
        />
        <div v-else key="empty" class="detail-empty">
          <Icon :path="mdiPencil" :size="24" />
          <p>{{ t("today.pickEntry") }}</p>
        </div>
      </Transition>
    </aside>

    <button
      v-if="!isWide"
      class="fab"
      :class="{ secondary: !isToday }"
      type="button"
      :disabled="isToday && creating"
      @click="primaryAction"
    >
      <Icon :path="isToday ? mdiPlus : mdiCrosshairsGps" :size="20" />
      {{ isToday ? t("common.new") : t("nav.today") }}
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
  grid-template-columns: 300px minmax(0, 1fr);
}

.view.three {
  grid-template-columns: 240px minmax(300px, 1fr) minmax(420px, 1fr);
}

.detail-col {
  min-width: 0;
  min-height: 0;
  overflow: hidden;
  border-inline-start: 1px solid var(--c-hairline);
  background: var(--c-surface-1);
}

.detail-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: var(--s-2);
  height: 100%;
  padding: var(--s-6);
  color: var(--c-on-surface-faint);
  font-size: var(--t-small);
  text-align: center;
}

.detail-panel-enter-active,
.detail-panel-leave-active {
  transition:
    opacity var(--m-2) var(--ease),
    transform var(--m-2) var(--ease);
}

.detail-panel-enter-from,
.detail-panel-leave-to {
  opacity: 0;
  transform: translateX(12px);
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
  transition: transform var(--m-1) var(--ease);
}

.new-btn:active:not(:disabled) {
  transform: scale(0.98);
}

.fab {
  position: fixed;
  inset-inline-end: var(--s-4);
  bottom: calc(var(--nav-h) + var(--safe-bottom) + var(--s-4));
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
  transition: transform var(--m-1) var(--ease);
}

.fab:active:not(:disabled) {
  transform: scale(0.98);
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
  max-width: 480px;
  max-height: 100dvh;
  margin-inline: auto;
  overflow-y: auto;
  padding: var(--s-2) var(--s-3) var(--s-3);
  padding-top: calc(var(--s-2) + var(--safe-top));
  background: var(--c-surface-1);
  border: 1px solid var(--c-hairline);
  border-top: none;
  border-start-start-radius: 0;
  border-start-end-radius: 0;
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

.new-btn.secondary,
.fab.secondary {
  background: var(--c-surface-2);
  color: var(--c-primary);
}

.scroll {
  flex: 1;
  min-height: 0;
  overflow-x: hidden;
  overflow-y: auto;
  scrollbar-gutter: stable;
  padding-bottom: 96px;
}

@media (min-width: 960px) {
  .scroll {
    padding-bottom: var(--s-6);
  }
}

.day-stack {
  position: relative;
  display: grid;
  grid-template-columns: minmax(0, 1fr);
}

.day-stack > .day-list {
  grid-area: 1 / 1;
}

.day-list {
  display: flex;
  flex-direction: column;
  gap: var(--s-3);
  min-width: 0;
  padding: 0 var(--s-4);
}

.day-next-enter-active,
.day-next-leave-active,
.day-prev-enter-active,
.day-prev-leave-active {
  transition:
    opacity var(--m-3) var(--ease),
    transform var(--m-3) var(--ease);
}

.day-next-leave-active,
.day-prev-leave-active {
  position: absolute;
  inset-inline: 0;
  top: 0;
  pointer-events: none;
}

.day-next-enter-from {
  opacity: 0;
  transform: translateX(var(--s-8));
}

.day-next-leave-to {
  opacity: 0;
  transform: translateX(calc(var(--s-8) * -1));
}

.day-prev-enter-from {
  opacity: 0;
  transform: translateX(calc(var(--s-8) * -1));
}

.day-prev-leave-to {
  opacity: 0;
  transform: translateX(var(--s-8));
}

.card-enter-active {
  transition:
    opacity var(--m-1) var(--ease),
    transform var(--m-1) var(--ease);
}

.card-enter-from {
  opacity: 0;
  transform: translateY(var(--s-2));
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