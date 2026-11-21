import { ref, type Ref } from "vue";

export type SaveState = "clean" | "dirty" | "saving" | "error";

export function useAutosave(
  getValue: () => string,
  save: (value: string) => Promise<void>,
  delay = 1000,
): {
  state: Ref<SaveState>;
  schedule: () => void;
  flush: () => Promise<void>;
  flushQuiet: () => Promise<void>;
} {
  const state = ref<SaveState>("clean");
  let timer: number | undefined;
  let inFlight: Promise<void> | null = null;
  let dirty = false;
  let queued = false;

  function clearTimer() {
    if (timer !== undefined) {
      window.clearTimeout(timer);
      timer = undefined;
    }
  }

  function schedule() {
    dirty = true;
    state.value = "dirty";
    clearTimer();
    timer = window.setTimeout(() => {
      void flush();
    }, delay);
  }

  async function flush() {
    clearTimer();
    if (!dirty) {
      if (inFlight) await inFlight;
      return;
    }
    if (inFlight) {
      queued = true;
      await inFlight;
      if (queued) {
        queued = false;
        if (dirty) await flush();
      }
      return;
    }

    dirty = false;
    state.value = "saving";
    inFlight = (async () => {
      try {
        await save(getValue());
        state.value = "clean";
      } catch {
        state.value = "error";
        dirty = true;
      } finally {
        inFlight = null;
      }
    })();
    await inFlight;
    if (queued) {
      queued = false;
      if (dirty) await flush();
    }
  }

  async function flushQuiet() {
    try {
      await flush();
    } catch {
      /* 保存失败已反映在 state 上，导航不因此中断 */
    }
  }

  return { state, schedule, flush, flushQuiet };
}