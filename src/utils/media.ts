import { mediaImportPaths, mediaPasteImport, mediaPickImport } from "../ipc/client";
import type { Asset, AssetKind } from "../ipc/types";
import type { MessageKey } from "../i18n";

const KIND_BY_EXT: Record<string, AssetKind> = {
  jpg: "image",
  jpeg: "image",
  png: "image",
  gif: "image",
  webp: "image",
  bmp: "image",
  avif: "image",
  mp3: "audio",
  wav: "audio",
  ogg: "audio",
  m4a: "audio",
  aac: "audio",
  flac: "audio",
  mp4: "video",
  webm: "video",
  mov: "video",
  mkv: "video",
};

function extOf(name: string): string {
  const base = name.split(/[\\/]/).pop() ?? "";
  return base.includes(".") ? base.split(".").pop()!.toLowerCase() : "";
}

export function mediaKindOfPath(path: string): AssetKind | null {
  return KIND_BY_EXT[extOf(path)] ?? null;
}

export function isMediaFile(file: File): boolean {
  if (/^(image|audio|video)\//.test(file.type)) return true;
  return extOf(file.name) in KIND_BY_EXT;
}

export function insertLabelKey(kinds: AssetKind[]): MessageKey {
  const set = new Set(kinds);
  if (set.size === 1) {
    if (set.has("image")) return "editor.insertingImage";
    if (set.has("video")) return "editor.insertingVideo";
    if (set.has("audio")) return "editor.insertingAudio";
  }
  return "editor.insertingMedia";
}

export function pickMedia(): Promise<Asset[]> {
  return mediaPickImport();
}

export function pasteMedia(): Promise<Asset[]> {
  return mediaPasteImport();
}

export function importMediaPaths(paths: string[]): Promise<Asset[]> {
  return mediaImportPaths(paths);
}