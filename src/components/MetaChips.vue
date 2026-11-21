<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import {
  mdiCheck,
  mdiClose,
  mdiEmoticonOutline,
  mdiPlus,
  mdiWeatherPartlyCloudy,
} from "@mdi/js";
import type { Tag } from "../ipc/types";
import { IpcError, tagCreate, tagList } from "../ipc/client";
import { t } from "../i18n";
import { MOODS, moodByCode } from "../constants/moods";
import { WEATHERS, weatherByCode } from "../constants/weather";
import Icon from "./Icon.vue";

const props = defineProps<{
  tags: Tag[];
  mood: string | null;
  weather: string | null;
  editable: boolean;
}>();

const emit = defineEmits<{
  "update:tags": [tags: Tag[]];
  "update:mood": [code: string | null];
  "update:weather": [code: string | null];
}>();

const panel = ref<"mood" | "weather" | "tags" | null>(null);
const suggestions = ref<Tag[]>([]);
const draft = ref("");
const busy = ref(false);
const root = ref<HTMLElement | null>(null);
const input = ref<HTMLInputElement | null>(null);

const attachedIds = computed(() => new Set(props.tags.map((tag) => tag.id)));
const currentMood = computed(() => moodByCode(props.mood));
const currentWeather = computed(() => weatherByCode(props.weather));
const hasMeta = computed(
  () => props.tags.length > 0 || !!props.mood || !!props.weather,
);

const visibleSuggestions = computed(() => {
  const needle = draft.value.trim().toLowerCase();
  return suggestions.value
    .filter((tag) => !attachedIds.value.has(tag.id))
    .filter((tag) => !needle || tag.name.toLowerCase().includes(needle))
    .slice(0, 8);
});

function togglePanel(which: "mood" | "weather" | "tags") {
  if (!props.editable) return;
  const next = panel.value === which ? null : which;
  panel.value = next;
  if (next === "tags") {
    void loadSuggestions();
    draft.value = "";
    void nextTick(() => input.value?.focus());
  }
}

async function loadSuggestions() {
  try {
    suggestions.value = await tagList();
  } catch {
    suggestions.value = [];
  }
}

function close() {
  panel.value = null;
  draft.value = "";
}

function onDocumentPointer(event: PointerEvent) {
  if (!panel.value) return;
  if (root.value && !root.value.contains(event.target as Node)) close();
}

function onDocumentKey(event: KeyboardEvent) {
  if (event.key === "Escape" && panel.value) {
    event.stopPropagation();
    close();
  }
}

watch(panel, (value) => {
  if (value) document.addEventListener("pointerdown", onDocumentPointer, true);
  else document.removeEventListener("pointerdown", onDocumentPointer, true);
});

onMounted(() => {
  document.addEventListener("keydown", onDocumentKey, true);
});

onBeforeUnmount(() => {
  document.removeEventListener("pointerdown", onDocumentPointer, true);
  document.removeEventListener("keydown", onDocumentKey, true);
});

function pickMood(code: string | null) {
  emit("update:mood", code);
  close();
}

function pickWeather(code: string | null) {
  emit("update:weather", code);
  close();
}

function attach(tag: Tag) {
  if (attachedIds.value.has(tag.id)) return;
  emit("update:tags", [...props.tags, tag]);
  draft.value = "";
  void nextTick(() => input.value?.focus());
}

function detach(tag: Tag) {
  emit(
    "update:tags",
    props.tags.filter((item) => item.id !== tag.id),
  );
}

async function submitDraft() {
  const name = draft.value.trim();
  if (!name || busy.value) return;
  const existing = suggestions.value.find(
    (tag) => tag.name.trim().toLowerCase() === name.toLowerCase(),
  );
  if (existing) {
    attach(existing);
    return;
  }
  busy.value = true;
  try {
    const created = await tagCreate(name);
    suggestions.value = [created, ...suggestions.value];
    attach(created);
  } catch (error) {
    if (error instanceof IpcError && error.code === "tag_exists") {
      await loadSuggestions();
      const match = suggestions.value.find(
        (tag) => tag.name.trim().toLowerCase() === name.toLowerCase(),
      );
      if (match) attach(match);
    }
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <div v-if="hasMeta || editable" ref="root" class="meta-chips">
    <button
      v-if="editable"
      class="chip"
      :class="{ active: !!currentMood }"
      type="button"
      :aria-label="currentMood ? t(currentMood.labelKey) : t('meta.mood')"
      @click="togglePanel('mood')"
    >
      <Icon :path="currentMood?.path ?? mdiEmoticonOutline" :size="16" />
      <span>{{ currentMood ? t(currentMood.labelKey) : t("meta.mood") }}</span>
    </button>
    <span v-else-if="currentMood" class="chip static">
      <Icon :path="currentMood.path" :size="16" />
      <span>{{ t(currentMood.labelKey) }}</span>
    </span>

    <button
      v-if="editable"
      class="chip"
      :class="{ active: !!currentWeather }"
      type="button"
      :aria-label="currentWeather ? t(currentWeather.labelKey) : t('meta.weather')"
      @click="togglePanel('weather')"
    >
      <Icon :path="currentWeather?.path ?? mdiWeatherPartlyCloudy" :size="16" />
      <span>{{ currentWeather ? t(currentWeather.labelKey) : t("meta.weather") }}</span>
    </button>
    <span v-else-if="currentWeather" class="chip static">
      <Icon :path="currentWeather.path" :size="16" />
      <span>{{ t(currentWeather.labelKey) }}</span>
    </span>

    <span v-for="tag in tags" :key="tag.id" class="chip static">
      <span>#{{ tag.name }}</span>
      <button
        v-if="editable"
        class="chip-remove"
        type="button"
        :aria-label="t('meta.removeTag', { name: tag.name })"
        @click="detach(tag)"
      >
        <Icon :path="mdiClose" :size="14" />
      </button>
    </span>

    <button
      v-if="editable"
      class="chip add"
      type="button"
      :aria-label="t('meta.addTag')"
      @click="togglePanel('tags')"
    >
      <Icon :path="mdiPlus" :size="16" />
    </button>

    <div v-if="panel" class="panel" role="dialog">
      <div class="panel-head">
        <span>{{
          panel === "mood"
            ? t("meta.mood")
            : panel === "weather"
              ? t("meta.weather")
              : t("meta.tags")
        }}</span>
        <button class="panel-close" type="button" :aria-label="t('common.close')" @click="close">
          <Icon :path="mdiClose" :size="16" />
        </button>
      </div>

      <div v-if="panel === 'mood'" class="mood-grid">
        <button
          class="mood"
          type="button"
          :class="{ active: !mood }"
          :aria-label="t('meta.noMood')"
          @click="pickMood(null)"
        >
          <Icon :path="mdiClose" :size="20" />
        </button>
        <button
          v-for="item in MOODS"
          :key="item.code"
          class="mood"
          type="button"
          :class="{ active: item.code === mood }"
          :aria-label="t(item.labelKey)"
          :title="t(item.labelKey)"
          @click="pickMood(item.code)"
        >
          <Icon :path="item.path" :size="24" />
          <Icon v-if="item.code === mood" class="mood-check" :path="mdiCheck" :size="12" />
        </button>
      </div>

      <div v-else-if="panel === 'weather'" class="mood-grid">
        <button
          class="mood"
          type="button"
          :class="{ active: !weather }"
          :aria-label="t('meta.noWeather')"
          @click="pickWeather(null)"
        >
          <Icon :path="mdiClose" :size="20" />
        </button>
        <button
          v-for="item in WEATHERS"
          :key="item.code"
          class="mood"
          type="button"
          :class="{ active: item.code === weather }"
          :aria-label="t(item.labelKey)"
          :title="t(item.labelKey)"
          @click="pickWeather(item.code)"
        >
          <Icon :path="item.path" :size="24" />
          <Icon v-if="item.code === weather" class="mood-check" :path="mdiCheck" :size="12" />
        </button>
      </div>

      <div v-else class="tag-panel">
        <input
          ref="input"
          v-model="draft"
          class="tag-input"
          type="text"
          :placeholder="t('meta.tagPlaceholder')"
          @keydown.enter.prevent="submitDraft"
        />
        <button
          v-if="draft.trim()"
          class="tag-create"
          type="button"
          :disabled="busy"
          @click="submitDraft"
        >
          {{ t("meta.createTag", { name: draft.trim() }) }}
        </button>
        <button
          v-for="tag in visibleSuggestions"
          :key="tag.id"
          class="tag-row"
          type="button"
          @click="attach(tag)"
        >
          <span>#{{ tag.name }}</span>
          <span class="tag-usage">{{ tag.usage }}</span>
        </button>
        <p v-if="!visibleSuggestions.length && !draft.trim()" class="tag-hint">
          {{ t("meta.tagHint") }}
        </p>
      </div>
    </div>
  </div>
</template>

<style scoped>
.meta-chips {
  position: relative;
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--s-2);
  padding: var(--s-2) var(--s-4);
  border-bottom: 1px solid var(--c-hairline);
}

.chip {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  min-height: 32px;
  padding-inline: var(--s-2);
  border-radius: var(--r-sm);
  background: var(--c-surface-2);
  color: var(--c-on-surface-muted);
  font-size: var(--t-small);
  line-height: var(--t-small-lh);
}

.chip.active {
  background: var(--c-primary-soft);
  color: var(--c-primary);
}

.chip.add {
  justify-content: center;
  width: 32px;
  padding-inline: 0;
  color: var(--c-on-surface-muted);
}

@media (hover: hover) {
  .chip.add:hover,
  .tag-row:hover,
  .mood:hover {
    background: var(--c-primary-soft);
  }
}

.chip-remove {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  margin-inline-end: -4px;
  border-radius: var(--r-pill);
  color: var(--c-on-surface-faint);
}

.panel {
  position: absolute;
  inset-inline-start: var(--s-4);
  top: calc(100% + var(--s-1));
  z-index: 20;
  width: min(320px, calc(100vw - var(--s-8)));
  padding: var(--s-3);
  border: 1px solid var(--c-hairline);
  border-radius: var(--r-md);
  background: var(--c-surface-1);
  box-shadow: var(--e-2);
}

.panel-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: var(--s-2);
  font-size: var(--t-small);
  color: var(--c-on-surface-muted);
}

.panel-close {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border-radius: var(--r-pill);
  color: var(--c-on-surface-muted);
}

.mood-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(40px, 1fr));
  gap: var(--s-1);
}

.mood {
  position: relative;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  height: 40px;
  border-radius: var(--r-sm);
  color: var(--c-on-surface-muted);
}

.mood.active {
  background: var(--c-primary-soft);
  color: var(--c-primary);
}

.mood-check {
  position: absolute;
  inset-block-end: 2px;
  inset-inline-end: 2px;
  color: var(--c-primary);
}

.tag-panel {
  display: flex;
  flex-direction: column;
  gap: var(--s-1);
  max-height: 260px;
  overflow-y: auto;
}

.tag-input {
  height: 38px;
  padding-inline: var(--s-2);
  border: 1px solid var(--c-hairline);
  border-radius: var(--r-sm);
  background: var(--c-surface-2);
  color: var(--c-on-surface);
  font-size: var(--t-small);
}

.tag-input:focus-visible {
  outline: 2px solid var(--c-primary);
  outline-offset: 1px;
}

.tag-create {
  align-self: flex-start;
  padding: var(--s-1) var(--s-2);
  border-radius: var(--r-sm);
  background: var(--c-primary);
  color: var(--c-on-primary);
  font-size: var(--t-small);
}

.tag-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  min-height: 38px;
  padding-inline: var(--s-2);
  border-radius: var(--r-sm);
  color: var(--c-on-surface);
  font-size: var(--t-small);
  text-align: start;
}

.tag-usage {
  color: var(--c-on-surface-faint);
  font-size: var(--t-caption);
}

.tag-hint {
  margin: var(--s-1) 0 0;
  color: var(--c-on-surface-faint);
  font-size: var(--t-caption);
  line-height: var(--t-caption-lh);
}
</style>