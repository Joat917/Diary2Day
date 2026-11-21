import { locale, t, type MessageKey } from "../i18n";
import type { DateFormat } from "../ipc/types";

const pad = (value: number) => String(value).padStart(2, "0");

export function diaryDayKey(date: Date, boundaryMinutes = 0): string {
  const shifted = new Date(date.getTime() - boundaryMinutes * 60_000);
  return `${shifted.getFullYear()}-${pad(shifted.getMonth() + 1)}-${pad(shifted.getDate())}`;
}

export function diaryToday(boundaryMinutes = 0, now = new Date()): string {
  return diaryDayKey(now, boundaryMinutes);
}

export function parseDayKey(dayKey: string): { year: number; month: number; day: number } {
  const [year, month, day] = dayKey.split("-").map(Number);
  return { year, month, day };
}

export function addDays(dayKey: string, delta: number): string {
  const { year, month, day } = parseDayKey(dayKey);
  const date = new Date(Date.UTC(year, month - 1, day + delta));
  return `${date.getUTCFullYear()}-${pad(date.getUTCMonth() + 1)}-${pad(date.getUTCDate())}`;
}

export function weekdayOf(dayKey: string): number {
  const { year, month, day } = parseDayKey(dayKey);
  return new Date(Date.UTC(year, month - 1, day)).getUTCDay();
}

export function startOfWeek(dayKey: string, weekStart: number): string {
  const diff = (weekdayOf(dayKey) - weekStart + 7) % 7;
  return addDays(dayKey, -diff);
}

export function weekDays(dayKey: string, weekStart: number): string[] {
  const start = startOfWeek(dayKey, weekStart);
  return Array.from({ length: 7 }, (_, index) => addDays(start, index));
}

function localOffsetMinutes(now = new Date()): number {
  return -now.getTimezoneOffset();
}

export function recordedAtNow(now = new Date()): {
  recordedAt: string;
  recordedOff: number;
} {
  return { recordedAt: now.toISOString(), recordedOff: localOffsetMinutes(now) };
}

export function weekdayName(dayKey: string): string {
  return t(`weekday.${weekdayOf(dayKey)}` as MessageKey);
}

function wallDate(recordedAt: string, offsetMinutes: number): Date {
  const parsed = Date.parse(recordedAt);
  return new Date((Number.isNaN(parsed) ? Date.now() : parsed) + offsetMinutes * 60_000);
}

function parts(date: Date) {
  return {
    year: date.getUTCFullYear(),
    month: date.getUTCMonth() + 1,
    day: date.getUTCDate(),
    hour: date.getUTCHours(),
    minute: date.getUTCMinutes(),
    weekday: date.getUTCDay(),
  };
}

function weekdayLabel(weekday: number): string {
  if (locale.value === "en") {
    return new Intl.DateTimeFormat("en", { weekday: "short", timeZone: "UTC" }).format(
      new Date(Date.UTC(2024, 0, 7 + weekday)),
    );
  }
  return `周${t(`weekday.${weekday}` as MessageKey)}`;
}

function displayParts(
  recordedAt: string,
  offsetMinutes: number,
  boundaryMinutes: number,
) {
  const wall = wallDate(recordedAt, offsetMinutes);
  const value = parts(wall);
  if (value.hour * 60 + value.minute < boundaryMinutes) {
    const previous = parts(new Date(wall.getTime() - 86_400_000));
    return { ...previous, hour: value.hour + 24 };
  }
  return value;
}

export function formatRecordedAt(
  recordedAt: string,
  offsetMinutes: number,
  format: DateFormat,
  boundaryMinutes = 0,
): string {
  const value = displayParts(recordedAt, offsetMinutes, boundaryMinutes);
  if (format === "short") {
    return `${pad(value.month)}/${pad(value.day)} ${pad(value.hour)}:${pad(value.minute)}`;
  }
  if (format === "zh") {
    return `${value.year}年${value.month}月${value.day}日`;
  }
  return `${value.year}/${pad(value.month)}/${pad(value.day)} ${weekdayLabel(value.weekday)}`;
}

function formatCardDate(
  recordedAt: string,
  offsetMinutes: number,
  boundaryMinutes = 0,
): string {
  const value = displayParts(recordedAt, offsetMinutes, boundaryMinutes);
  return `${pad(value.month)}/${pad(value.day)} ${pad(value.hour)}:${pad(value.minute)}`;
}

export function formatCardTime(
  recordedAt: string,
  recordedOff: number,
  updatedAt: string,
  showUpdated: boolean,
  boundaryMinutes = 0,
): string {
  if (!showUpdated) return formatCardDate(recordedAt, recordedOff, boundaryMinutes);
  return formatCardDate(
    updatedAt,
    localOffsetMinutes(new Date(updatedAt)),
    boundaryMinutes,
  );
}

export function formatBoundary(minutes: number): string {
  return `${pad(Math.floor(minutes / 60))}:${pad(minutes % 60)}`;
}

export function parseBoundary(value: string): number {
  const [hour = 0, minute = 0] = value.split(":").map(Number);
  return Math.min(Math.max(hour * 60 + minute, 0), 479);
}

export function formatDayTitle(dayKey: string): string {
  const { year, month, day } = parseDayKey(dayKey);
  return locale.value === "en"
    ? `${year}/${pad(month)}/${pad(day)} ${weekdayLabel(weekdayOf(dayKey))}`
    : `${year}年${month}月${day}日 ${weekdayLabel(weekdayOf(dayKey))}`;
}

export function monthLabel(dayKey: string): string {
  const { year, month } = parseDayKey(dayKey);
  return locale.value === "en" ? `${year}/${pad(month)}` : `${year}年${month}月`;
}

export function greetingKey(now = new Date()): MessageKey {
  const hour = now.getHours();
  if (hour >= 6 && hour < 11) return "greeting.morning";
  if (hour >= 11 && hour < 13) return "greeting.noon";
  if (hour >= 13 && hour < 18) return "greeting.afternoon";
  if (hour >= 18 && hour < 23) return "greeting.evening";
  return "greeting.night";
}

export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}