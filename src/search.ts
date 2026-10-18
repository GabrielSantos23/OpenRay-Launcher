import type { RegisteredCommand } from "./extension-api";

export type Frecency = Record<string, number>;

const SUGGESTION_LIMIT = 8;

function words(text: string): string[] {
  return text
    .replace(/([a-z])([A-Z])/g, "$1 $2")
    .toLowerCase()
    .split(/[\s\-_.()]+/)
    .filter(Boolean);
}

function isSubsequence(query: string, text: string): boolean {
  let i = 0;
  for (const ch of text) if (ch === query[i]) i++;
  return i === query.length;
}

function scoreText(query: string, text: string): number {
  const lower = text.toLowerCase();
  if (lower.startsWith(query)) return 100;
  const parts = words(text);
  if (parts.some((w) => w.startsWith(query))) return 80;
  if (parts.map((w) => w[0]).join("").startsWith(query)) return 70;
  if (lower.includes(query)) return 50;
  if (isSubsequence(query, lower)) return 20;
  return 0;
}

export function matchScore(query: string, title: string, keywords: string[] = []): number {
  const q = query.trim().toLowerCase();
  if (!q) return 1;
  const candidates = [title, ...keywords];
  const whole = Math.max(...candidates.map((c) => scoreText(q, c)));
  if (whole > 0) return whole;
  const tokens = q.split(/\s+/);
  if (tokens.length < 2) return 0;
  const haystack = candidates.flatMap(words);
  return tokens.every((t) => haystack.some((w) => w.startsWith(t))) ? 60 : 0;
}

function byFrecencyThenTitle(frecency: Frecency) {
  return (a: RegisteredCommand, b: RegisteredCommand) =>
    (frecency[b.key] ?? 0) - (frecency[a.key] ?? 0) || a.title.localeCompare(b.title);
}

export interface Section {
  title: string;
  items: RegisteredCommand[];
}

function aliasScore(query: string, alias = ""): number {
  const q = query.trim().toLowerCase();
  return alias && alias.toLowerCase().startsWith(q) ? 110 : 0;
}

export function search(
  commands: RegisteredCommand[],
  query: string,
  frecency: Frecency,
  favorites: string[] = [],
  aliases: Record<string, string> = {},
): Section[] {
  if (!query.trim()) {
    const byKey = new Map(commands.map((c) => [c.key, c]));
    const used = commands
      .filter((c) => !favorites.includes(c.key) && (frecency[c.key] ?? 0) > 0)
      .sort(byFrecencyThenTitle(frecency))
      .slice(0, SUGGESTION_LIMIT);
    const pinned = favorites.flatMap((key) => byKey.get(key) ?? []);
    const shown = new Set([...pinned, ...used].map((c) => c.key));
    const extensionCommands = commands.filter((c) => c.kind === "Command" && !shown.has(c.key)).sort((a, b) => a.title.localeCompare(b.title));
    const sections = [
      { title: "Favorites", items: pinned },
      { title: "Suggestions", items: used },
      { title: "Commands", items: extensionCommands },
    ].filter((s) => s.items.length > 0);
    return sections.length > 0 ? sections : [{ title: "Results", items: [...commands].sort(byFrecencyThenTitle(frecency)) }];
  }
  const tieBreak = byFrecencyThenTitle(frecency);
  const items = commands
    .map((command) => ({ command, score: Math.max(aliasScore(query, aliases[command.key]), matchScore(query, command.title, command.keywords)) }))
    .filter((m) => m.score > 0)
    .sort((a, b) => b.score - a.score || tieBreak(a.command, b.command))
    .map((m) => m.command);
  return items.length > 0 ? [{ title: "Results", items }] : [];
}
