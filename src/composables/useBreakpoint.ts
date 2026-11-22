import { computed, ref } from "vue";

const narrowMedia = window.matchMedia("(max-width: 599px)");
// Short viewports (landscape phones) take the wide shell: the bottom nav and the
// stacked week strip cost too much of a ~400px-tall screen. The 720 floor keeps
// enough width for the 184 rail + 300 calendar + a usable list column.
const wideMedia = window.matchMedia(
  "(min-width: 960px), (min-width: 720px) and (max-height: 559px)",
);
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