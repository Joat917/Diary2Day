import { computed, ref } from "vue";

const narrowMedia = window.matchMedia("(max-width: 599px)");
const wideMedia = window.matchMedia("(min-width: 960px)");
const threePaneMedia = window.matchMedia("(min-width: 1200px)");

const isNarrow = ref(narrowMedia.matches);
const isWide = ref(wideMedia.matches);
const isThreePane = ref(threePaneMedia.matches);

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
  threePaneMedia.addEventListener("change", (event) => {
    isThreePane.value = event.matches;
  });
}

export function useBreakpoint() {
  boot();
  return {
    isNarrow,
    isWide,
    isThreePane,
    isMedium: computed(() => !isNarrow.value && !isWide.value),
  };
}