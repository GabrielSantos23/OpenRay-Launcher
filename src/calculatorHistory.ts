export type CalculationKind = "math" | "unitConversion" | "dateTime";

export interface Calculation {
  id: string;
  expression: string;
  result: string;
  kind: CalculationKind;
  createdAt: number;
  updatedAt: number;
  pinned?: number;
}

export const RETENTION_MS = 92 * 86_400_000;
export const MAX_ENTRIES = 4096;

export function prune(list: Calculation[], now: number): Calculation[] {
  const kept = list.filter((c) => c.pinned !== undefined || now - c.updatedAt < RETENTION_MS);
  const unpinned = kept.filter((c) => c.pinned === undefined).sort((a, b) => b.updatedAt - a.updatedAt);
  const dropped = new Set(unpinned.slice(Math.max(0, MAX_ENTRIES - (kept.length - unpinned.length))).map((c) => c.id));
  return kept.filter((c) => !dropped.has(c.id));
}

export function upsert(list: Calculation[], expression: string, result: string, kind: CalculationKind, now: number): { list: Calculation[]; id: string } {
  const trimmed = expression.trim();
  const existing = list.find((c) => c.expression === trimmed);
  if (existing) {
    return { list: prune(list.map((c) => (c === existing ? { ...c, result, kind, updatedAt: now } : c)), now), id: existing.id };
  }
  const id = `${now.toString(36)}${Math.random().toString(36).slice(2, 8)}`;
  return { list: prune([...list, { id, expression: trimmed, result, kind, createdAt: now, updatedAt: now }], now), id };
}

export function pinned(list: Calculation[]): Calculation[] {
  return list.filter((c) => c.pinned !== undefined).sort((a, b) => a.pinned! - b.pinned!);
}

export function setPinned(list: Calculation[], id: string, pin: boolean): Calculation[] {
  const next = pinned(list).reduce((max, c) => Math.max(max, c.pinned!), 0) + 1;
  return list.map((c) => (c.id !== id ? c : pin ? { ...c, pinned: next } : { ...c, pinned: undefined }));
}

export function movePinned(list: Calculation[], id: string, step: -1 | 1): Calculation[] {
  const order = pinned(list);
  const from = order.findIndex((c) => c.id === id);
  const to = from + step;
  if (from < 0 || to < 0 || to >= order.length) return list;
  const a = order[from].pinned!;
  const b = order[to].pinned!;
  return list.map((c) => (c.id === order[from].id ? { ...c, pinned: b } : c.id === order[to].id ? { ...c, pinned: a } : c));
}

export function search(list: Calculation[], query: string): Calculation[] {
  const q = query.trim().toLowerCase();
  if (!q) return list;
  return list.filter((c) => c.expression.toLowerCase().includes(q) || c.result.toLowerCase().includes(q));
}

function startOfDay(ms: number): number {
  const d = new Date(ms);
  d.setHours(0, 0, 0, 0);
  return d.getTime();
}

export function groupHistory(list: Calculation[], now: number): { title: string; items: Calculation[] }[] {
  const today = startOfDay(now);
  const yesterday = startOfDay(today - 1);
  const weekday = (new Date(today).getDay() + 6) % 7;
  const week = startOfDay(today - weekday * 86_400_000);
  const month = new Date(new Date(today).getFullYear(), new Date(today).getMonth(), 1).getTime();
  const year = new Date(new Date(today).getFullYear(), 0, 1).getTime();
  const groups: [string, (t: number) => boolean][] = [
    ["Today", (t) => t >= today],
    ["Yesterday", (t) => t >= yesterday],
    ["This Week", (t) => t >= week],
    ["This Month", (t) => t >= month],
    ["This Year", (t) => t >= year],
    ["Older", () => true],
  ];
  const recent = list.filter((c) => c.pinned === undefined).sort((a, b) => b.updatedAt - a.updatedAt);
  const sections = [{ title: "Pinned", items: pinned(list) }, ...groups.map(([title]) => ({ title, items: [] as Calculation[] }))];
  for (const c of recent) sections[1 + groups.findIndex(([, match]) => match(c.updatedAt))].items.push(c);
  return sections.filter((s) => s.items.length > 0);
}
