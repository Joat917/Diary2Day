<script setup lang="ts">
import { nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { t } from "../i18n";
import { useBackLayer } from "../composables/useBackLayer";

const props = withDefaults(
  defineProps<{
    open: boolean;
    title: string;
    message: string;
    confirmLabel?: string;
    cancelLabel?: string;
    danger?: boolean;
  }>(),
  { confirmLabel: undefined, cancelLabel: undefined, danger: false },
);

const emit = defineEmits<{ confirm: []; cancel: [] }>();

useBackLayer(
  () => props.open,
  () => emit("cancel"),
);

const cancelButton = ref<HTMLButtonElement | null>(null);
let previousFocus: HTMLElement | null = null;

watch(
  () => props.open,
  async (open) => {
    if (open) {
      previousFocus =
        document.activeElement instanceof HTMLElement ? document.activeElement : null;
      await nextTick();
      cancelButton.value?.focus();
    } else if (previousFocus) {
      previousFocus.focus();
      previousFocus = null;
    }
  },
);

function onKeydown(event: KeyboardEvent) {
  if (!props.open) return;
  if (event.key === "Escape") {
    event.preventDefault();
    emit("cancel");
  }
}

onMounted(() => window.addEventListener("keydown", onKeydown));
onBeforeUnmount(() => window.removeEventListener("keydown", onKeydown));
</script>

<template>
  <Teleport to="body">
    <Transition name="dialog">
      <div v-if="open" class="scrim" @click.self="emit('cancel')">
        <div class="dialog" role="alertdialog" aria-modal="true" :aria-label="title">
          <h2 class="title">{{ title }}</h2>
          <p class="message">{{ message }}</p>
          <div class="actions">
            <button
              ref="cancelButton"
              class="button"
              type="button"
              @click="emit('cancel')"
            >
              {{ cancelLabel ?? t("common.cancel") }}
            </button>
            <button
              class="button solid"
              :class="{ danger }"
              type="button"
              @click="emit('confirm')"
            >
              {{ confirmLabel ?? t("common.confirm") }}
            </button>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.scrim {
  position: fixed;
  inset: 0;
  z-index: 100;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: var(--s-6);
  padding-inline: calc(var(--s-6) + var(--safe-left)) calc(var(--s-6) + var(--safe-right));
  padding-bottom: calc(var(--s-6) + var(--ime));
  background: var(--c-scrim);
}

.dialog {
  width: min(100%, 360px);
  padding: var(--s-5);
  border-radius: var(--r-lg);
  background: var(--c-surface-1);
  border: 1px solid var(--c-hairline);
  box-shadow: var(--e-2);
}

.title {
  font-size: var(--t-title);
  line-height: var(--t-title-lh);
  font-weight: 500;
  color: var(--c-on-surface);
}

.message {
  margin-top: var(--s-2);
  font-size: var(--t-small);
  line-height: var(--t-small-lh);
  color: var(--c-on-surface-muted);
}

.actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--s-2);
  margin-top: var(--s-5);
}

.button {
  min-height: 38px;
  padding-inline: var(--s-4);
  border-radius: var(--r-pill);
  font-size: var(--t-small);
  color: var(--c-on-surface-muted);
  background: var(--c-surface-2);
}

.button.solid {
  background: var(--c-primary);
  color: var(--c-on-primary);
}

.button.solid.danger {
  background: var(--c-danger);
  color: var(--c-on-danger);
}

.dialog-enter-active,
.dialog-leave-active {
  transition: opacity var(--m-2) var(--ease);
}

.dialog-enter-active .dialog,
.dialog-leave-active .dialog {
  transition: transform var(--m-2) var(--ease);
}

.dialog-enter-from,
.dialog-leave-to {
  opacity: 0;
}

.dialog-enter-from .dialog,
.dialog-leave-to .dialog {
  transform: scale(0.96);
}
</style>
