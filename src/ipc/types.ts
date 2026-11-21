export type Theme = "system" | "light" | "dark";
export type DateFormat = "slash" | "short" | "zh";
export type LocaleCode = "zh-CN" | "en";

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
  recorded_at: string;
  recorded_off: number;
  day_key: string;
  words: number;
  asset_n: number;
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

export interface DataInfo {
  db_path: string | null;
  data_dir: string;
  assets_dir: string;
  db_size: number;
  assets_size: number;
  counts: Counts;
}

export interface DataChanged {
  kind: "journal" | "tag" | "all";
  id?: string;
  day_key?: string;
}