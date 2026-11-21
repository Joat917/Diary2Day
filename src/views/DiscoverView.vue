<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useRouter } from "vue-router";
import { mdiClose, mdiMagnify } from "@mdi/js";
import { journalList, onDataChanged, searchQuery, tagList } from "../ipc/client";
import type { JournalSummary, Tag } from "../ipc/types";
import { t } from "../i18n";
import Icon from "../components/Icon.vue";
import JournalCard from "../components/JournalCard.vue";
import StateBlock from "../components/StateBlock.vue";

const router = useRouter();
const text = ref("");
const results = ref<JournalSummary[]>([]);
const partial = ref(false);
const loading = ref(true);
const error = ref(false);
const tags = ref<Tag[]>([]);
const tagId = ref<string | null>(null);

let timer: number | undefined;
let token = 0;

async function run() {
  const current = ++token;
  loading.value = true;
  error.value = false;
  try {
    if (!text.value.trim() && tagId.value) {
      const page = await journalList({ tagId: tagId.value, limit: 100 });
      if (current !== token) return;
      results.value = page.items;
      partial.value = false;
      return;
    }
    const page = await searchQuery(text.value.trim(), { limit: 100 });
    if (current !== token) return;
    results.value = page.items;
    partial.value = page.partial;
  } catch {
    if (current === token) error.value = true;
  } finally {
    if (current === token) loading.value = false;
  }
}

async function loadTags() {
  try {
    tags.value = await tagList();
  } catch {
    tags.value = [];
  }
}

function pickTag(id: string | null) {
  tagId.value = id;
  void run();
}

watch(text, () => {
  window.clearTimeout(timer);
  timer = window.setTimeout(() => void run(), 250);
});

onMounted(() => {
  void run();
  void loadTags();
});

let unlisten: (() => void) | null = null;
onMounted(async () => {
  unlisten = await onDataChanged(() => {
    void loadTags();
    void run();
  });
});

onBeforeUnmount(() => {
  window.clearTimeout(timer);
  unlisten?.();
});
</script>

<template>
  <div class="view">
    <header class="topbar">
      <h1 class="heading">{{ t("nav.discover") }}</h1>
      <div class="search">
        <Icon :path="mdiMagnify" :size="18" class="search-icon" />
        <input
          v-model="text"
          class="input"
          type="search"
          :placeholder="t('discover.placeholder')"
          autocomplete="off"
          spellcheck="false"
        />
        <button
          v-if="text"
          class="clear"
          type="button"
          :aria-label="t('common.close')"
          @click="text = ''"
        >
          <Icon :path="mdiClose" :size="16" />
        </button>
      </div>
      <p v-if="partial" class="partial">{{ t("discover.partial") }}</p>

      <div v-if="!text.trim() && tags.length" class="tag-bar">
        <button
          class="tag-chip"
          :class="{ active: !tagId }"
          type="button"
          @click="pickTag(null)"
        >
          {{ t("discover.allTags") }}
        </button>
        <button
          v-for="tag in tags"
          :key="tag.id"
          class="tag-chip"
          :class="{ active: tagId === tag.id }"
          type="button"
          @click="pickTag(tag.id)"
        >
          #{{ tag.name }}
          <span class="usage">{{ tag.usage }}</span>
        </button>
      </div>
    </header>

    <div class="scroll">
      <StateBlock v-if="loading && !results.length" kind="loading" />
      <StateBlock
        v-else-if="error"
        kind="error"
        :title="t('common.error')"
        :action-label="t('common.retry')"
        @action="run"
      />
      <StateBlock
        v-else-if="!results.length"
        kind="empty"
        :title="
          text ? t('discover.empty') : tagId ? t('discover.emptyTag') : t('today.empty')
        "
        :hint="text ? t('discover.emptyHint') : undefined"
      />
      <template v-else>
        <p class="section">
          {{
            text
              ? t("discover.placeholder")
              : tagId
                ? `#${tags.find((tag) => tag.id === tagId)?.name ?? ""}`
                : t("discover.recent")
          }}
        </p>
        <JournalCard
          v-for="item in results"
          :key="item.id"
          :item="item"
          variant="result"
          :needle="text.trim()"
          @open="
            router.push({
              path: `/j/${item.id}`,
              query: text.trim() ? { q: text.trim() } : {},
            })
          "
        />
      </template>
    </div>
  </div>
</template>

<style scoped>
.view {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
}

.topbar {
  padding: var(--s-3) var(--s-4);
  border-bottom: 1px solid var(--c-hairline);
  background: var(--c-surface);
}

.heading {
  font-size: var(--t-display);
  line-height: var(--t-display-lh);
  font-weight: 500;
  margin-bottom: var(--s-2);
}

.search {
  display: flex;
  align-items: center;
  gap: var(--s-2);
  height: 44px;
  padding-inline: var(--s-3);
  border-radius: var(--r-md);
  background: var(--c-surface-2);
}

.search-icon {
  color: var(--c-on-surface-muted);
  flex: none;
}

.input {
  flex: 1;
  min-width: 0;
  border: none;
  outline: none;
  background: transparent;
  font-size: var(--t-body);
}

.input::placeholder {
  color: var(--c-on-surface-faint);
}

.clear {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border-radius: var(--r-pill);
  color: var(--c-on-surface-muted);
}

.partial {
  margin-top: var(--s-1);
  font-size: var(--t-caption);
  line-height: var(--t-caption-lh);
  color: var(--c-on-surface-muted);
}

.tag-bar {
  display: flex;
  flex-wrap: wrap;
  gap: var(--s-2);
  margin-top: var(--s-2);
}

.tag-chip {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  min-height: 32px;
  padding-inline: var(--s-2);
  border-radius: var(--r-sm);
  background: var(--c-surface-2);
  color: var(--c-on-surface-muted);
  font-size: var(--t-small);
  line-height: var(--t-small-lh);
}

.tag-chip.active {
  background: var(--c-primary-soft);
  color: var(--c-primary);
}

.usage {
  color: var(--c-on-surface-faint);
  font-size: var(--t-caption);
}

.scroll {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  scrollbar-gutter: stable;
  padding: 0 var(--s-4) var(--s-6);
}

.section {
  padding: var(--s-3) 0 var(--s-1);
  font-size: var(--t-caption);
  line-height: var(--t-caption-lh);
  color: var(--c-on-surface-muted);
}
</style>