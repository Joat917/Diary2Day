<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { mdiClose, mdiMagnifyMinusOutline, mdiMagnifyPlusOutline, mdiRestore } from "@mdi/js";
import { t } from "../i18n";
import { useBackLayer } from "../composables/useBackLayer";
import Icon from "./Icon.vue";

const props = defineProps<{ src: string | null; alt?: string }>();
const emit = defineEmits<{ close: [] }>();

const scale = ref(1);
const x = ref(0);
const y = ref(0);
const dragging = ref(false);
let startX = 0;
let startY = 0;
let originX = 0;
let originY = 0;

const open = computed(() => !!props.src);

useBackLayer(open, () => emit("close"));

const transform = computed(
  () => `translate(${x.value}px, ${y.value}px) scale(${scale.value})`,
);

function reset() {
  scale.value = 1;
  x.value = 0;
  y.value = 0;
}

function zoom(delta: number) {
  const next = Math.min(6, Math.max(0.4, scale.value + delta));
  scale.value = Number(next.toFixed(2));
  if (scale.value <= 1) {
    x.value = 0;
    y.value = 0;
  }
}

function onWheel(event: WheelEvent) {
  if (!open.value) return;
  event.preventDefault();
  zoom(event.deltaY < 0 ? 0.2 : -0.2);
}

function onPointerDown(event: PointerEvent) {
  if (!open.value || scale.value <= 1) return;
  dragging.value = true;
  startX = event.clientX;
  startY = event.clientY;
  originX = x.value;
  originY = y.value;
  (event.target as HTMLElement).setPointerCapture?.(event.pointerId);
}

function onPointerMove(event: PointerEvent) {
  if (!dragging.value) return;
  x.value = originX + (event.clientX - startX);
  y.value = originY + (event.clientY - startY);
}

function onPointerUp() {
  dragging.value = false;
}

function onKey(event: KeyboardEvent) {
  if (!open.value) return;
  if (event.key === "Escape") {
    event.stopPropagation();
    emit("close");
  }
}

watch(
  () => props.src,
  () => reset(),
);

onMounted(() => document.addEventListener("keydown", onKey, true));
onBeforeUnmount(() => document.removeEventListener("keydown", onKey, true));
</script>

<template>
  <Transition name="viewer">
    <div v-if="open" class="viewer" role="dialog" aria-modal="true">
      <div class="scrim" @click="emit('close')" />
      <div class="stage" @dblclick="reset">
        <img
          :src="src ?? ''"
          :alt="alt ?? ''"
          :style="{ transform }"
          :class="{ dragging }"
          draggable="false"
          @wheel="onWheel"
          @pointerdown="onPointerDown"
          @pointermove="onPointerMove"
          @pointerup="onPointerUp"
          @pointercancel="onPointerUp"
        />
      </div>
      <div class="toolbar">
        <button type="button" :aria-label="t('viewer.zoomOut')" @click="zoom(-0.2)">
          <Icon :path="mdiMagnifyMinusOutline" :size="20" />
        </button>
        <span class="scale">{{ Math.round(scale * 100) }}%</span>
        <button type="button" :aria-label="t('viewer.zoomIn')" @click="zoom(0.2)">
          <Icon :path="mdiMagnifyPlusOutline" :size="20" />
        </button>
        <button type="button" :aria-label="t('viewer.reset')" @click="reset">
          <Icon :path="mdiRestore" :size="20" />
        </button>
        <button type="button" :aria-label="t('common.close')" @click="emit('close')">
          <Icon :path="mdiClose" :size="20" />
        </button>
      </div>
      <p class="hint">{{ t("viewer.hint") }}</p>
    </div>
  </Transition>
</template>

<style scoped>
.viewer {
  position: fixed;
  inset: 0;
  z-index: 60;
  display: flex;
  align-items: center;
  justify-content: center;
}

.scrim {
  position: absolute;
  inset: 0;
  background: rgba(0, 0, 0, 0.82);
}

.stage {
  position: absolute;
  inset: calc(48px + var(--safe-top)) calc(16px + var(--safe-right))
    calc(72px + var(--safe-bottom)) calc(16px + var(--safe-left));
  display: flex;
  align-items: center;
  justify-content: center;
}

.stage img {
  max-width: 100%;
  max-height: 100%;
  border-radius: var(--r-md);
  transition: transform var(--m-1) var(--ease);
  cursor: zoom-in;
  touch-action: none;
}

.stage img.dragging {
  transition: none;
  cursor: grabbing;
}

.toolbar {
  position: absolute;
  inset-inline-end: calc(var(--s-4) + var(--safe-right));
  bottom: calc(var(--s-4) + var(--safe-bottom));
  display: flex;
  align-items: center;
  gap: var(--s-1);
  padding: var(--s-1);
  border-radius: var(--r-pill);
  background: rgba(24, 26, 25, 0.86);
  color: #fff;
}

.toolbar button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 40px;
  height: 40px;
  border-radius: var(--r-pill);
  color: inherit;
}

@media (hover: hover) {
  .toolbar button:hover {
    background: rgba(255, 255, 255, 0.12);
  }
}

.scale {
  min-width: 48px;
  text-align: center;
  font-size: var(--t-caption);
  font-variant-numeric: tabular-nums;
}

.hint {
  position: absolute;
  inset-inline-start: 50%;
  bottom: calc(var(--s-4) + var(--safe-bottom));
  transform: translateX(-50%);
  color: rgba(255, 255, 255, 0.66);
  font-size: var(--t-caption);
  white-space: nowrap;
}

.viewer-enter-active,
.viewer-leave-active {
  transition: opacity var(--m-2) var(--ease);
}

.viewer-enter-from,
.viewer-leave-to {
  opacity: 0;
}
</style>