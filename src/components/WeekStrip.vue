<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { dayMarks } from "../ipc/client";
import {
  addDays,
  diaryToday,
  parseDayKey,
  weekDays,
  weekdayName,
} from "../utils/date";
import { useSession } from "../stores/session";

const props = defineProps<{
  selected: string;
  weekStart: number;
  reloadKey?: number;
}>();

const emit = defineEmits<{ select: [dayKey: string] }>();

const session = useSession();
const days = computed(() => weekDays(props.selected, props.weekStart));
const marks = ref<Record<string, number>>({});
const today = computed(() => diaryToday(session.settings.dayBoundaryMinutes));
const slide = ref<"next" | "prev">("next");

watch(
  () => days.value[0],
  (next, previous) => {
    if (previous && next !== previous) slide.value = next < previous ? "prev" : "next";
  },
);

let loadToken = 0;

async function load() {
  const token = ++loadToken;
  try {
    const list = await dayMarks(days.value[0], days.value[6]);
    if (token !== loadToken) return;
    const next: Record<string, number> = {};
    for (const mark of list) next[mark.day_key] = mark.n;
    marks.value = next;
  } catch {
    if (token === loadToken) marks.value = {};
  }
}

watch([days, () => props.reloadKey], load, { immediate: true });

let startX = 0;
let startY = 0;
let tracking = false;
let swiped = false;

function onPointerDown(event: PointerEvent) {
  if (event.pointerType === "mouse" && event.button !== 0) return;
  startX = event.clientX;
  startY = event.clientY;
  tracking = true;
  swiped = false;
}

function onPointerUp(event: PointerEvent) {
  if (!tracking) return;
  tracking = false;
  const deltaX = event.clientX - startX;
  const deltaY = event.clientY - startY;
  if (Math.abs(deltaX) < 48 || Math.abs(deltaX) <= Math.abs(deltaY)) return;
  swiped = true;
  emit("select", addDays(props.selected, deltaX > 0 ? -7 : 7));
}

function pick(day: string) {
  if (swiped) {
    swiped = false;
    return;
  }
  emit("select", day);
}

function dotCount(day: string): number {
  return Math.min(marks.value[day] ?? 0, 3);
}

function dayNumber(day: string): number {
  return parseDayKey(day).day;
}
</script>

<template>
  <div class="week" @pointerdown="onPointerDown" @pointerup="onPointerUp">
    <div class="viewport">
      <Transition :name="`week-${slide}`">
        <div :key="days[0]" class="grid">
          <button
            v-for="day in days"
            :key="day"
            type="button"
            class="cell"
            :class="{ today: day === today, selected: day === selected }"
            :aria-pressed="day === selected"
            @click="pick(day)"
          >
            <span class="weekday">{{ weekdayName(day) }}</span>
            <span class="number">{{ dayNumber(day) }}</span>
            <span class="dots" aria-hidden="true">
              <span v-for="index in dotCount(day)" :key="index" class="dot" />
            </span>
          </button>
        </div>
      </Transition>
    </div>
  </div>
</template>

<style scoped>
.week {
  position: relative;
  height: var(--week-h);
  padding: var(--s-2) var(--s-3);
  background: var(--c-surface-2);
  border-bottom: 1px solid var(--c-hairline);
  touch-action: pan-y;
  user-select: none;
}

.viewport {
  display: grid;
  height: 100%;
  overflow: hidden;
}

.grid {
  grid-area: 1 / 1;
  display: grid;
  grid-template-columns: repeat(7, 1fr);
  gap: var(--s-1);
  height: 100%;
}

.week-next-enter-active,
.week-next-leave-active,
.week-prev-enter-active,
.week-prev-leave-active {
  transition:
    opacity var(--m-2) var(--ease),
    transform var(--m-2) var(--ease);
}

.week-next-leave-active,
.week-prev-leave-active {
  pointer-events: none;
}

.week-next-enter-from {
  opacity: 0;
  transform: translateX(var(--s-4));
}

.week-next-leave-to {
  opacity: 0;
  transform: translateX(calc(var(--s-4) * -1));
}

.week-prev-enter-from {
  opacity: 0;
  transform: translateX(calc(var(--s-4) * -1));
}

.week-prev-leave-to {
  opacity: 0;
  transform: translateX(var(--s-4));
}

.cell {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 2px;
  border-radius: var(--r-md);
  color: var(--c-on-surface);
  min-width: 0;
}

.weekday {
  font-size: var(--t-caption);
  line-height: var(--t-caption-lh);
  color: var(--c-on-surface-muted);
}

.number {
  font-size: var(--t-title);
  line-height: 1;
  font-weight: 500;
  width: 32px;
  height: 32px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border-radius: 50%;
}

.cell.selected .number {
  background: var(--c-primary);
  color: var(--c-on-primary);
}

.cell.today .number {
  box-shadow: inset 0 0 0 1.5px var(--c-accent);
}

.cell.selected.today .number {
  box-shadow: inset 0 0 0 1.5px var(--c-on-primary);
}

.cell.selected .weekday {
  color: var(--c-primary);
}

.dots {
  height: 6px;
  display: flex;
  align-items: center;
  gap: 3px;
}

.dot {
  width: 4px;
  height: 4px;
  border-radius: var(--r-pill);
  background: var(--c-primary);
}

@media (hover: hover) {
  .cell:hover:not(.selected) {
    background: var(--c-surface-1);
  }
}
</style>