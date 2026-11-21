import { ref } from "vue";
import { en, type MessageKey } from "./messages/en";
import { zhCN } from "./messages/zh-CN";

export type Locale = "en" | "zh-CN";
export type { MessageKey };

export const locale = ref<Locale>("zh-CN");

const messages: Record<Locale, Record<MessageKey, string>> = {
  en,
  "zh-CN": zhCN,
};

export function setLocale(next: string) {
  locale.value = next === "en" ? "en" : "zh-CN";
  document.documentElement.lang = locale.value;
}

export function t(key: MessageKey, params?: Record<string, string | number>): string {
  const text = messages[locale.value][key] ?? en[key] ?? key;
  if (!params) return text;
  return text.replace(/\{(\w+)\}/g, (_, name: string) =>
    params[name] === undefined ? `{${name}}` : String(params[name]),
  );
}