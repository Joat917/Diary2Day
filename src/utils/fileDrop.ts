import { getCurrentWebview } from "@tauri-apps/api/webview";
import { isTauri } from "../ipc/client";

export interface FileDropHandlers {
  enter(paths: string[]): void;
  leave(): void;
  drop(paths: string[]): void;
}

const targets = new Map<Element, FileDropHandlers>();
let listening = false;
let active: FileDropHandlers | null = null;
let lastPaths: string[] = [];

function hitTest(position: { x: number; y: number }): FileDropHandlers | null {
  const scale = window.devicePixelRatio || 1;
  let element = document.elementFromPoint(position.x / scale, position.y / scale);
  while (element) {
    const handlers = targets.get(element);
    if (handlers) return handlers;
    element = element.parentElement;
  }
  return null;
}

function activate(next: FileDropHandlers | null) {
  if (active === next) return;
  active?.leave();
  active = next;
  next?.enter(lastPaths);
}

async function listen() {
  if (listening || !isTauri()) return;
  listening = true;
  try {
    await getCurrentWebview().onDragDropEvent((event) => {
      const payload = event.payload;
      if (payload.type === "leave") {
        activate(null);
        return;
      }
      if (payload.type === "enter") {
        lastPaths = payload.paths;
        activate(hitTest(payload.position));
        return;
      }
      if (payload.type === "over") {
        activate(hitTest(payload.position));
        return;
      }
      const handlers = hitTest(payload.position);
      activate(null);
      handlers?.drop(payload.paths);
    });
  } catch (err) {
    listening = false;
    console.error("[drop] native drag-drop unavailable", err);
  }
}

export function registerFileDrop(
  element: HTMLElement,
  handlers: FileDropHandlers,
): () => void {
  targets.set(element, handlers);
  void listen();
  return () => {
    targets.delete(element);
    if (active === handlers) activate(null);
  };
}