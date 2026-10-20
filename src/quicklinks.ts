export interface Quicklink {
  id: string;
  name: string;
  link: string;
  appPath?: string;
  appName?: string;
}

export interface LinkArgument {
  name: string;
  defaultValue?: string;
}

const PLACEHOLDER = /\{([a-z-]+)((?:\s+[a-z]+="[^"]*")*)\s*\}/gi;

function attributes(text: string): Record<string, string> {
  return Object.fromEntries([...text.matchAll(/([a-z]+)="([^"]*)"/gi)].map((m) => [m[1].toLowerCase(), m[2]]));
}

export function linkArguments(link: string): LinkArgument[] {
  const found: LinkArgument[] = [];
  for (const match of link.matchAll(PLACEHOLDER)) {
    if (match[1].toLowerCase() !== "argument") continue;
    const attrs = attributes(match[2]);
    const name = attrs.name ?? "Query";
    if (!found.some((a) => a.name === name)) found.push({ name, defaultValue: attrs.default });
  }
  return found;
}

export interface FillContext {
  clipboard?: string;
  now?: Date;
  uuid?: () => string;
}

export function fillLink(link: string, values: Record<string, string>, context: FillContext = {}): string {
  const isUrl = /^[a-z][a-z0-9+.-]*:\/\//i.test(link.trim());
  const encode = (value: string) => (isUrl ? encodeURIComponent(value) : value);
  const now = context.now ?? new Date();
  return link.replace(PLACEHOLDER, (whole, kind: string, attrText: string) => {
    const attrs = attributes(attrText);
    switch (kind.toLowerCase()) {
      case "argument": {
        const name = attrs.name ?? "Query";
        return encode(values[name] || attrs.default || "");
      }
      case "clipboard":
        return encode(context.clipboard ?? "");
      case "date":
        return encode(now.toLocaleDateString("en-US", { dateStyle: "medium" }));
      case "time":
        return encode(now.toLocaleTimeString("en-US", { timeStyle: "short" }));
      case "datetime":
        return encode(now.toLocaleString("en-US", { dateStyle: "medium", timeStyle: "short" }));
      case "day":
        return encode(now.toLocaleDateString("en-US", { weekday: "long" }));
      case "uuid":
        return (context.uuid ?? (() => crypto.randomUUID()))();
      default:
        return whole;
    }
  });
}

export function displayLink(link: string, values: Record<string, string> = {}): string {
  return link.replace(PLACEHOLDER, (whole, kind: string, attrText: string) => {
    if (kind.toLowerCase() !== "argument") return whole;
    return values[attributes(attrText).name ?? "Query"] || "_";
  });
}

export function faviconUrl(link: string): string | null {
  try {
    const url = new URL(displayLink(link).trim());
    return /^https?:$/.test(url.protocol) ? `https://www.google.com/s2/favicons?domain=${encodeURIComponent(url.hostname)}&sz=64` : null;
  } catch {
    return null;
  }
}

export interface QuicklinkImport {
  name: string;
  link: string;
  openWith?: string;
}

export function parseQuicklinkImport(json: string): QuicklinkImport[] {
  const data = JSON.parse(json);
  const list = Array.isArray(data) ? data : Array.isArray(data?.quicklinks) ? data.quicklinks : [data];
  return list.map((raw: Record<string, unknown>) => {
    const text = (key: string) => (typeof raw?.[key] === "string" ? (raw[key] as string) : undefined);
    const name = text("name");
    if (name === undefined) throw new Error("Invalid quicklink import format");
    const openWith = text("openWith") ?? text("applicationID");
    return { name, link: text("link") ?? text("url") ?? "", ...(openWith ? { openWith } : {}) };
  });
}

export function mergeQuicklinks(existing: Quicklink[], incoming: QuicklinkImport[], apps: { name: string; path: string }[], newId: () => string) {
  const list = [...existing];
  let added = 0;
  let skipped = 0;
  for (const item of incoming) {
    if (list.some((q) => q.link === item.link)) {
      skipped += 1;
      continue;
    }
    const app = item.openWith ? apps.find((a) => a.name.toLowerCase() === item.openWith!.toLowerCase() || a.path === item.openWith) : undefined;
    list.push({ id: newId(), name: item.name, link: item.link.trim(), ...(app ? { appPath: app.path, appName: app.name } : {}) });
    added += 1;
  }
  return { list, added, skipped };
}

export function exportQuicklinks(list: Quicklink[]): string {
  return JSON.stringify(
    list.map((q) => (q.appName ? { name: q.name, link: q.link, openWith: q.appName } : { name: q.name, link: q.link })),
    null,
    2,
  );
}
