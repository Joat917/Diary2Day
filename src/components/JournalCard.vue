<script setup lang="ts">
import { computed } from "vue";
import { mdiImageMultipleOutline, mdiPin } from "@mdi/js";
import type { JournalSummary } from "../ipc/types";
import { t } from "../i18n";
import { moodByCode } from "../constants/moods";
import { formatCardDate } from "../utils/date";
import Icon from "./Icon.vue";

const props = withDefaults(
  defineProps<{
    item: JournalSummary;
    variant?: "stack" | "result";
    needle?: string;
  }>(),
  { variant: "stack", needle: "" },
);

defineEmits<{ open: [id: string] }>();

const title = computed(() => props.item.title || t("common.untitled"));
const mood = computed(() => moodByCode(props.item.mood));
const hasMeta = computed(
  () =>
    Boolean(props.item.words) ||
    props.item.asset_n > 0 ||
    props.item.pinned ||
    !!mood.value,
);

const previewParts = computed(() => {
  const text = props.item.preview;
  const needle = props.needle.trim();
  if (!needle) return [{ text, hit: false }];
  const lower = text.toLowerCase();
  const target = needle.toLowerCase();
  const parts: { text: string; hit: boolean }[] = [];
  let start = 0;
  let index = lower.indexOf(target);
  while (index !== -1 && parts.length < 40) {
    if (index > start) parts.push({ text: text.slice(start, index), hit: false });
    parts.push({ text: text.slice(index, index + needle.length), hit: true });
    start = index + needle.length;
    index = lower.indexOf(target, start);
  }
  if (start < text.length) parts.push({ text: text.slice(start), hit: false });
  return parts;
});
</script>

<template>
  <article
    class="card"
    :class="variant"
    role="button"
    tabindex="0"
    @click="$emit('open', item.id)"
    @keydown.enter.prevent="$emit('open', item.id)"
    @keydown.space.prevent="$emit('open', item.id)"
  >
    <h3 class="title">{{ title }}</h3>
    <p v-if="item.preview" class="preview">
      <template v-for="(part, index) in previewParts" :key="index">
        <mark v-if="part.hit" class="hit">{{ part.text }}</mark>
        <template v-else>{{ part.text }}</template>
      </template>
    </p>
    <div v-if="hasMeta" class="meta">
      <span class="time">{{ formatCardDate(item.recorded_at, item.recorded_off) }}</span>
      <span v-if="mood" class="mood">
        <Icon :path="mood.path" :size="16" :label="t(mood.labelKey)" />
      </span>
      <span v-if="item.words">{{ t("today.words", { n: item.words }) }}</span>
      <span v-if="item.asset_n > 0" class="badge">
        <Icon :path="mdiImageMultipleOutline" :size="14" />
        {{ item.asset_n }}
      </span>
      <span v-if="item.pinned" class="pin">
        <Icon :path="mdiPin" :size="14" :label="t('detail.pinned')" />
      </span>
    </div>
  </article>
</template>

<style scoped>
.card {
  display: block;
  padding: var(--s-4);
  border-radius: var(--r-md);
  background: var(--c-surface-1);
  border: 1px solid var(--c-hairline);
  cursor: pointer;
  transition: background var(--m-1) var(--ease), border-color var(--m-1) var(--ease);
}

@media (hover: hover) {
  .card:hover {
    border-color: var(--c-on-surface-faint);
  }
}

.card.result {
  border: none;
  border-bottom: 1px solid var(--c-hairline);
  border-radius: 0;
  background: transparent;
  padding-inline: 0;
}

.title {
  font-size: var(--t-title);
  line-height: var(--t-title-lh);
  font-weight: 500;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.preview {
  margin-top: var(--s-1);
  font-size: var(--t-body);
  line-height: 1.6;
  color: var(--c-on-surface-muted);
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.hit {
  background: var(--c-hit);
  color: inherit;
  border-radius: 2px;
}

.meta {
  margin-top: var(--s-2);
  display: flex;
  align-items: center;
  gap: var(--s-2);
  font-size: var(--t-caption);
  line-height: var(--t-caption-lh);
  color: var(--c-on-surface-muted);
}

.badge,
.pin {
  display: inline-flex;
  align-items: center;
  gap: 2px;
}

.mood {
  display: inline-flex;
  color: var(--c-on-surface-muted);
}

.pin {
  color: var(--c-accent);
}
</style>