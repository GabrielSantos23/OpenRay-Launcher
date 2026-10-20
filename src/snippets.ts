export interface Snippet {
  id: string;
  name: string;
  text: string;
  keyword?: string;
  pinned?: boolean;
  createdAt: number;
  modifiedAt: number;
  copiedAt?: number;
  copyCount?: number;
}

export interface ResolveContext {
  clipboard: (offset: number) => Promise<string>;
  snippets: Snippet[];
  now?: Date;
  locale?: string;
  uuid?: () => string;
}

export interface Resolved {
  text: string;
  cursorBack: number;
}

const PLACEHOLDER = /\{([a-z-]+)((?:\s+[a-z]+="[^"]*")*)\s*\}/gi;
const CURSOR = "\u0000";

const DATE_FORMATS: Record<string, Intl.DateTimeFormatOptions> = {
  date: { year: "numeric", month: "short", day: "numeric" },
  datetime: { year: "numeric", month: "short", day: "numeric", hour: "numeric", minute: "2-digit" },
  time: { hour: "numeric", minute: "2-digit" },
  day: { weekday: "long" },
};

function attributes(text: string): Record<string, string> {
  return Object.fromEntries([...text.matchAll(/([a-z]+)="([^"]*)"/gi)].map((m) => [m[1].toLowerCase(), m[2]]));
}

async function fill(text: string, context: ResolveContext, nested: boolean): Promise<string> {
  const now = context.now ?? new Date();
  const parts: (string | Promise<string>)[] = [];
  let last = 0;
  for (const match of text.matchAll(PLACEHOLDER)) {
    parts.push(text.slice(last, match.index));
    last = match.index + match[0].length;
    const kind = match[1].toLowerCase();
    const attrs = attributes(match[2]);
    if (kind in DATE_FORMATS) parts.push(new Intl.DateTimeFormat(context.locale, DATE_FORMATS[kind]).format(now));
    else if (kind === "clipboard") parts.push(context.clipboard(Math.max(0, parseInt(attrs.offset ?? "0", 10) || 0)));
    else if (kind === "uuid") parts.push((context.uuid ?? (() => crypto.randomUUID()))());
    else if (kind === "cursor") parts.push(CURSOR);
    else if (kind === "snippet" && !nested) {
      const target = context.snippets.find((s) => s.name.toLowerCase() === (attrs.name ?? "").toLowerCase());
      parts.push(target ? fill(target.text, context, true).then((t) => t.replaceAll(CURSOR, "")) : match[0]);
    } else parts.push(match[0]);
  }
  parts.push(text.slice(last));
  return (await Promise.all(parts)).join("");
}

export async function resolveSnippet(text: string, context: ResolveContext): Promise<Resolved> {
  const filled = (await fill(text, context, false)).replace(/\r\n/g, "\n");
  const at = filled.indexOf(CURSOR);
  const clean = filled.replaceAll(CURSOR, "");
  return { text: clean, cursorBack: at < 0 ? 0 : [...clean.slice(at)].length };
}

export function keywordInUse(keyword: string, snippets: Snippet[], exceptId?: string): boolean {
  const trimmed = keyword.trim();
  return !!trimmed && snippets.some((s) => s.id !== exceptId && s.keyword?.trim() === trimmed);
}

export function parseSnippetImport(json: string): { name: string; text: string; keyword?: string }[] {
  const data = JSON.parse(json);
  const list = Array.isArray(data) ? data : Array.isArray(data?.snippets) ? data.snippets : [data];
  return list.map((raw: Record<string, unknown>) => {
    const name = typeof raw.name === "string" ? raw.name : typeof raw.title === "string" ? raw.title : "";
    const text = typeof raw.text === "string" ? raw.text : typeof raw.snippet === "string" ? raw.snippet : "";
    if (!name.trim() || !text) throw new Error("Invalid snippet import format");
    const keyword = typeof raw.keyword === "string" && raw.keyword.trim() ? raw.keyword.trim() : undefined;
    return keyword ? { name: name.trim(), text, keyword } : { name: name.trim(), text };
  });
}

export function mergeImport(existing: Snippet[], incoming: ReturnType<typeof parseSnippetImport>, now: number, newId: () => string) {
  const result = [...existing];
  let added = 0;
  let skipped = 0;
  for (const item of incoming) {
    const duplicate = result.some((s) => s.name === item.name && s.text === item.text);
    if (duplicate || (item.keyword && keywordInUse(item.keyword, result))) {
      skipped += 1;
      continue;
    }
    result.push({ id: newId(), ...item, createdAt: now, modifiedAt: now });
    added += 1;
  }
  return { snippets: result, added, skipped };
}

export function exportSnippets(snippets: Snippet[]): string {
  return JSON.stringify(
    snippets.map((s) => (s.keyword ? { name: s.name, text: s.text, keyword: s.keyword } : { name: s.name, text: s.text })),
    null,
    2,
  );
}
