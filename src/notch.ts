import type { UiItem, UiTree } from "@flowkey-cli/native-sdk";

export const NOTCH_WIDGETS_EVENT = "notch-widgets";
export const NOTCH_READY_EVENT = "notch-ready";
export const NOTCH_ACTION_EVENT = "notch-action";

export interface NotchWidget {
  key: string;
  title: string;
  icon?: string;
  tree: UiTree | null;
  error?: string;
}

export interface NotchMedia {
  playing: boolean;
  title?: string | null;
  artist?: string | null;
  album?: string | null;
  source?: string;
  artwork?: string | null;
  positionMs?: number | null;
  durationMs?: number | null;
  updatedAtMs?: number;
}

export type NotchMode = "collapsed" | "expanded" | "volume";

export interface NotchSize {
  width: number;
  height: number;
  radius: number;
}

export function notchSize(mode: NotchMode, playing: boolean): NotchSize {
  if (mode === "expanded") return { width: 680, height: 204, radius: 22 };
  if (mode === "volume") return { width: 360, height: 60, radius: 18 };
  return playing ? { width: 300, height: 30, radius: 12 } : { width: 200, height: 30, radius: 12 };
}

export interface StripDay {
  key: string;
  weekday: string;
  day: string;
  today: boolean;
}

export function weekStrip(today: Date, locale?: string): StripDay[] {
  const weekday = new Intl.DateTimeFormat(locale, { weekday: "narrow" });
  return [-3, -2, -1, 0, 1, 2, 3].map((offset) => {
    const date = new Date(today.getFullYear(), today.getMonth(), today.getDate() + offset);
    return { key: date.toDateString(), weekday: weekday.format(date), day: String(date.getDate()).padStart(2, "0"), today: offset === 0 };
  });
}

export function litSegments(percent: number, total: number): number {
  return Math.round((Math.min(100, Math.max(0, percent)) / 100) * total);
}

export function mediaPosition(media: NotchMedia, now: number): number | null {
  if (media.positionMs == null || !media.durationMs) return null;
  const drift = media.playing && media.updatedAtMs ? Math.max(0, now - media.updatedAtMs) : 0;
  return Math.min(media.durationMs, media.positionMs + drift);
}

export function formatClock(ms: number): string {
  const seconds = Math.floor(ms / 1000);
  return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, "0")}`;
}

export function notchItems(tree: UiTree): UiItem[] {
  if (tree.type === "list") return tree.sections.flatMap((s) => s.items);
  if (tree.type === "grid") return tree.sections?.flatMap((s) => s.items) ?? tree.items;
  return [];
}

export function primaryAction(item: UiItem) {
  return item.actions?.find((a) => a.primary) ?? item.actions?.[0];
}
