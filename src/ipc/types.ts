export type Theme =
  | "light"
  | "warm"
  | "mist"
  | "blossom"
  | "dark"
  | "midnight"
  | "mocha"
  | "forest";
export type DateFormat = "slash" | "short" | "zh";
export type LocaleCode = "zh-CN" | "en";
export type CardTime = "recorded" | "updated";

export interface Settings {
  theme: Theme;
  language: LocaleCode;
  dateFormat: DateFormat;
  weekStart: number;
  indentFirstLine: boolean;
  imageLazy: boolean;
  showStatsCard: boolean;
  quickRecord: boolean;
  lockLeaveDelayS: number;
  reduceMotion: boolean;
  dayBoundaryMinutes: number;
  cardTime: CardTime;
  autoEdit: boolean;
  exportWidth: number;
}

export interface Tag {
  id: string;
  name: string;
  usage: number;
}

export interface JournalSummary {
  id: string;
  title: string;
  preview: string;
  mood: string | null;
  weather: string | null;
  recorded_at: string;
  recorded_off: number;
  day_key: string;
  updated_at: string;
  words: number;
  asset_n: number;
  audio_n: number;
  video_n: number;
  cover_asset: string | null;
  pinned: boolean;
  sensitive: boolean;
  deleted_at: string | null;
}

export interface Journal extends JournalSummary {
  content: string;
  format: "markdown" | "plain";
  created_at: string;
  updated_at: string;
  revision: number;
  tags: Tag[];
}

export interface JournalPage {
  items: JournalSummary[];
  next_cursor: string | null;
}

export interface DayMark {
  day_key: string;
  n: number;
  words: number;
}

export interface SearchPage {
  items: JournalSummary[];
  partial: boolean;
}

export interface Counts {
  journals: number;
  trash: number;
  tags: number;
  assets: number;
  words: number;
}

export interface AssetPaths {
  assets_dir: string;
}

export interface DataInfo {
  db_path: string | null;
  data_dir: string;
  assets_dir: string;
  db_size: number;
  assets_size: number;
  counts: Counts;
  mobile: boolean;
}

export interface DataChanged {
  kind: "journal" | "tag" | "all";
  id?: string;
  day_key?: string;
}

export type AssetKind = "image" | "audio" | "video";

export interface Asset {
  name: string;
  kind: AssetKind;
  ext: string;
  size: number;
  width: number | null;
  height: number | null;
}

export interface ExportView {
  name: string;
  view: string;
}


export interface ExportReport {
  dir: string;
  journals: number;
  assets: number;
}

export interface ImportReport {
  journals: number;
  assets: number;
}

export interface BackupReport {
  path: string;
  size: number;
}

export interface CleanupReport {
  removed: number;
  freed: number;
}

export interface PickedBackup {
  path: string;
}

export interface ShareReport {
  id: string;
  dayKey: string;
  assets: number;
}

export interface LockState {
  enabled: boolean;
  locked: boolean;
}

export interface OpProgress {
  opId: string;
  phase: string;
  done: number;
  total: number;
}