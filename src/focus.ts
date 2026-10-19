import { invoke } from "@tauri-apps/api/core";
import {
  BookOpen,
  Briefcase,
  Code,
  Gamepad2,
  MessageCircle,
  Music,
  Newspaper,
  Plane,
  Play,
  ShoppingCart,
  Target,
  Users,
  type LucideIcon,
} from "lucide-react";
import type { IconSpec } from "./extension-api";

export type FocusMode = "block" | "allow";

export interface FocusCategory {
  id: string;
  title: string;
  icon: string;
  apps: string[];
  websites: string[];
}

export interface FocusPreferences {
  snoozeMs: number | null;
  playSound: boolean;
  restoreApps: boolean;
  allowAllWebsites: boolean;
}

export interface FocusForm {
  goal: string;
  durationMs: number | null;
  mode: FocusMode;
  items: string[];
}

export interface FocusSession {
  id: number;
  title: string;
  mode: FocusMode;
  status: "running" | "paused";
  startedAt: number;
  durationMs: number | null;
  endAt: number | null;
  remainingMs: number;
  pauseEndsAt: number | null;
  form: FocusForm | null;
}

interface FocusData {
  categories?: FocusCategory[];
  preferences?: Partial<FocusPreferences>;
  lastForm?: FocusForm;
}

export const FOCUS_ICON: IconSpec = { glyph: Target, tone: "green" };
export const DEFAULT_TITLE = "Focus session";
export const DEFAULT_PREFERENCES: FocusPreferences = { snoozeMs: 180_000, playSound: true, restoreApps: false, allowAllWebsites: false };
export const EMPTY_FORM: FocusForm = { goal: "", durationMs: null, mode: "block", items: [] };

export const CATEGORY_ICONS: Record<string, LucideIcon> = {
  target: Target,
  users: Users,
  message: MessageCircle,
  game: Gamepad2,
  play: Play,
  news: Newspaper,
  cart: ShoppingCart,
  plane: Plane,
  code: Code,
  book: BookOpen,
  music: Music,
  work: Briefcase,
};

export const SNOOZE_OPTIONS = [
  { value: "60000", label: "1 minute" },
  { value: "180000", label: "3 minutes" },
  { value: "300000", label: "5 minutes" },
  { value: "600000", label: "10 minutes" },
  { value: "0", label: "No Snooze" },
];

export const DURATION_OPTIONS: { value: string; label: string }[] = [
  { value: "", label: "No Limit" },
  ...[5, 10, 15, 25, 30, 45].map((m) => ({ value: String(m * 60_000), label: `${m} minutes` })),
  { value: String(60 * 60_000), label: "1 hour" },
  { value: String(90 * 60_000), label: "1.5 hours" },
  ...[2, 3, 4].map((h) => ({ value: String(h * 3_600_000), label: `${h} hours` })),
];

export const MODE_OPTIONS: { value: FocusMode; label: string }[] = [
  { value: "block", label: "Block Apps & Websites" },
  { value: "allow", label: "Allow Apps & Websites" },
];

export const BUILTIN_CATEGORIES: FocusCategory[] = [
  {
    id: "social",
    title: "Social",
    icon: "users",
    apps: [],
    websites: ["facebook.com", "instagram.com", "x.com", "twitter.com", "reddit.com", "tiktok.com", "linkedin.com", "pinterest.com", "threads.net", "bsky.app", "tumblr.com", "snapchat.com"],
  },
  {
    id: "messaging",
    title: "Messaging",
    icon: "message",
    apps: ["discord.exe", "slack.exe", "telegram.exe", "signal.exe", "ms-teams.exe", "whatsapp.exe"],
    websites: ["web.whatsapp.com", "discord.com", "web.telegram.org", "messenger.com", "slack.com"],
  },
  {
    id: "gaming",
    title: "Gaming",
    icon: "game",
    apps: ["steam.exe", "epicgameslauncher.exe", "battle.net.exe", "riotclientservices.exe", "eadesktop.exe", "galaxyclient.exe"],
    websites: ["store.steampowered.com", "steamcommunity.com", "epicgames.com", "itch.io", "gog.com"],
  },
  {
    id: "streaming",
    title: "Streaming",
    icon: "play",
    apps: [],
    websites: ["youtube.com", "netflix.com", "primevideo.com", "disneyplus.com", "max.com", "hulu.com", "twitch.tv", "crunchyroll.com", "globoplay.globo.com"],
  },
  {
    id: "news",
    title: "News",
    icon: "news",
    apps: [],
    websites: ["news.ycombinator.com", "bbc.com", "cnn.com", "nytimes.com", "theguardian.com", "reuters.com", "g1.globo.com", "uol.com.br"],
  },
  {
    id: "shopping",
    title: "Shopping",
    icon: "cart",
    apps: [],
    websites: ["amazon.*", "ebay.com", "aliexpress.com", "mercadolivre.com.br", "shopee.com.br", "etsy.com", "temu.com", "shein.com"],
  },
];

async function loadData(): Promise<FocusData> {
  const value = await invoke<FocusData | null>("data_get", { name: "focus" });
  return value && typeof value === "object" ? value : {};
}

async function updateData(change: (data: FocusData) => FocusData) {
  await invoke("data_set", { name: "focus", value: change(await loadData()) });
}

export async function loadCategories(): Promise<FocusCategory[]> {
  return (await loadData()).categories ?? BUILTIN_CATEGORIES;
}

export function saveCategories(categories: FocusCategory[]) {
  return updateData((data) => ({ ...data, categories }));
}

export async function loadPreferences(): Promise<FocusPreferences> {
  return { ...DEFAULT_PREFERENCES, ...(await loadData()).preferences };
}

export function savePreferences(preferences: FocusPreferences) {
  return updateData((data) => ({ ...data, preferences }));
}

export async function loadLastForm(): Promise<FocusForm> {
  return (await loadData()).lastForm ?? EMPTY_FORM;
}

export function saveLastForm(form: FocusForm) {
  return updateData((data) => ({ ...data, lastForm: form }));
}

export function normalizeHost(text: string): string | null {
  const value = text.trim().toLowerCase();
  if (!value || /\s/.test(value)) return null;
  const rest = value.includes("://") ? value.split("://")[1] : value;
  const host = rest.split(/[/?#]/)[0].split("@").pop()!.split(":")[0].replace(/^www\./, "");
  if (!/^[a-z0-9*][a-z0-9.*-]*\.[a-z0-9*-]+$/.test(host)) return null;
  return host;
}

const unique = (values: string[]) => Array.from(new Set(values.filter(Boolean)));

export function startInput(form: FocusForm, categories: FocusCategory[], preferences: FocusPreferences) {
  const selected = categories.filter((c) => form.items.includes(`category:${c.id}`));
  const of = (prefix: string) => form.items.filter((i) => i.startsWith(prefix)).map((i) => i.slice(prefix.length));
  return {
    title: form.goal.trim() || DEFAULT_TITLE,
    mode: form.mode,
    durationMs: form.durationMs,
    apps: unique([...selected.flatMap((c) => c.apps), ...of("app:")]),
    websites: unique([...selected.flatMap((c) => c.websites), ...of("site:")]),
    allowAllWebsites: preferences.allowAllWebsites,
    snoozeMs: preferences.snoozeMs || null,
    playSound: preferences.playSound,
    restoreApps: preferences.restoreApps,
    form,
  };
}

export function remainingMs(session: FocusSession, now: number): number {
  if (session.durationMs === null) return 0;
  if (session.status === "paused" || session.endAt === null) return Math.max(0, session.remainingMs);
  return Math.max(0, session.endAt - now);
}

export function pauseRemainingMs(session: FocusSession, now: number): number | null {
  if (session.status !== "paused" || session.pauseEndsAt === null) return null;
  return Math.max(0, session.pauseEndsAt - now);
}

export function elapsedFraction(session: FocusSession, now: number): number {
  if (!session.durationMs) return 0;
  return Math.min(1, Math.max(0, (session.durationMs - remainingMs(session, now)) / session.durationMs));
}

export function formatTimer(ms: number): string {
  const seconds = Math.max(0, Math.ceil(ms / 1000));
  const pad = (n: number) => String(n).padStart(2, "0");
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  return hours > 0 ? `${pad(hours)}:${pad(minutes)}` : `${pad(minutes)}:${pad(seconds % 60)}`;
}

export function timerStep(ms: number): number {
  return ms <= 600_000 ? 1000 : ms >= 3_600_000 ? 60_000 : 5000;
}

export function slugify(title: string, taken: string[]): string {
  const base = title.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "") || "category";
  let id = base;
  for (let n = 2; taken.includes(id); n++) id = `${base}-${n}`;
  return id;
}

export function parseCategoryImport(text: string): Omit<FocusCategory, "id">[] {
  const data: unknown = JSON.parse(text);
  const list = Array.isArray(data)
    ? data
    : data && typeof data === "object" && Array.isArray((data as { focusCategories?: unknown }).focusCategories)
      ? (data as { focusCategories: unknown[] }).focusCategories
      : [data];
  return list.map((entry) => {
    if (!entry || typeof entry !== "object") throw new Error("Invalid focus category import format");
    const e = entry as Record<string, unknown>;
    const strings = (v: unknown) => (Array.isArray(v) ? v.filter((x): x is string => typeof x === "string") : []);
    if (typeof e.title !== "string" || !e.title.trim()) throw new Error("Invalid focus category import format");
    const icon = typeof e.icon === "string" ? e.icon : typeof e.iconName === "string" ? e.iconName : "target";
    return {
      title: e.title.trim(),
      icon: icon in CATEGORY_ICONS ? icon : "target",
      apps: strings(e.applicationIds ?? e.apps),
      websites: strings(e.websiteHosts ?? e.websites)
        .map(normalizeHost)
        .filter((h): h is string => h !== null),
    };
  });
}
