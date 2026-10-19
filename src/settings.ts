import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useRef, useState } from "react";

export type ShowOn = "mouse-screen" | "active-screen" | "primary-screen";
export type EscapeKeyBehavior = "pop-back-or-close" | "close-and-pop-to-root";

export interface CommandPrefs {
  alias: string;
  hotkey: string;
  disabled: boolean;
}

export interface Settings {
  hotkey: string;
  showInTray: boolean;
  showOn: ShowOn;
  escapeKeyBehavior: EscapeKeyBehavior;
  escapeKeyClosesWindow: boolean;
  appearance: "system" | "dark" | "light";
  themeDarkId: string;
  themeLightId: string;
  commands: Record<string, CommandPrefs>;
}

export const DEFAULT_HOTKEY = "Alt+Space";

const NO_PREFS: CommandPrefs = { alias: "", hotkey: "", disabled: false };

export function commandPrefs(settings: Settings, key: string): CommandPrefs {
  return { ...NO_PREFS, ...settings.commands[key] };
}

export function isEnabled(settings: Settings, key: string, extensionId: string): boolean {
  return !commandPrefs(settings, key).disabled && !commandPrefs(settings, extensionId).disabled;
}

export function withCommandPrefs(settings: Settings, key: string, patch: Partial<CommandPrefs>): Settings {
  return { ...settings, commands: { ...settings.commands, [key]: { ...commandPrefs(settings, key), ...patch } } };
}

export const LAUNCHER_OWNER = "launcher";

function normalizeHotkey(hotkey: string): string {
  const parts = hotkey.split("+").map((part) => part.trim().toLowerCase()).filter(Boolean);
  const key = parts.pop();
  return [...parts.sort(), key].join("+");
}

export function sameHotkey(a: string, b: string): boolean {
  return !!a.trim() && !!b.trim() && normalizeHotkey(a) === normalizeHotkey(b);
}

export function hotkeyOwner(settings: Settings, hotkey: string, self: string): string | null {
  if (self !== LAUNCHER_OWNER && sameHotkey(settings.hotkey, hotkey)) return LAUNCHER_OWNER;
  return Object.entries(settings.commands).find(([key, prefs]) => key !== self && sameHotkey(prefs.hotkey, hotkey))?.[0] ?? null;
}

export function withHotkey(settings: Settings, owner: string, hotkey: string, overwrite: boolean): Settings {
  let next = owner === LAUNCHER_OWNER ? { ...settings, hotkey } : withCommandPrefs(settings, owner, { hotkey });
  if (!overwrite) return next;
  for (const [key, prefs] of Object.entries(settings.commands)) {
    if (key !== owner && sameHotkey(prefs.hotkey, hotkey)) next = withCommandPrefs(next, key, { hotkey: "" });
  }
  return next;
}

const KEY_NAMES: Record<string, string> = {
  Space: "Space",
  Enter: "Enter",
  Tab: "Tab",
  Backquote: "`",
  Period: ".",
  Comma: ",",
  Slash: "/",
  Semicolon: ";",
  Minus: "-",
  Equal: "=",
  BracketLeft: "[",
  BracketRight: "]",
  Backslash: "\\",
  Quote: "'",
  ArrowUp: "ArrowUp",
  ArrowDown: "ArrowDown",
  ArrowLeft: "ArrowLeft",
  ArrowRight: "ArrowRight",
  Home: "Home",
  End: "End",
  PageUp: "PageUp",
  PageDown: "PageDown",
  Insert: "Insert",
  Delete: "Delete",
  Backspace: "Backspace",
};

interface KeyEventLike {
  code: string;
  ctrlKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
  metaKey: boolean;
}

export function hotkeyFromEvent(event: KeyEventLike): string | null {
  const key =
    KEY_NAMES[event.code] ?? event.code.match(/^Key([A-Z])$/)?.[1] ?? event.code.match(/^Digit(\d)$/)?.[1] ?? event.code.match(/^(F\d{1,2})$/)?.[1] ?? event.code.match(/^(Numpad\d)$/)?.[1];
  const modifiers = [event.ctrlKey && "Ctrl", event.altKey && "Alt", event.shiftKey && "Shift", event.metaKey && "Win"].filter(Boolean);
  if (!key || modifiers.length === 0) return null;
  return [...modifiers, key].join("+");
}

const KEY_GLYPHS: Record<string, string> = { ArrowUp: "↑", ArrowDown: "↓", ArrowLeft: "←", ArrowRight: "→", Backspace: "⌫", PageUp: "PgUp", PageDown: "PgDn", Delete: "Del", Insert: "Ins" };

export function hotkeyLabels(hotkey: string): string[] {
  return hotkey.split("+").map((part) => KEY_GLYPHS[part.trim()] ?? part.trim());
}

export function useSettings(): [Settings | null, (next: Settings) => Promise<void>] {
  const [settings, setSettings] = useState<Settings | null>(null);
  const latest = useRef<Settings | null>(null);
  latest.current = settings;

  useEffect(() => {
    invoke<Settings>("settings_get").then(setSettings);
    const unlisten = listen<Settings>("settings-changed", (event) => setSettings(event.payload));
    return () => {
      unlisten.then((off) => off());
    };
  }, []);

  const save = useCallback(async (next: Settings) => {
    const previous = latest.current;
    setSettings(next);
    try {
      await invoke("settings_set", { settings: next });
    } catch (error) {
      setSettings(previous);
      throw error;
    }
  }, []);

  return [settings, save];
}
