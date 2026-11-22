<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { mdiChevronLeft, mdiChevronRight } from "@mdi/js";
import { dayMarks } from "../ipc/client";
import { t, type MessageKey } from "../i18n";
import {
  addDays,
  diaryToday,
  monthLabel,
  parseDayKey,
  startOfWeek,
  weekdayName,
  weekdayOf,
} from "../utils/date";
import { useSession } from "../stores/session";
import Icon from "./Icon.vue";

const props = withDefaults(
  defineProps<{
    selected: string;
    weekStart: number;
    reloadKey?: number;
    large?: boolean;
  }>(),
  { large: false },
);

const emit = defineEmits<{
  select: [dayKey: string];
  close: [];
}>();

const session = useSession();
const today = computed(() => diaryToday(session.settings.dayBoundaryMinutes));
const viewMonth = ref(props.selected.slice(0, 7));
const cursor = ref(props.selected);
const marks = ref<Record<string, number>>({});
const slide = ref<"next" | "prev">("next");
let loadToken = 0;

watch(
  () => props.selected,
  (value) => {
    cursor.value = value;
    const month = value.slice(0, 7);
    if (month !== viewMonth.value) {
      slide.value = month < viewMonth.value ? "prev" : "next";
      viewMonth.value = month;
    }
  },
);

function shiftMonth(month: string, delta: number): string {
  const { year, month: number } = parseDayKey(`${month}-01`);
  const date = new Date(Date.UTC(year, number - 1 + delta, 1));
  return `${date.getUTCFullYear()}-${String(date.getUTCMonth() + 1).padStart(2, "0")}`;
}

function firstOf(month: string): string {
  return `${month}-01`;
}

const leading = computed(
  () => (weekdayOf(firstOf(viewMonth.value)) - props.weekStart + 7) % 7,
);

const cells = computed(() => {
  const start = addDays(firstOf(viewMonth.value), -leading.value);
  const out: { day: string; inMonth: boolean }[] = [];
  for (let index = 0; index < 42; index += 1) {
    const day = addDays(start, index);
    out.push({ day, inMonth: day.slice(0, 7) === viewMonth.value });
  }
  return out;
});

const headers = computed(() =>
  Array.from({ length: 7 }, (_, index) =>
    t(`weekday.${(props.weekStart + index) % 7}` as MessageKey),
  ),
);

async function load() {
  const token = ++loadToken;
  const list = cells.value;
  try {
    const result = await dayMarks(list[0].day, list[list.length - 1].day);
    if (token !== loadToken) return;
    const next: Record<string, number> = {};
    for (const mark of result) next[mark.day_key] = mark.n;
    marks.value = next;
  } catch {
    if (token === loadToken) marks.value = {};
  }
}

watch([viewMonth, () => props.reloadKey], load, { immediate: true });

let swiped = false;

function pick(day: string) {
  if (swiped) {
    swiped = false;
    return;
  }
  if (day.slice(0, 7) !== viewMonth.value) return;
  cursor.value = day;
  emit("select", day);
}

function move(delta: number) {
  cursor.value = addDays(cursor.value, delta);
  const month = cursor.value.slice(0, 7);
  if (month !== viewMonth.value) viewMonth.value = month;
}

function jumpMonth(delta: number) {
  slide.value = delta < 0 ? "prev" : "next";
  viewMonth.value = shiftMonth(viewMonth.value, delta);
  cursor.value = addDays(cursor.value, delta * 28);
}

function onKeydown(event: KeyboardEvent) {
  switch (event.key) {
    case "ArrowLeft":
      move(-1);
      break;
    case "ArrowRight":
      move(1);
      break;
    case "ArrowUp":
      move(-7);
      break;
    case "ArrowDown":
      move(7);
      break;
    case "PageUp":
      jumpMonth(-1);
      break;
    case "PageDown":
      jumpMonth(1);
      break;
    case "Home":
      cursor.value = startOfWeek(cursor.value, props.weekStart);
      if (cursor.value.slice(0, 7) !== viewMonth.value) viewMonth.value = cursor.value.slice(0, 7);
      break;
    case "End":
      cursor.value = addDays(startOfWeek(cursor.value, props.weekStart), 6);
      if (cursor.value.slice(0, 7) !== viewMonth.value) viewMonth.value = cursor.value.slice(0, 7);
      break;
    case "Enter":
      pick(cursor.value);
      break;
    case "Escape":
      emit("close");
      break;
    default:
      return;
  }
  event.preventDefault();
}

let startX = 0;
let tracking = false;

function onPointerDown(event: PointerEvent) {
  if (event.pointerType === "mouse" && event.button !== 0) return;
  startX = event.clientX;
  tracking = true;
  swiped = false;
}

function onPointerUp(event: PointerEvent) {
  if (!tracking) return;
  tracking = false;
  const delta = event.clientX - startX;
  if (Math.abs(delta) < 48) return;
  swiped = true;
  jumpMonth(delta > 0 ? -1 : 1);
}

function dotCount(day: string): number {
  return Math.min(marks.value[day] ?? 0, 3);
}
</script>

<template>
  <section class="calendar" :class="{ large }" @pointerdown="onPointerDown" @pointerup="onPointerUp">
    <header class="head">
      <button
        class="nav"
        type="button"
        :aria-label="t('calendar.prev')"
        @click="jumpMonth(-1)"
      >
        <Icon :path="mdiChevronLeft" :size="18" />
      </button>
      <span class="month">{{ monthLabel(firstOf(viewMonth)) }}</span>
      <button
        class="nav"
        type="button"
        :aria-label="t('calendar.next')"
        @click="jumpMonth(1)"
      >
        <Icon :path="mdiChevronRight" :size="18" />
      </button>
    </header>

    <div class="grid-viewport">
      <Transition :name="`cal-${slide}`">
        <div :key="viewMonth" class="grid" tabindex="0" role="grid" @keydown="onKeydown">
          <span v-for="(label, index) in headers" :key="index" class="weekday" role="columnheader">
            {{ label }}
          </span>
          <button
            v-for="cell in cells"
            :key="cell.day"
            type="button"
            class="day"
            role="gridcell"
            :class="{
              out: !cell.inMonth,
              today: cell.day === today,
              selected: cell.day === selected,
              cursor: cell.day === cursor,
            }"
            :disabled="!cell.inMonth"
            :aria-selected="cell.day === selected"
            :aria-label="`${cell.day} ${weekdayName(cell.day)}`"
            @click="pick(cell.day)"
          >
            <span class="number">{{ parseDayKey(cell.day).day }}</span>
            <span class="dots" aria-hidden="true">
              <template v-if="cell.inMonth && (marks[cell.day] ?? 0) > 0">
                <span v-for="index in dotCount(cell.day)" :key="index" class="dot" />
              </template>
            </span>
          </button>
        </div>
      </Transition>
    </div>
  </section>
</template>

<style scoped>
.calendar {
  display: flex;
  flex-direction: column;
  gap: var(--s-2);
  padding: var(--s-2);
  background: transparent;
}

.head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--s-1);
}

.month {
  font-size: var(--t-small);
  line-height: var(--t-small-lh);
  color: var(--c-on-surface);
}

.nav {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border-radius: var(--r-sm);
  color: var(--c-on-surface-muted);
}

@media (hover: hover) {
  .nav:hover,
  .day:hover:not(:disabled):not(.selected) {
    background: var(--c-surface-2);
  }
}

.grid-viewport {
  display: grid;
  overflow: hidden;
}

.grid {
  grid-area: 1 / 1;
  display: grid;
  grid-template-columns: repeat(7, minmax(0, 1fr));
  gap: 2px;
  outline: none;
}

.grid:focus-visible {
  outline: 2px solid var(--c-primary);
  outline-offset: -2px;
  border-radius: var(--r-sm);
}

.cal-next-enter-active,
.cal-next-leave-active,
.cal-prev-enter-active,
.cal-prev-leave-active {
  transition:
    opacity var(--m-3) var(--ease),
    transform var(--m-3) var(--ease);
}

.cal-next-leave-active,
.cal-prev-leave-active {
  pointer-events: none;
}

.cal-next-enter-from {
  opacity: 0;
  transform: translateX(var(--s-4));
}

.cal-next-leave-to {
  opacity: 0;
  transform: translateX(calc(var(--s-4) * -1));
}

.cal-prev-enter-from {
  opacity: 0;
  transform: translateX(calc(var(--s-4) * -1));
}

.cal-prev-leave-to {
  opacity: 0;
  transform: translateX(var(--s-4));
}

.weekday {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 22px;
  font-size: var(--t-caption);
  line-height: var(--t-caption-lh);
  color: var(--c-on-surface-muted);
}

.day {
  position: relative;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 1px;
  height: 34px;
  border-radius: var(--r-sm);
  color: var(--c-on-surface);
}

.large .day {
  height: 44px;
}

.number {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 26px;
  height: 26px;
  flex: none;
  border-radius: 50%;
  font-size: var(--t-small);
  line-height: 1;
  font-weight: 500;
}

.large .number {
  width: 30px;
  height: 30px;
}

.day.out {
  color: var(--c-on-surface-faint);
  opacity: 0.4;
  cursor: default;
}

.day.today .number {
  box-shadow: inset 0 0 0 1.5px var(--c-accent);
}

.day.selected {
  background: var(--c-primary);
  color: var(--c-on-primary);
}

.day.selected.today .number {
  box-shadow: inset 0 0 0 1.5px var(--c-on-primary);
}

.day.selected .dot {
  background: var(--c-on-primary);
}

.day.cursor:not(.selected) {
  box-shadow: inset 0 0 0 1px var(--c-hairline);
}

.dots {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 2px;
  height: 5px;
}

.dot {
  width: 4px;
  height: 4px;
  border-radius: var(--r-pill);
  background: var(--c-primary);
}
</style>