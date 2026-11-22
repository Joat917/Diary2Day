import { toPng } from "html-to-image";
import { describeError, IpcError } from "../ipc/client";

// WebView canvas 单边上限（Chromium 16384）。超长日记按块切成多张，单张不降采样。
const MAX_EDGE = 16384;
const CHUNK_LIMIT = 7500;
const SETTLE_QUIET_MS = 250;
const SETTLE_TIMEOUT_MS = 4000;

export function waitForStable(
  root: HTMLElement,
  quietMs = SETTLE_QUIET_MS,
  timeoutMs = SETTLE_TIMEOUT_MS,
): Promise<void> {
  return new Promise((resolve) => {
    let quietTimer = 0;
    let deadline = 0;
    const finish = () => {
      observer.disconnect();
      window.clearTimeout(quietTimer);
      window.clearTimeout(deadline);
      resolve();
    };
    const observer = new MutationObserver(() => {
      window.clearTimeout(quietTimer);
      quietTimer = window.setTimeout(finish, quietMs);
    });
    observer.observe(root, {
      childList: true,
      subtree: true,
      attributes: true,
      characterData: true,
    });
    quietTimer = window.setTimeout(finish, quietMs);
    deadline = window.setTimeout(finish, timeoutMs);
  });
}

function blobToDataUrl(blob: Blob): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(String(reader.result));
    reader.onerror = () => reject(reader.error ?? new Error("file reader failed"));
    reader.readAsDataURL(blob);
  });
}

// foreignObject 不能引用外部资源：图片必须先变成 data URL，否则截出来是空白。
export async function inlineImages(root: HTMLElement): Promise<void> {
  const images = [...root.querySelectorAll("img")];
  await Promise.all(
    images.map(async (image) => {
      const src = image.getAttribute("src") ?? "";
      if (!src || src.startsWith("data:")) return;
      let blob: Blob;
      try {
        const response = await fetch(src);
        if (!response.ok) throw new Error(`HTTP ${response.status}`);
        blob = await response.blob();
      } catch (err) {
        throw new IpcError(
          "export_image",
          "无法读取正文里的图片",
          `${describeError(err)}\n${src}`,
        );
      }
      image.src = await blobToDataUrl(blob);
      try {
        await image.decode();
      } catch (err) {
        throw new IpcError(
          "export_image",
          "图片无法解码",
          `${describeError(err)}\n${src}`,
        );
      }
    }),
  );
}

export interface MediaLabels {
  audio: string;
  video: string;
  embed: string;
}

const MEDIA_TIMEOUT_MS = 4000;
const HIDDEN_STYLE =
  "position:fixed;left:-10000px;top:0;width:1px;height:1px;opacity:0;pointer-events:none";
const PLAY_ICON =
  '<svg viewBox="0 0 24 24" aria-hidden="true" focusable="false"><path d="M8 5v14l11-7z"/></svg>';

function fileLabel(src: string): string {
  const last = src.split(/[\\/]/).pop() ?? "";
  try {
    return decodeURIComponent(last);
  } catch {
    return last;
  }
}

function formatDuration(seconds: number): string {
  if (!Number.isFinite(seconds) || seconds <= 0) return "";
  const total = Math.round(seconds);
  const hours = Math.floor(total / 3600);
  const minutes = Math.floor((total % 3600) / 60);
  const rest = total % 60;
  const mm = hours > 0 ? String(minutes).padStart(2, "0") : String(minutes);
  return hours > 0
    ? `${hours}:${mm}:${String(rest).padStart(2, "0")}`
    : `${mm}:${String(rest).padStart(2, "0")}`;
}

function waitMedia(
  target: HTMLMediaElement,
  event: "loadedmetadata" | "loadeddata" | "seeked",
  src: string,
): Promise<void> {
  return new Promise((resolve, reject) => {
    if (event === "loadedmetadata" && target.readyState >= 1) {
      resolve();
      return;
    }
    if (event === "loadeddata" && target.readyState >= 2) {
      resolve();
      return;
    }
    const cleanup = () => {
      target.removeEventListener(event, onDone);
      target.removeEventListener("error", onError);
      window.clearTimeout(timer);
    };
    const onDone = () => {
      cleanup();
      resolve();
    };
    const onError = () => {
      cleanup();
      const media = target.error;
      reject(
        new IpcError(
          "export_media",
          "无法读取音视频信息",
          media ? `code ${media.code}: ${media.message}` : src,
        ),
      );
    };
    const timer = window.setTimeout(() => {
      cleanup();
      reject(new IpcError("export_media", "读取音视频信息超时", `${MEDIA_TIMEOUT_MS}ms\n${src}`));
    }, MEDIA_TIMEOUT_MS);
    target.addEventListener(event, onDone);
    target.addEventListener("error", onError);
  });
}

function detachMedia(media: HTMLMediaElement): void {
  media.removeAttribute("src");
  media.load();
  media.remove();
}

async function grabCover(video: HTMLVideoElement, src: string, duration: number): Promise<string> {
  if (duration > 0.4) {
    const seeked = waitMedia(video, "seeked", src);
    try {
      video.currentTime = Math.min(Math.max(duration * 0.1, 0.15), 1);
    } catch (err) {
      seeked.catch(() => {});
      throw err;
    }
    await seeked;
  } else {
    await waitMedia(video, "loadeddata", src);
  }
  await new Promise((resolve) => requestAnimationFrame(() => resolve(null)));
  const width = video.videoWidth;
  const height = video.videoHeight;
  if (!width || !height) throw new IpcError("export_media", "视频没有可用画面", src);
  const canvas = document.createElement("canvas");
  canvas.width = width;
  canvas.height = height;
  const context = canvas.getContext("2d");
  if (!context) throw new IpcError("export_media", "无法创建画布", src);
  context.drawImage(video, 0, 0, width, height);
  return canvas.toDataURL("image/jpeg", 0.85);
}

async function probeVideo(src: string): Promise<{ duration: number; cover: string | null }> {
  const video = document.createElement("video");
  video.preload = "metadata";
  video.muted = true;
  // CORS-clean 才能把画面画进 canvas；asset 协议会回与窗口 origin 匹配的 ACAO。
  video.crossOrigin = "anonymous";
  video.src = src;
  video.style.cssText = HIDDEN_STYLE;
  document.body.append(video);
  try {
    await waitMedia(video, "loadedmetadata", src);
    const duration = video.duration;
    let cover: string | null = null;
    try {
      cover = await grabCover(video, src, duration);
    } catch (err) {
      console.warn(`[export] video cover failed: ${describeError(err)}`, err);
    }
    return { duration, cover };
  } finally {
    detachMedia(video);
  }
}

async function probeAudio(src: string): Promise<number> {
  const audio = document.createElement("audio");
  audio.preload = "metadata";
  audio.src = src;
  audio.style.cssText = HIDDEN_STYLE;
  document.body.append(audio);
  try {
    await waitMedia(audio, "loadedmetadata", src);
    return audio.duration;
  } finally {
    detachMedia(audio);
  }
}

function captionNode(text: string): HTMLElement {
  const caption = document.createElement("div");
  caption.className = "export-media__caption";
  caption.textContent = text;
  return caption;
}

function textChip(text: string): HTMLElement {
  const chip = document.createElement("div");
  chip.className = "export-media export-embed";
  chip.textContent = text;
  return chip;
}

function videoBlock(duration: number, cover: string | null): HTMLElement {
  const block = document.createElement("div");
  block.className = "export-media export-video";
  const frame = document.createElement("div");
  frame.className = "export-media__frame";
  if (cover) {
    const image = document.createElement("img");
    image.className = "export-media__cover";
    image.src = cover;
    image.alt = "";
    frame.append(image);
  }
  const play = document.createElement("span");
  play.className = "export-media__play";
  play.innerHTML = PLAY_ICON;
  frame.append(play);
  const time = formatDuration(duration);
  if (time) {
    const chip = document.createElement("span");
    chip.className = "export-media__duration";
    chip.textContent = time;
    frame.append(chip);
  }
  block.append(frame);
  return block;
}

function audioBlock(label: string, duration: number): HTMLElement {
  const block = document.createElement("div");
  block.className = "export-media export-audio";
  block.append(captionNode(label));
  const row = document.createElement("div");
  row.className = "export-audio__row";
  const bar = document.createElement("div");
  bar.className = "export-audio__bar";
  const knob = document.createElement("span");
  knob.className = "export-audio__knob";
  bar.append(knob);
  const time = document.createElement("span");
  time.className = "export-audio__time";
  const total = formatDuration(duration);
  time.textContent = total ? `0:00 / ${total}` : "0:00";
  row.append(bar, time);
  block.append(row);
  return block;
}

async function buildMediaBlock(
  kind: "audio" | "video",
  src: string,
  label: string,
): Promise<HTMLElement> {
  if (src) {
    try {
      if (kind === "video") {
        const probe = await probeVideo(src);
        return videoBlock(probe.duration, probe.cover);
      }
      return audioBlock(label, await probeAudio(src));
    } catch (err) {
      console.warn(`[export] media preview unavailable: ${describeError(err)}`, err);
    }
  }
  return textChip(label);
}

// <video>/<audio> 在 foreignObject 里截不出内容，换成静态块：音频是进度条 + 总时长，
// 视频是封面帧 + 播放标 + 总时长；元数据读不到时退回一行类型说明，不让整次导出失败。
export async function replaceMediaElements(
  root: HTMLElement,
  labels: MediaLabels,
): Promise<void> {
  for (const element of [...root.querySelectorAll("audio, video, iframe")]) {
    const src = element.getAttribute("src") ?? "";
    const tag = element.tagName.toLowerCase();
    if (tag === "iframe") {
      const name = fileLabel(src);
      element.replaceWith(textChip(name ? `${labels.embed} · ${name}` : labels.embed));
      continue;
    }
    const kind = tag === "video" ? "video" : "audio";
    const label = kind === "video" ? labels.video : labels.audio;
    element.replaceWith(await buildMediaBlock(kind, src, label));
  }
  const covers = [...root.querySelectorAll<HTMLImageElement>("img.export-media__cover")];
  await Promise.all(
    covers.map((image) =>
      image.decode().catch((err) => {
        console.warn(`[export] cover decode failed: ${describeError(err)}`, err);
      }),
    ),
  );
}

// 超长卡片按块级子节点切分：克隆卡片外壳（保留 scoped 的 data-v 属性），把原节点逐个
// 搬进新卡片并实时量高，越线就换下一张。单个块自身超高时只能让它独占一张。
export function splitExportChunks(
  card: HTMLElement,
  stage: HTMLElement,
  label: (index: number, total: number) => string,
): HTMLElement[] {
  if (card.getBoundingClientRect().height <= CHUNK_LIMIT) return [card];
  const article = card.querySelector<HTMLElement>("article");
  const shell = card.cloneNode(true) as HTMLElement;
  const shellArticle = shell.querySelector<HTMLElement>("article");
  if (!article || !shellArticle) return [card];
  shellArticle.replaceChildren();

  const chunks: HTMLElement[] = [];
  const makeChunk = (): { card: HTMLElement; article: HTMLElement } | null => {
    const clone = shell.cloneNode(true) as HTMLElement;
    const body = clone.querySelector<HTMLElement>("article");
    if (!body) return null;
    stage.append(clone);
    chunks.push(clone);
    return { card: clone, article: body };
  };

  const nodes = [...article.childNodes];
  const first = makeChunk();
  if (!first) return [card];
  let current = first;
  article.replaceChildren();

  for (const node of nodes) {
    if (node.nodeType === Node.TEXT_NODE && !(node.textContent ?? "").trim()) continue;
    current.article.append(node);
    if (current.card.getBoundingClientRect().height <= CHUNK_LIMIT) continue;
    if (current.article.childNodes.length === 1) continue;
    current.article.removeChild(node);
    const next = makeChunk();
    if (!next) break;
    current = next;
    current.article.append(node);
  }

  card.remove();
  if (chunks.length > 1) {
    chunks.forEach((chunk, index) => {
      const date = chunk.querySelector(".export-date");
      if (date) date.textContent = label(index + 1, chunks.length);
    });
  }
  return chunks;
}

export async function capturePng(card: HTMLElement, background: string): Promise<string> {
  await document.fonts.ready;
  const height = Math.ceil(card.getBoundingClientRect().height);
  if (height <= 0) throw new IpcError("export_empty", "没有可导出的内容");
  if (height > MAX_EDGE) {
    throw new IpcError(
      "export_too_long",
      "这段内容太高，无法导出",
      `单块高度 ${height}px，上限 ${MAX_EDGE}px`,
    );
  }
  const pixelRatio = Math.min(2, Math.max(1, (MAX_EDGE - 64) / height));
  return toPng(card, { backgroundColor: background, pixelRatio });
}