import Vditor from "vditor";
import { expandAssetUrls } from "./assetUrl";

export interface NotePreviewSettings {
  dark: boolean;
  lazyImages: boolean;
}

function previewOptions({ dark, lazyImages }: NotePreviewSettings): IPreviewOptions {
  return {
    cdn: "/vditor",
    mode: dark ? "dark" : "light",
    theme: { current: dark ? "dark" : "light", path: "/vditor/dist/css/content-theme" },
    hljs: { style: dark ? "native" : "github", lineNumber: false },
    anchor: 0,
    ...(lazyImages ? { lazyLoadImage: "lazy" } : {}),
  };
}

export function renderNotePreview(
  element: HTMLElement,
  markdown: string,
  settings: NotePreviewSettings,
): Promise<void> {
  element.innerHTML = "";
  if (!markdown.trim()) return Promise.resolve();
  element.classList.add("vditor-reset");
  return Vditor.preview(
    element as HTMLDivElement,
    expandAssetUrls(markdown),
    previewOptions(settings),
  );
}