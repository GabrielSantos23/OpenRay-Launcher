import { parseColor } from "./calculator";

export interface ClipboardEntry {
  id: string;
  kind: "text" | "files" | "image";
  text?: string;
  paths?: string[];
  width?: number;
  height?: number;
  bytes?: number;
  sourceName: string;
  sourcePath: string;
  copyCount: number;
  createdAt: number;
  updatedAt: number;
  pinned: number | null;
}

export type EntryType = "Text" | "Image" | "File" | "Link" | "Email" | "Color";
export type TypeFilter = "all" | "text" | "image" | "file" | "link" | "email" | "color";

const EMAIL = /^(mailto:)?[^\s@]+@[^\s@]+\.[^\s@]+$/i;
const LINK = /^(https?|ftp):\/\/\S+$/i;

export function entryType(entry: ClipboardEntry): EntryType {
  if (entry.kind === "image") return "Image";
  if (entry.kind === "files") return "File";
  const text = entry.text?.trim() ?? "";
  if (EMAIL.test(text)) return "Email";
  if (LINK.test(text)) return "Link";
  if (parseColor(text)) return "Color";
  return "Text";
}

export function matchesFilter(entry: ClipboardEntry, filter: TypeFilter): boolean {
  const type = entryType(entry);
  switch (filter) {
    case "all":
      return true;
    case "text":
      return entry.kind === "text";
    case "image":
      return type === "Image";
    case "file":
      return type === "File";
    case "link":
      return type === "Link" || type === "Email";
    case "email":
      return type === "Email";
    case "color":
      return type === "Color";
  }
}

function basename(path: string): string {
  return path.split(/[\\/]/).filter(Boolean).pop() ?? path;
}

export function entryTitle(entry: ClipboardEntry): string {
  if (entry.kind === "image") return `Image (${entry.width}×${entry.height})`;
  if (entry.kind === "files") {
    const paths = entry.paths ?? [];
    return paths.length === 1 ? basename(paths[0]) : `${paths.length} Files`;
  }
  const line = (entry.text ?? "").split(/\r?\n/).find((l) => l.trim()) ?? "";
  return line.trim().slice(0, 200);
}

export function wordCount(text: string): number {
  return text.trim() ? text.trim().split(/\s+/).length : 0;
}

function startOfDay(ms: number): number {
  const date = new Date(ms);
  date.setHours(0, 0, 0, 0);
  return date.getTime();
}

export function dayTitle(ms: number, now: number): string {
  const days = Math.round((startOfDay(now) - startOfDay(ms)) / 86_400_000);
  if (days <= 0) return "Today";
  if (days === 1) return "Yesterday";
  return new Date(ms).toLocaleDateString("en-US", { month: "long", day: "numeric" });
}

export function groupEntries(entries: ClipboardEntry[], now: number): { title: string; entries: ClipboardEntry[] }[] {
  const groups: { title: string; entries: ClipboardEntry[] }[] = [];
  for (const entry of entries) {
    const title = entry.pinned !== null ? "Pinned" : dayTitle(entry.updatedAt, now);
    const last = groups[groups.length - 1];
    if (last?.title === title) last.entries.push(entry);
    else groups.push({ title, entries: [entry] });
  }
  return groups;
}

export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 ** 2) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / 1024 ** 2).toFixed(1)} MB`;
}
