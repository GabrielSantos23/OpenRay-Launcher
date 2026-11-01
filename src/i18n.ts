import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useSyncExternalStore } from "react";
import { pt } from "./i18n/pt";

export type Language = "en" | "pt";
export type LanguageSetting = "system" | Language;

const DICTIONARIES: Record<Language, Record<string, string>> = { en: {}, pt };

let locale: Language = "en";
const listeners = new Set<() => void>();

export function detectLanguage(setting: LanguageSetting, system: string): Language {
  return setting === "system" ? (system.toLowerCase().startsWith("pt") ? "pt" : "en") : setting;
}

export function setLanguage(setting: LanguageSetting): void {
  const next = detectLanguage(setting, navigator.language);
  if (next === locale) return;
  locale = next;
  listeners.forEach((listener) => listener());
}

export function language(): Language {
  return locale;
}

export function t(key: string, params?: Record<string, string | number>): string {
  let text = DICTIONARIES[locale][key] ?? key;
  if (params) for (const [name, value] of Object.entries(params)) text = text.replaceAll(`{${name}}`, String(value));
  return text;
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

export function useLanguage(): Language {
  return useSyncExternalStore(subscribe, () => locale);
}

export function useApplyLanguage(): void {
  useEffect(() => {
    invoke<{ language: LanguageSetting }>("settings_get").then((settings) => setLanguage(settings.language));
    const unlisten = listen<{ language: LanguageSetting }>("settings-changed", (event) => setLanguage(event.payload.language));
    return () => {
      unlisten.then((off) => off());
    };
  }, []);
}
