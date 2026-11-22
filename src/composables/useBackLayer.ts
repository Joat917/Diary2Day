import { onBeforeUnmount, watch, type WatchSource } from "vue";

type BackHandler = () => void;

const layers: BackHandler[] = [];

export function handleBackLayer(): boolean {
  const handler = layers.pop();
  if (!handler) return false;
  handler();
  return true;
}

export function useBackLayer(active: WatchSource<boolean>, close: BackHandler) {
  let release: (() => void) | null = null;
  watch(
    active,
    (open) => {
      release?.();
      release = null;
      if (!open) return;
      layers.push(close);
      release = () => {
        const index = layers.lastIndexOf(close);
        if (index >= 0) layers.splice(index, 1);
      };
    },
    { immediate: true },
  );
  onBeforeUnmount(() => release?.());
}