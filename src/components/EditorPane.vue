<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from "vue";
import Vditor from "vditor";
import { t } from "../i18n";

const props = defineProps<{ modelValue: string }>();
const emit = defineEmits<{ "update:modelValue": [value: string]; ready: [] }>();

const host = ref<HTMLDivElement | null>(null);
let editor: Vditor | null = null;
let lastValue = props.modelValue;

watch(
  () => props.modelValue,
  (value) => {
    if (value === lastValue || !editor) return;
    lastValue = value;
    if (editor.getValue() !== value) editor.setValue(value, true);
  },
);

function patchToolbar(element: HTMLElement) {
  const toolbar = element.querySelector(".vditor-toolbar");
  if (!(toolbar instanceof HTMLElement)) return;
  toolbar.onmousedown = (event) => {
    if ((event.target as HTMLElement).closest(".keep-focus")) return;
    event.preventDefault();
  };
}

onMounted(() => {
  if (!host.value) return;
  editor = new Vditor(host.value, {
    mode: "ir",
    cdn: "/vditor",
    value: props.modelValue,
    height: "100%",
    cache: { enable: false },
    placeholder: t("today.placeholder"),
    theme: document.documentElement.dataset.theme === "dark" ? "dark" : "classic",
    toolbar: [
      "headings",
      "bold",
      "italic",
      "strike",
      "|",
      "list",
      "ordered-list",
      "check",
      "|",
      "quote",
      "code",
      "inline-code",
      "link",
      "table",
      "|",
      "undo",
      "redo",
    ],
    toolbarConfig: { hide: false, pin: true },
    counter: { enable: false },
    resize: { enable: false },
    input: (value) => {
      lastValue = value;
      emit("update:modelValue", value);
    },
    after: () => {
      if (host.value) patchToolbar(host.value);
      emit("ready");
    },
  });
});

onBeforeUnmount(() => {
  editor?.destroy();
  editor = null;
});

defineExpose({
  focus: () => editor?.focus(),
});
</script>

<template>
  <div ref="host" class="editor" />
</template>

<style scoped>
.editor {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
  background: var(--c-surface-1);
  border: 1px solid var(--c-hairline);
  border-radius: var(--r-md);
  overflow: hidden;
}

.editor :deep(.vditor) {
  height: 100%;
  min-height: 0;
}

.editor :deep(.vditor-content) {
  height: auto;
  flex: 1;
  min-height: 0;
}

.editor :deep(.vditor-ir) {
  height: 100%;
  overflow-y: auto;
  padding: var(--s-3) var(--s-4) var(--s-8);
}
</style>