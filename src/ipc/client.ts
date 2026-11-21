import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  Asset,
  AssetPaths,
  BackupReport,
  DataChanged,
  DataInfo,
  DayMark,
  ExportReport,
  ImportFile,
  ImportReport,
  Journal,
  JournalPage,
  LockState,
  OpProgress,
  PickedBackup,
  SearchPage,
  Settings,
  ShareReport,
  Tag,
} from "./types";

export class IpcError extends Error {
  code: string;
  detail?: string;

  constructor(code: string, message: string, detail?: string) {
    super(message);
    this.name = "IpcError";
    this.code = code;
    this.detail = detail;
  }
}

function normalizeError(error: unknown): IpcError {
  if (error && typeof error === "object" && "code" in error) {
    const value = error as { code: unknown; msg?: unknown; detail?: unknown };
    return new IpcError(
      String(value.code),
      String(value.msg ?? "操作失败"),
      value.detail === undefined ? undefined : String(value.detail),
    );
  }
  if (error instanceof Error) {
    return new IpcError("unknown", error.message);
  }
  return new IpcError("unknown", String(error));
}

export function describeError(error: unknown): string {
  if (error instanceof IpcError) {
    const head = error.code ? `[${error.code}] ${error.message}` : error.message;
    return error.detail ? `${head}\n${error.detail}` : head;
  }
  if (error instanceof Error) return error.message;
  return String(error);
}

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    const normalized = normalizeError(error);
    console.error(`[ipc] ${command} failed: ${describeError(normalized)}`, error);
    throw normalized;
  }
}

export interface ListQuery {
  dayKey?: string | null;
  from?: string | null;
  to?: string | null;
  tagId?: string | null;
  mood?: string | null;
  includeTrash?: boolean;
  limit?: number;
  cursor?: string | null;
}

export function journalList(query: ListQuery = {}): Promise<JournalPage> {
  return call("journal_list", {
    dayKey: query.dayKey ?? null,
    from: query.from ?? null,
    to: query.to ?? null,
    tagId: query.tagId ?? null,
    mood: query.mood ?? null,
    includeTrash: query.includeTrash ?? false,
    limit: query.limit ?? 50,
    cursor: query.cursor ?? null,
  });
}

export function journalGet(id: string): Promise<Journal> {
  return call("journal_get", { id });
}

export interface SaveInput {
  id?: string | null;
  content: string;
  format?: "markdown" | "plain";
  title?: string | null;
  mood?: string | null;
  tagIds?: string[];
  recordedAt?: string | null;
  recordedOff?: number | null;
  pinned?: boolean;
  sensitive?: boolean;
  revision?: number | null;
}

export function journalSave(input: SaveInput): Promise<Journal> {
  return call("journal_save", {
    id: input.id ?? null,
    content: input.content,
    format: input.format ?? null,
    title: input.title ?? null,
    mood: input.mood ?? null,
    tagIds: input.tagIds ?? [],
    recordedAt: input.recordedAt ?? null,
    recordedOff: input.recordedOff ?? null,
    pinned: input.pinned ?? false,
    sensitive: input.sensitive ?? false,
    revision: input.revision ?? null,
  });
}

export function journalDelete(id: string): Promise<string> {
  return call("journal_delete", { id });
}

export function journalRestore(id: string): Promise<string> {
  return call("journal_restore", { id });
}

export function journalPurge(id: string): Promise<void> {
  return call("journal_purge", { id });
}

export function journalEmptyTrash(): Promise<number> {
  return call("journal_empty_trash");
}

export function journalTrashList(limit = 50, cursor?: string | null): Promise<JournalPage> {
  return call("journal_trash_list", { limit, cursor: cursor ?? null });
}

export function journalSetPinned(id: string, value: boolean): Promise<void> {
  return call("journal_set_pinned", { id, value });
}

export function journalSetSensitive(id: string, value: boolean): Promise<void> {
  return call("journal_set_sensitive", { id, value });
}

export function dayMarks(from: string, to: string): Promise<DayMark[]> {
  return call("day_marks", { from, to });
}

export function searchQuery(
  text: string,
  options: {
    from?: string | null;
    to?: string | null;
    tagId?: string | null;
    mood?: string | null;
    limit?: number;
  } = {},
): Promise<SearchPage> {
  return call("search_query", {
    text,
    from: options.from ?? null,
    to: options.to ?? null,
    tagId: options.tagId ?? null,
    mood: options.mood ?? null,
    limit: options.limit ?? 50,
  });
}

export function mediaImport(files: ImportFile[]): Promise<Asset[]> {
  return call("media_import", { files });
}

export function assetGc(names: string[]): Promise<number> {
  return call("asset_gc", { names });
}

export function shareDrain(): Promise<ShareReport | null> {
  return call("share_drain");
}

export function dataExportDir(targetDir: string): Promise<ExportReport> {
  return call("data_export_dir", { targetDir });
}

export function dataImportMd(dir: string): Promise<ImportReport> {
  return call("data_import_md", { dir });
}

export function dataBackup(target: string): Promise<BackupReport> {
  return call("data_backup", { target });
}

export function dataRestore(path: string): Promise<ImportReport> {
  return call("data_restore", { path });
}

export function dataExportAndroid(): Promise<ExportReport | null> {
  return call("data_export_android");
}

export function dataImportAndroid(): Promise<ImportReport | null> {
  return call("data_import_android");
}

export function dataBackupAndroid(): Promise<BackupReport | null> {
  return call("data_backup_android");
}

export function dataPickRestoreAndroid(): Promise<PickedBackup | null> {
  return call("data_pick_restore_android");
}

export function dataRestoreAndroid(path: string): Promise<ImportReport> {
  return call("data_restore_android", { path });
}

export function appLockStatus(): Promise<LockState> {
  return call("app_lock_status");
}

export function appLockSet(code: string): Promise<LockState> {
  return call("app_lock_set", { code });
}

export function appLockClear(code: string): Promise<LockState> {
  return call("app_lock_clear", { code });
}

export function appLockUnlock(code: string): Promise<LockState> {
  return call("app_lock_unlock", { code });
}

export function appLockEngage(): Promise<LockState> {
  return call("app_lock_engage");
}

export function tagList(): Promise<Tag[]> {
  return call("tag_list");
}

export function tagCreate(name: string): Promise<Tag> {
  return call("tag_create", { name });
}

export function tagRename(id: string, name: string): Promise<void> {
  return call("tag_rename", { id, name });
}

export function tagDelete(id: string): Promise<void> {
  return call("tag_delete", { id });
}

export function tagMerge(source: string, target: string): Promise<void> {
  return call("tag_merge", { source, target });
}

export function settingsGetAll(): Promise<Settings> {
  return call("settings_get_all");
}

export function settingsPatch(patch: Partial<Settings>): Promise<Settings> {
  return call("settings_patch", { patch });
}

export function dataInfo(): Promise<DataInfo> {
  return call("data_info");
}

export function assetPaths(): Promise<AssetPaths> {
  return call("asset_paths");
}

export function openDataDir(): Promise<void> {
  return call("app_open_data_dir");
}

export function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export async function onDataChanged(
  handler: (payload: DataChanged) => void,
): Promise<UnlistenFn> {
  if (!isTauri()) return () => {};
  try {
    return await listen<DataChanged>("dd://data-changed", (event) => handler(event.payload));
  } catch {
    return () => {};
  }
}

export async function onOpProgress(
  handler: (payload: OpProgress) => void,
): Promise<UnlistenFn> {
  if (!isTauri()) return () => {};
  try {
    return await listen<OpProgress>("dd://op-progress", (event) => handler(event.payload));
  } catch {
    return () => {};
  }
}

export async function onLockState(
  handler: (payload: LockState) => void,
): Promise<UnlistenFn> {
  if (!isTauri()) return () => {};
  try {
    return await listen<LockState>("dd://lock-state", (event) => handler(event.payload));
  } catch {
    return () => {};
  }
}