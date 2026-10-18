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
