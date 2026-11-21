import { defineStore } from "pinia";
import { ref } from "vue";
import {
  appLockClear,
  appLockEngage,
  appLockSet,
  appLockStatus,
  appLockUnlock,
  onLockState,
} from "../ipc/client";
import type { LockState } from "../ipc/types";

export const useLock = defineStore("lock", () => {
  const enabled = ref(false);
  const locked = ref(false);
  const ready = ref(false);

  function apply(state: LockState) {
    enabled.value = state.enabled;
    locked.value = state.locked;
  }

  async function load() {
    try {
      apply(await appLockStatus());
    } catch (error) {
      console.error("[lock] status load failed", error);
      enabled.value = false;
      locked.value = false;
    } finally {
      ready.value = true;
    }
  }

  async function listen() {
    const unlisten = await onLockState(apply);
    return () => unlisten();
  }

  async function unlock(code: string) {
    apply(await appLockUnlock(code));
  }

  async function setCode(code: string) {
    apply(await appLockSet(code));
  }

  async function clear(code: string) {
    apply(await appLockClear(code));
  }

  async function engage() {
    if (!enabled.value || locked.value) return;
    apply(await appLockEngage());
  }

  return { enabled, locked, ready, load, listen, unlock, setCode, clear, engage };
});