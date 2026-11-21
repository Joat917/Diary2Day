import { computed, ref } from "vue";

const narrowMedia = window.matchMedia("(max-width: 599px)");
const wideMedia = window.matchMedia("(min-width: 960px)");

const isNarrow = ref(narrowMedia.matches);
const isWide = ref(wideMedia.matches);

let booted = false;

function boot() {
  if (booted) return;
  booted = true;
  narrowMedia.addEventListener("change", (event) => {
    isNarrow.value = event.matches;
  });
  wideMedia.addEventListener("change", (event) => {
    isWide.value = event.matches;
  });
}

export function useBreakpoint() {
  boot();
  return {
    isNarrow,
    isWide,
    isMedium: computed(() => !isNarrow.value && !isWide.value),
  };
}