<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { dayMarks } from "../ipc/client";
import {
  addDays,
  monthLabel,
  parseDayKey,
  todayKey,
  weekDays,
  weekdayName,
} from "../utils/date";

const props = defineProps<{
  selected: string;
  weekStart: number;
  reloadKey?: number;
}>();

const emit = defineEmits<{ select: [dayKey: string] }>();

const days = computed(() => weekDays(props.selected, props.weekStart));
const marks = ref<Record<string, number>>({});
const today = todayKey();

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
let tracking = false;

function onPointerDown(event: PointerEvent) {
  if (event.pointerType === "mouse" && event.button !== 0) return;
  startX = event.clientX;
  tracking = true;
  (event.currentTarget as HTMLElement).setPointerCapture?.(event.pointerId);
}

function onPointerUp(event: PointerEvent) {
  if (!tracking) return;
  tracking = false;
  const delta = event.clientX - startX;
  if (Math.abs(delta) < 48) return;
  emit("select", addDays(props.selected, delta > 0 ? -7 : 7));
}

const crossesMonth = computed(
  () => days.value[0].slice(0, 7) !== days.value[6].slice(0, 7),
);

function dotCount(day: string): number {
  const count = marks.value[day] ?? 0;
  if (count <= 1) return count;
  if (count <= 3) return 3;
  return 2;
}

function dayNumber(day: string): number {
  return parseDayKey(day).day;
}
</script>

<template>
  <div class="week" @pointerdown="onPointerDown" @pointerup="onPointerUp">
    <div v-if="crossesMonth" class="month">{{ monthLabel(days[3]) }}</div>
    <div class="grid">
      <button
        v-for="day in days"
        :key="day"
        type="button"
        class="cell"
        :class="{ today: day === today, selected: day === selected }"
        :aria-pressed="day === selected"
        @click="emit('select', day)"
      >
        <span class="weekday">{{ weekdayName(day) }}</span>
        <span class="number">{{ dayNumber(day) }}</span>
        <span class="dots" aria-hidden="true">
          <span v-if="(marks[day] ?? 0) > 3" class="more">{{ marks[day] }}</span>
          <span v-for="index in dotCount(day)" :key="index" class="dot" />
        </span>
      </button>
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

.month {
  position: absolute;
  top: 2px;
  inset-inline-end: var(--s-3);
  font-size: var(--t-caption);
  line-height: var(--t-caption-lh);
  color: var(--c-on-surface-muted);
}

.grid {
  display: grid;
  grid-template-columns: repeat(7, 1fr);
  gap: var(--s-1);
  height: 100%;
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

.more {
  font-size: var(--t-caption);
  line-height: var(--t-caption-lh);
  color: var(--c-primary);
}

@media (hover: hover) {
  .cell:hover:not(.selected) {
    background: var(--c-surface-1);
  }
}
</style>