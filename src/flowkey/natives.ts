import { invoke } from "@tauri-apps/api/core";
import type { ClipboardEntry } from "../clipboardHistory";
import { showHUD, showToast } from "../toast";
import { ask } from "./dialogs";
import { dataGet, dataSet, grantsFor, preferencesOf } from "./extensions";
import { globMatches, methodAllowed, pathAllowed, schemeAllowed, scopeRoot, type Grants } from "./gate";
import type { CallContext } from "./runtime";

export class NativeError extends Error {
  constructor(
    readonly code: string,
    message: string,
  ) {
    super(message);
  }
}

interface Call {
  id: string;
  params: Record<string, unknown>;
  grants: Grants;
  preferences: Record<string, unknown>;
  signal: AbortSignal;
  context: CallContext;
}

type Handler = (call: Call) => Promise<unknown>;

const STORAGE_LIMITS = { keys: 256, bytes: 256 * 1024 };
const CACHE_LIMITS = { keys: 512, bytes: 1024 * 1024 };
const SECRET_LIMITS = { keys: 64, bytes: 8 * 1024 };

function str(params: Record<string, unknown>, name: string, optional?: false): string;
function str(params: Record<string, unknown>, name: string, optional: true): string | undefined;
function str(params: Record<string, unknown>, name: string, optional = false): string | undefined {
  const value = params[name];
  if (value === undefined || value === null) {
    if (optional) return undefined;
    throw new NativeError("invalidParams", `'${name}' is required`);
  }
  if (typeof value !== "string") throw new NativeError("invalidParams", `'${name}' must be a string`);
  return value;
}

function key(params: Record<string, unknown>): string {
  const value = str(params, "key");
  if (!value || value.length > 256) throw new NativeError("invalidParams", "keys must be 1–256 characters");
  return value;
}

type Entries = Record<string, { value: unknown; expiresAt?: number }>;
const stores = new Map<string, Promise<Entries>>();
const writes = new Map<string, Promise<void>>();

function load(id: string, name: "storage" | "cache"): Promise<Entries> {
  const cacheKey = `${id}/${name}`;
  if (!stores.has(cacheKey)) stores.set(cacheKey, dataGet<Entries>(id, name).then((v) => v ?? {}));
  return stores.get(cacheKey)!;
}

function persist(id: string, name: string, entries: Entries) {
  const cacheKey = `${id}/${name}`;
  const next = (writes.get(cacheKey) ?? Promise.resolve()).then(() => dataSet(id, name, entries));
  writes.set(cacheKey, next.catch(() => {}));
  return next;
}

export function forgetStores(id: string) {
  stores.delete(`${id}/storage`);
  stores.delete(`${id}/cache`);
}

function kv(name: "storage" | "cache", limits: { keys: number; bytes: number }): Record<string, Handler> {
  const live = (entries: Entries, k: string) => {
    const entry = entries[k];
    if (entry?.expiresAt !== undefined && entry.expiresAt <= Date.now()) {
      delete entries[k];
      return undefined;
    }
    return entry;
  };
  return {
    get: async ({ id, params }) => ({ ok: true, value: live(await load(id, name), key(params))?.value ?? null }),
    set: async ({ id, params }) => {
      const entries = await load(id, name);
      const k = key(params);
      const ttl = params.ttlSeconds;
      if (ttl !== undefined && ttl !== null && (typeof ttl !== "number" || ttl <= 0)) throw new NativeError("invalidParams", "ttlSeconds must be a positive number");
      const next = { ...entries, [k]: { value: params.value ?? null, ...(typeof ttl === "number" ? { expiresAt: Date.now() + ttl * 1000 } : {}) } };
      Object.keys(next).forEach((existing) => live(next, existing));
      if (Object.keys(next).length > limits.keys) throw new NativeError("quotaExceeded", `${name} is limited to ${limits.keys} keys`);
      if (JSON.stringify(next).length > limits.bytes) throw new NativeError("quotaExceeded", `${name} is limited to ${limits.bytes / 1024} KB`);
      Object.assign(entries, next);
      await persist(id, name, entries);
      return { ok: true };
    },
    delete: async ({ id, params }) => {
      const entries = await load(id, name);
      delete entries[key(params)];
      await persist(id, name, entries);
      return { ok: true };
    },
    keys: async ({ id }) => {
      const entries = await load(id, name);
      return { ok: true, keys: Object.keys(entries).filter((k) => live(entries, k)) };
    },
    clear: async ({ id }) => {
      const entries = await load(id, name);
      Object.keys(entries).forEach((k) => delete entries[k]);
      await persist(id, name, entries);
      return { ok: true };
    },
  };
}

function checkedPath(call: Call, name = "path"): string {
  const path = str(call.params, name);
  if (!pathAllowed(path, call.grants.fsPaths, call.preferences)) throw new NativeError("pathNotAllowed", `'${path}' is outside the extension's declared fsPaths`);
  return path;
}

function fs(op: string, args: Record<string, unknown>) {
  return invoke<Record<string, unknown>>("flowkey_fs", { op, ...args });
}

function historyText(entry: ClipboardEntry): string {
  return entry.text ?? entry.paths?.join("\n") ?? "Image";
}

const storage = kv("storage", STORAGE_LIMITS);
const cache = kv("cache", CACHE_LIMITS);

const METHODS: Record<string, Handler> = {
  "http.fetch": async ({ id, params, grants }) => {
    const auth = str(params, "auth", true);
    if (auth !== undefined && !grants.oauth.includes(auth)) throw new NativeError("providerNotDeclared", `extension did not declare oauth provider '${auth}'`);
    return invoke("flowkey_http", {
      extensionId: id,
      request: {
        url: str(params, "url"),
        method: str(params, "method", true),
        headers: params.headers ?? {},
        body: str(params, "body", true),
        auth,
        discardBody: params.discardBody === true,
      },
      hosts: grants.httpHosts,
    });
  },
  "http.upload": async ({ id, params, grants }) =>
    invoke("flowkey_http", {
      extensionId: id,
      request: {
        url: str(params, "url"),
        method: str(params, "method", true) ?? "POST",
        headers: params.headers ?? {},
        uploadBytes: typeof params.bytes === "number" ? Math.max(0, Math.min(params.bytes, 512 * 1024 * 1024)) : 0,
        timeoutMs: typeof params.timeoutMs === "number" ? params.timeoutMs : undefined,
      },
      hosts: grants.httpHosts,
    }),

  "clipboard.read": () => invoke("read_clipboard_rich"),
  "clipboard.write": async ({ params }) => {
    const paths = Array.isArray(params.paths) ? params.paths.map(String) : undefined;
    await invoke("copy_rich", { text: str(params, "text", true), html: str(params, "html", true), paths });
    return { ok: true };
  },
  "clipboard.paste": async ({ params }) => {
    await invoke("paste_text", { text: str(params, "text") });
    return { ok: true };
  },
  "clipboard.clear": async () => {
    await invoke("copy_rich", {});
    return { ok: true };
  },
  "clipboard.history": async ({ params }) => {
    const query = (str(params, "query", true) ?? "").toLowerCase();
    const limit = typeof params.limit === "number" ? params.limit : 100;
    const entries = await invoke<ClipboardEntry[]>("clipboard_list");
    return { items: entries.map((e) => ({ id: e.id, text: historyText(e) })).filter((e) => e.text.toLowerCase().includes(query)).slice(0, limit) };
  },
  "clipboard.clearHistory": async () => invoke("clipboard_delete_all").then(() => ({ ok: true })),
  "clipboard.deleteEntry": async ({ params }) => invoke("clipboard_delete", { id: str(params, "id") }).then(() => ({ ok: true })),
  "clipboard.copyEntry": async ({ params }) => invoke("clipboard_copy", { id: str(params, "id") }).then(() => ({ ok: true })),
  "clipboard.pasteEntry": async ({ params }) => invoke("clipboard_paste", { id: str(params, "id") }).then(() => ({ ok: true })),
  "clipboard.editEntry": async ({ params }) => invoke("flowkey_edit_clipboard_entry", { id: str(params, "id") }).then(() => ({ ok: true })),

  "storage.get": storage.get,
  "storage.set": storage.set,
  "storage.delete": storage.delete,
  "storage.keys": storage.keys,
  "cache.get": cache.get,
  "cache.set": cache.set,
  "cache.delete": cache.delete,
  "cache.clear": cache.clear,

  "secrets.get": async ({ id, params }) => ({ ok: true, value: (await invoke<string | null>("flowkey_secret", { extensionId: id, op: "get", key: key(params) })) ?? undefined }),
  "secrets.set": async ({ id, params }) => {
    const value = str(params, "value");
    if (new TextEncoder().encode(value).length > SECRET_LIMITS.bytes) throw new NativeError("quotaExceeded", "secret values are limited to 8 KB");
    await invoke("flowkey_secret", { extensionId: id, op: "set", key: key(params), value, maxKeys: SECRET_LIMITS.keys });
    return { ok: true };
  },
  "secrets.delete": async ({ id, params }) => {
    await invoke("flowkey_secret", { extensionId: id, op: "delete", key: key(params) });
    return { ok: true };
  },

  "oauth.authorize": (call) => oauth(call, "authorize"),
  "oauth.status": (call) => oauth(call, "status"),
  "oauth.disconnect": (call) => oauth(call, "disconnect"),

  "shell.openUrl": async ({ params, grants }) => {
    const url = str(params, "url");
    if (!schemeAllowed(url, grants.uriSchemes)) throw new NativeError("schemeNotAllowed", `'${url}' is not an http(s) link or a declared uriScheme`);
    await invoke("open_url", { url });
    return { ok: true };
  },
  "shell.open": async ({ params }) => {
    const target = str(params, "target", true) ?? str(params, "url", true) ?? str(params, "path");
    if (/^https?:\/\/[^/]/i.test(target)) await invoke("open_url", { url: target });
    else if (/^(?:[a-z]:[\\/]|\\\\)/i.test(target) && (await fs("exists", { path: target })).exists) await invoke("open_url", { url: target });
    else throw new NativeError("invalidTarget", "shell.open accepts http(s) links or existing absolute paths");
    return { ok: true };
  },
  "shell.openPath": async (call) => {
    await invoke("open_url", { url: checkedPath(call) });
    return { ok: true };
  },
  "shell.revealPath": async (call) => {
    await invoke("show_in_explorer", { path: checkedPath(call) });
    return { ok: true };
  },

  "fs.readText": async (call) => ({ ok: true, content: (await fs("readText", { path: checkedPath(call) })).content }),
  "fs.writeText": async (call) => {
    await fs("writeText", { path: checkedPath(call), content: str(call.params, "content"), append: call.params.append === true });
    return { ok: true };
  },
  "fs.delete": async (call) => fs("delete", { path: checkedPath(call) }).then(() => ({ ok: true })),
  "fs.stat": async (call) => (await fs("stat", { path: checkedPath(call) })).entry,
  "fs.mkdir": async (call) => fs("mkdir", { path: checkedPath(call) }).then(() => ({ ok: true })),
  "fs.exists": async (call) => ({ ok: true, exists: (await fs("exists", { path: checkedPath(call) })).exists === true }),
  "fs.copy": async (call) => fs("copy", { path: checkedPath(call, "from"), to: checkedPath(call, "to") }).then(() => ({ ok: true })),
  "fs.move": async (call) => fs("move", { path: checkedPath(call, "from"), to: checkedPath(call, "to") }).then(() => ({ ok: true })),
  "fs.trash": async (call) => fs("trash", { path: checkedPath(call) }).then(() => ({ ok: true })),
  "fs.glob": async ({ params, grants, preferences }) => {
    const pattern = str(params, "pattern");
    const limit = typeof params.limit === "number" ? Math.max(1, Math.min(params.limit, 5000)) : 500;
    const absolute = /^(?:[a-z]:[\\/]|\\\\)/i.test(pattern);
    const roots = grants.fsPaths.map((scope) => scopeRoot(scope, preferences)).filter((root): root is string => !!root);
    const entries: { path: string }[] = [];
    for (const root of new Set(roots)) {
      const full = absolute ? pattern : `${root.replace(/[\\/]+$/, "")}/${pattern}`;
      const walked = (await fs("walk", { path: root, limit: 20000 })).entries as { path: string }[];
      for (const entry of walked) {
        if (entries.length >= limit) break;
        if (globMatches(full, entry.path) && pathAllowed(entry.path, grants.fsPaths, preferences) && !entries.some((e) => e.path === entry.path)) entries.push(entry);
      }
    }
    return { ok: true, entries };
  },

  "apps.list": async ({ params }) => {
    const query = (str(params, "query", true) ?? "").toLowerCase();
    const apps = await invoke<{ name: string; path: string }[]>("list_apps");
    return {
      apps: apps
        .filter((app) => app.name.toLowerCase().includes(query))
        .map((app) => ({ id: app.path, name: app.name, iconUri: `http://appicon.localhost/${encodeURIComponent(app.path)}?size=64` })),
    };
  },
  "apps.launch": async ({ params }) => {
    const id = str(params, "id");
    const apps = await invoke<{ name: string; path: string }[]>("list_apps");
    const app = apps.find((a) => a.path === id) ?? apps.find((a) => a.name.toLowerCase() === id.toLowerCase());
    if (!app) throw new NativeError("appNotFound", `no installed application matches '${id}'`);
    await invoke("launch_app", { path: app.path });
    return { ok: true };
  },
  "apps.frontmost": async () => {
    const app = await invoke<{ name: string; path: string } | null>("frontmost_app");
    return { app: app ? { id: app.path, name: app.name, path: app.path } : null };
  },
  "apps.default": async ({ params }) => ({ path: await invoke<string | null>("flowkey_default_app", { path: str(params, "path") }) }),

  "media.current": () => invoke("flowkey_media_current"),
  "media.control": async ({ params }) => {
    const command = str(params, "command");
    if (!["playPause", "next", "previous"].includes(command)) throw new NativeError("invalidParams", "command must be playPause, next or previous");
    await invoke("flowkey_media_control", { command });
    return { ok: true };
  },

  "image.fetch": async ({ params, grants }) => ({ ok: true, uri: await invoke<string>("flowkey_image", { url: str(params, "url"), hosts: grants.httpHosts }) }),

  "system.selectedText": async ({ params }) => ({ text: await invoke<string | null>("flowkey_selected_text", { allowFallback: params.allowFallback === true }) }),

  "hud.show": async ({ params }) => {
    await invoke("hide_launcher", { restoreFocus: true });
    showHUD({ variant: "success", title: str(params, "title"), icon: str(params, "icon", true) });
    return { ok: true };
  },
  "toast.show": async ({ params }) => {
    showToast({ variant: params.style === "failure" ? "failure" : "success", title: str(params, "title"), message: str(params, "message", true), icon: str(params, "icon", true) });
    return { ok: true };
  },
  "alert.confirm": async ({ params, signal }) => {
    await invoke("show_launcher");
    const confirmed = await ask(
      {
        title: str(params, "title"),
        message: str(params, "message", true),
        confirmTitle: str(params, "confirmTitle", true) ?? "Confirm",
        cancelTitle: str(params, "cancelTitle", true) ?? "Cancel",
        destructive: params.destructive === true,
      },
      signal,
    );
    return { confirmed };
  },
};

async function oauth({ id, params, grants }: Call, op: "authorize" | "status" | "disconnect") {
  const provider = str(params, "provider");
  if (!grants.oauth.includes(provider)) throw new NativeError("providerNotDeclared", `extension did not declare oauth provider '${provider}' in its manifest`);
  return invoke("flowkey_oauth", { extensionId: id, op, provider, clientId: str(params, "clientId", true) ?? null });
}

export const SUPPORTED_METHODS = Object.keys(METHODS);

export async function dispatchNative(context: CallContext, method: string, params: Record<string, unknown>, signal: AbortSignal): Promise<unknown> {
  const handler = METHODS[method];
  const grants = await grantsFor(context.extension);
  if (!methodAllowed(method, grants.nativeMethods)) throw new NativeError("methodNotDeclared", `native method '${method}' is not declared in the manifest or was not granted`);
  if (!handler) throw new NativeError("methodNotSupported", `native method '${method}' is not supported by Openray`);
  try {
    return await handler({ id: context.extension.manifest.id, params, grants, preferences: await preferencesOf(context.extension.manifest), signal, context });
  } catch (error) {
    if (error instanceof NativeError) throw { code: error.code, message: error.message };
    if (error && typeof error === "object" && "code" in error) throw error;
    throw { code: "nativeFailed", message: error instanceof Error ? error.message : String(error) };
  }
}
