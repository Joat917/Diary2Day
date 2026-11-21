import { mediaImport } from "../ipc/client";
import type { Asset } from "../ipc/types";
import type { MessageKey } from "../i18n";

export function isMediaFile(file: File): boolean {
  return /^(image|audio|video)\//.test(file.type);
}

export function insertLabelKey(files: File[]): MessageKey {
  const kinds = new Set(files.map((file) => file.type.slice(0, file.type.indexOf("/"))));
  if (kinds.size === 1) {
    if (kinds.has("image")) return "editor.insertingImage";
    if (kinds.has("video")) return "editor.insertingVideo";
    if (kinds.has("audio")) return "editor.insertingAudio";
  }
  return "editor.insertingMedia";
}

function fallbackName(file: File): string {
  if (file.name) return file.name;
  if (file.type.startsWith("image/")) return "pasted.png";
  if (file.type.startsWith("audio/")) return "audio.mp3";
  return "video.mp4";
}

function readFile(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(String(reader.result ?? ""));
    reader.onerror = () => reject(reader.error);
    reader.readAsDataURL(file);
  });
}

export async function importMediaFiles(files: File[]): Promise<Asset[]> {
  const media = files.filter(isMediaFile);
  if (!media.length) return [];
  const payload = await Promise.all(
    media.map(async (file) => ({
      origName: fallbackName(file),
      data: await readFile(file),
    })),
  );
  return mediaImport(payload);
}