import { invoke } from "@tauri-apps/api/core";

export interface Note {
  id: string;
  markdown: string;
  text: string;
  createdAt: number;
  updatedAt: number;
  lastAccessedAt: number;
  pinned: boolean;
}

export interface NotesRequest {
  create?: boolean;
  id?: string;
}

export function newNote(now: number, markdown = "", text = ""): Note {
  return { id: crypto.randomUUID(), markdown, text, createdAt: now, updatedAt: now, lastAccessedAt: now, pinned: false };
}

export async function loadNotes(): Promise<Note[]> {
  const value = await invoke<unknown>("data_get", { name: "notes" });
  return Array.isArray(value) ? (value as Note[]) : [];
}

export function saveNotes(notes: Note[]): Promise<void> {
  return invoke("data_set", { name: "notes", value: notes });
}

export function noteTitle(text: string): string {
  return text.split("\n").find((line) => line.trim())?.trim() ?? "Untitled";
}

export function wordCount(text: string): number {
  return text.split(/\s+/).filter(Boolean).length;
}

export function countLabel(text: string, unit: "characters" | "words"): string {
  const n = unit === "words" ? wordCount(text) : text.trim().length;
  return `${n.toLocaleString("en-US")} ${n === 1 ? unit.slice(0, -1) : unit}`;
}

export function currentNote(notes: Note[]): Note | undefined {
  return notes.reduce<Note | undefined>((best, note) => (!best || note.lastAccessedAt > best.lastAccessedAt ? note : best), undefined);
}

export function browseSections(notes: Note[]): { title: string; notes: Note[] }[] {
  const recent = [...notes].sort((a, b) => b.lastAccessedAt - a.lastAccessedAt);
  return [
    { title: "Pinned", notes: recent.filter((n) => n.pinned) },
    { title: "Notes", notes: recent.filter((n) => !n.pinned) },
  ].filter((section) => section.notes.length > 0);
}

export function emptyNote(notes: Note[]): Note | undefined {
  return notes.find((note) => !note.text.trim());
}
