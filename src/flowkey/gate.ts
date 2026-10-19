import type { ExtensionManifest } from "@flowkey-cli/native-sdk";

export interface Grants {
  nativeMethods: string[];
  httpHosts: string[];
  oauth: string[];
  fsPaths: string[];
  uriSchemes: string[];
}

export const NO_GRANTS: Grants = { nativeMethods: [], httpHosts: [], oauth: [], fsPaths: [], uriSchemes: [] };

const KINDS = Object.keys(NO_GRANTS) as (keyof Grants)[];

export function grantsOf(manifest: ExtensionManifest): Grants {
  return {
    nativeMethods: manifest.nativeMethods ?? [],
    httpHosts: manifest.httpHosts ?? [],
    oauth: manifest.oauth ?? [],
    fsPaths: manifest.fsPaths ?? [],
    uriSchemes: manifest.uriSchemes ?? [],
  };
}

export function intersect(declared: Grants, consented: Grants | undefined): Grants {
  const out = { ...NO_GRANTS };
  for (const kind of KINDS) out[kind] = declared[kind].filter((value) => consented?.[kind].includes(value));
  return out;
}

export function missingConsent(declared: Grants, consented: Grants | undefined): Grants {
  const out = { ...NO_GRANTS };
  for (const kind of KINDS) out[kind] = declared[kind].filter((value) => !consented?.[kind].includes(value));
  return out;
}

export function isEmpty(grants: Grants): boolean {
  return KINDS.every((kind) => grants[kind].length === 0);
}

export function methodAllowed(method: string, declared: string[]): boolean {
  return declared.some((entry) => entry === method || (entry.endsWith(".*") && method.startsWith(entry.slice(0, -1)) && !method.slice(entry.length - 1).includes(".")));
}

export function hostAllowed(url: URL, declared: string[]): boolean {
  if (url.protocol !== "https:" || url.username || url.password) return false;
  const host = url.hostname.toLowerCase();
  const port = url.port || "443";
  return declared.some((entry) => {
    const [, name, entryPort] = entry.toLowerCase().match(/^(.*?)(?::(\d{1,5}))?$/)!;
    if ((entryPort ?? "443") !== port) return false;
    return name.startsWith(".") ? host.endsWith(name) && host.length > name.length : host === name;
  });
}

export function schemeAllowed(url: string, uriSchemes: string[]): boolean {
  const scheme = url.match(/^([a-z][a-z0-9+.-]*):/i)?.[1]?.toLowerCase();
  if (!scheme) return false;
  if (scheme === "http" || scheme === "https") return /^https?:\/\/[^/]/i.test(url);
  return uriSchemes.includes(scheme);
}

function normalizePath(path: string): string {
  return path.replace(/\\/g, "/").replace(/\/+/g, "/").toLowerCase();
}

function globToRegExp(glob: string): RegExp {
  let source = "";
  for (let i = 0; i < glob.length; i++) {
    const char = glob[i];
    if (char === "*" && glob[i + 1] === "*") {
      source += glob[i + 2] === "/" ? "(?:.*/)?" : ".*";
      i += glob[i + 2] === "/" ? 2 : 1;
    } else if (char === "*") source += "[^/]*";
    else if (char === "?") source += "[^/]";
    else source += char.replace(/[.+^${}()|[\]\\]/g, "\\$&");
  }
  return new RegExp(`^${source}$`);
}

export function interpolateScope(scope: string, preferences: Record<string, unknown>): string | null {
  let missing = false;
  const value = scope.replace(/\{\{([a-zA-Z_][a-zA-Z0-9_]*)\}\}/g, (_, name: string) => {
    const pref = preferences[name];
    if (typeof pref !== "string" || !pref.trim()) missing = true;
    return typeof pref === "string" ? pref.replace(/[\\/]+$/, "") : "";
  });
  return missing ? null : value;
}

function isAbsoluteSafe(path: string): boolean {
  const normalized = normalizePath(path);
  return /^(?:[a-z]:\/|\/\/)/.test(normalized) && !normalized.split("/").includes("..");
}

export function pathAllowed(path: string, scopes: string[], preferences: Record<string, unknown>): boolean {
  if (!isAbsoluteSafe(path)) return false;
  const target = normalizePath(path);
  return scopes.some((scope) => {
    const resolved = interpolateScope(scope, preferences);
    if (!resolved) return false;
    const glob = normalizePath(resolved);
    return globToRegExp(glob).test(target) || (!/[*?]/.test(glob) && globToRegExp(`${glob.replace(/\/$/, "")}/**`).test(target));
  });
}

export function scopeRoot(scope: string, preferences: Record<string, unknown>): string | null {
  const resolved = interpolateScope(scope, preferences);
  if (!resolved) return null;
  const parts = resolved.replace(/\\/g, "/").split("/");
  const firstWild = parts.findIndex((part) => /[*?]/.test(part));
  return (firstWild < 0 ? parts : parts.slice(0, firstWild)).join("/");
}

export function globMatches(pattern: string, path: string): boolean {
  return globToRegExp(normalizePath(pattern)).test(normalizePath(path));
}
