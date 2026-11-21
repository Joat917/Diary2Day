import { convertFileSrc } from "@tauri-apps/api/core";

export type AssetUiKind = "image" | "audio" | "video";

const KIND_BY_EXT: Record<string, AssetUiKind> = {
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

const LOGICAL_REF = /ddasset:\/\/(image|audio|video|t)\/([A-Za-z0-9_-]+\.[a-z0-9]{1,5})/g;
const LEGACY_PREFIX = "http://ddasset.localhost/";

let assetsHref = "";
let thumbsHref = "";

function fileSrc(path: string): string {
  // The app data dir can contain parentheses; keep them encoded so they stay
  // inside markdown link destinations.
  return convertFileSrc(path)
    .replace(/\(/g, "%28")
    .replace(/\)/g, "%29");
}

function escapeRegExp(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

export function setAssetBase(dir: string | null | undefined): void {
  const root = (dir ?? "").trim();
  if (!root) {
    assetsHref = "";
    thumbsHref = "";
    return;
  }
  const sep = root.includes("\\") ? "\\" : "/";
  const base = root.endsWith("\\") || root.endsWith("/") ? root : root + sep;
  assetsHref = fileSrc(base);
  thumbsHref = fileSrc(`${base}.thumbs${sep}`);
}

export function assetUrl(kind: "image" | "audio" | "video" | "t", name: string): string | null {
  if (kind === "t") return thumbsHref ? `${thumbsHref}${name}` : null;
  return assetsHref ? `${assetsHref}${name}` : null;
}

export function assetFileUrl(name: string | null | undefined): string | null {
  if (!name) return null;
  return assetUrl("image", name);
}

export function thumbUrl(name: string | null | undefined): string | null {
  if (!name) return null;
  const stem = name.replace(/\.[^.]+$/, "");
  return assetUrl("t", `${stem}.webp`);
}

export function assetMarkdown(name: string, kind: AssetUiKind): string {
  const url = `ddasset://${kind}/${name}`;
  if (kind === "image") return `![](${url})`;
  return `<${kind} controls src="${url}"></${kind}>`;
}

export function expandAssetUrls(text: string): string {
  if (!assetsHref) return text;
  const source = text.includes(LEGACY_PREFIX)
    ? text.split(LEGACY_PREFIX).join("ddasset://")
    : text;
  return source.replace(LOGICAL_REF, (whole, kind: string, name: string) => {
    return assetUrl(kind as "image" | "audio" | "video" | "t", name) ?? whole;
  });
}

export function collapseAssetUrls(text: string): string {
  if (!assetsHref || !text.includes(assetsHref)) return text;
  const pattern = new RegExp(
    `${escapeRegExp(assetsHref)}([A-Za-z0-9_-]+\\.[a-z0-9]{1,5})`,
    "g",
  );
  return text.replace(pattern, (whole, file: string) => {
    const ext = file.slice(file.lastIndexOf(".") + 1).toLowerCase();
    const kind = KIND_BY_EXT[ext];
    return kind ? `ddasset://${kind}/${file}` : whole;
  });
}

export function isAssetSrc(src: string): boolean {
  if (!src) return false;
  if (assetsHref && src.startsWith(assetsHref)) return true;
  if (thumbsHref && src.startsWith(thumbsHref)) return true;
  return src.startsWith("ddasset://") || src.startsWith(LEGACY_PREFIX);
}

export function referencedAssetNames(text: string): string[] {
  if (!text.includes("ddasset://") && !text.includes(LEGACY_PREFIX)) return [];
  const source = text.split(LEGACY_PREFIX).join("ddasset://");
  const names = new Set<string>();
  for (const match of source.matchAll(new RegExp(LOGICAL_REF.source, "g"))) {
    if (match[1] !== "t") names.add(match[2]);
  }
  return [...names];
}