import type { Arguments, ExtensionManifest, UiItem, UiTree } from "@flowkey-cli/native-sdk";
import type { FromWorker, ProtocolError, ToWorker } from "./protocol";
import workerUrl from "./worker.ts?worker&url";

export interface InstalledExtension {
  manifest: ExtensionManifest;
  source: "installed" | "bundled";
  dir: string;
  baseUrl: string;
}

export interface CallContext {
  extension: InstalledExtension;
  viewId?: string;
}

export interface HostHooks {
  native: (context: CallContext, method: string, params: Record<string, unknown>, signal: AbortSignal) => Promise<unknown>;
  windowCommand: (context: CallContext, command: "closeMainWindow" | "popToRoot" | "clearSearchBar") => void;
  launchCommand: (extension: InstalledExtension, commandId: string, query?: string) => void;
  preferences: (extension: InstalledExtension) => Promise<Record<string, unknown>>;
}

interface ViewListener {
  onTree: (tree: UiTree) => void;
  onError: (error: ProtocolError) => void;
}

let hooks: HostHooks | null = null;
let nextId = 1;
const hosts = new Map<string, ExtensionHost>();

export function installHostHooks(value: HostHooks) {
  hooks = value;
}

export function newViewId(): string {
  return `v${Date.now().toString(36)}${nextId++}`;
}

export class ExtensionHost {
  private worker: Worker;
  private ready: Promise<"react" | "functional">;
  private listeners = new Map<string, ViewListener>();
  private waiters = new Map<string, { resolve: () => void; reject: (error: ProtocolError) => void }>();
  private calls = new Map<string, AbortController>();

  constructor(readonly extension: InstalledExtension) {
    const loader = new Blob([`import ${JSON.stringify(new URL(workerUrl, location.href).href)};`], { type: "text/javascript" });
    const loaderUrl = URL.createObjectURL(loader);
    this.worker = new Worker(loaderUrl, { type: "module", name: `flowkey:${extension.manifest.id}` });
    setTimeout(() => URL.revokeObjectURL(loaderUrl), 30_000);
    this.worker.addEventListener("message", (event: MessageEvent<FromWorker>) => this.receive(event.data));
    this.worker.addEventListener("error", (event) => {
      const error = { code: "workerCrashed", message: event.message || "the extension runtime stopped" };
      this.listeners.forEach((listener) => listener.onError(error));
    });
    this.ready = (async () => {
      const preferences = await hooks!.preferences(extension);
      return new Promise<"react" | "functional">((resolve, reject) => {
        const requestId = `load${nextId++}`;
        const onLoaded = (event: MessageEvent<FromWorker>) => {
          const message = event.data;
          if (message.type !== "loaded" || message.requestId !== requestId) return;
          this.worker.removeEventListener("message", onLoaded);
          message.ok ? resolve(message.kind) : reject(message.error);
        };
        this.worker.addEventListener("message", onLoaded);
        this.send({ type: "load", requestId, extensionId: extension.manifest.id, url: `${extension.baseUrl}${extension.manifest.entry ?? "main.js"}`, manifest: extension.manifest, preferences, isDevelopment: false });
      });
    })();
    this.ready.catch(() => {});
  }

  private send(message: ToWorker) {
    this.worker.postMessage(message);
  }

  loaded() {
    return this.ready;
  }

  private receive(message: FromWorker) {
    const context = (viewId?: string): CallContext => ({ extension: this.extension, viewId });
    switch (message.type) {
      case "tree":
        this.listeners.get(message.viewId)?.onTree(message.tree);
        return;
      case "viewError":
        this.listeners.get(message.viewId)?.onError(message.error);
        return;
      case "done": {
        const waiter = this.waiters.get(message.requestId);
        this.waiters.delete(message.requestId);
        if (message.ok) waiter?.resolve();
        else waiter?.reject(message.error);
        return;
      }
      case "nativeCall": {
        const controller = new AbortController();
        this.calls.set(message.requestId, controller);
        hooks!
          .native(context(message.viewId), message.method, message.params ?? {}, controller.signal)
          .then(
            (result) => this.send({ type: "nativeResult", requestId: message.requestId, ok: true, result }),
            (error: unknown) => this.send({ type: "nativeResult", requestId: message.requestId, ok: false, error: asProtocolError(error) }),
          )
          .finally(() => this.calls.delete(message.requestId));
        return;
      }
      case "nativeAbort":
        this.calls.get(message.requestId)?.abort();
        return;
      case "windowCommand":
        hooks!.windowCommand(context(message.viewId), message.command);
        return;
      case "launchCommand":
        hooks!.launchCommand(this.extension, message.commandId, message.query);
        return;
      case "log":
        console[message.level === "debug" ? "log" : message.level](`[${this.extension.manifest.id}]`, message.message);
        return;
    }
  }

  private request(build: (requestId: string) => ToWorker): Promise<void> {
    const requestId = `r${nextId++}`;
    return new Promise((resolve, reject) => {
      this.waiters.set(requestId, { resolve, reject });
      this.send(build(requestId));
    });
  }

  async render(viewId: string, commandId: string, props: { query: string; filterValue?: string; arguments?: Arguments; reset?: boolean }, listener: ViewListener) {
    this.listeners.set(viewId, listener);
    await this.ready;
    this.send({ type: "render", viewId, extensionId: this.extension.manifest.id, commandId, ...props });
  }

  async action(viewId: string, actionId: string, item?: UiItem, formValues?: Record<string, unknown>) {
    await this.ready;
    await this.request((requestId) => ({ type: "action", requestId, viewId, actionId, item, formValues }));
  }

  async background(commandId: string, args?: Arguments): Promise<string> {
    await this.ready;
    let viewId = "";
    await this.request((requestId) => {
      viewId = `bg:${requestId}`;
      return { type: "background", requestId, extensionId: this.extension.manifest.id, commandId, arguments: args };
    });
    return viewId;
  }

  close(viewId: string) {
    this.listeners.delete(viewId);
    this.send({ type: "close", viewId });
  }

  async updatePreferences() {
    this.send({ type: "preferences", extensionId: this.extension.manifest.id, values: await hooks!.preferences(this.extension) });
  }

  terminate() {
    this.worker.terminate();
    this.calls.forEach((controller) => controller.abort());
    const error = { code: "aborted", message: "the extension was unloaded" };
    this.waiters.forEach((waiter) => waiter.reject(error));
  }
}

export function asProtocolError(error: unknown): ProtocolError {
  if (error && typeof error === "object" && "code" in error && "message" in error) return { code: String(error.code), message: String(error.message) };
  return { code: "nativeFailed", message: error instanceof Error ? error.message : String(error) };
}

export function hostFor(extension: InstalledExtension): ExtensionHost {
  const existing = hosts.get(extension.manifest.id);
  if (existing && existing.extension.dir === extension.dir && existing.extension.manifest.version === extension.manifest.version) return existing;
  existing?.terminate();
  const host = new ExtensionHost(extension);
  hosts.set(extension.manifest.id, host);
  return host;
}

export function unloadHost(extensionId: string) {
  hosts.get(extensionId)?.terminate();
  hosts.delete(extensionId);
}

export function loadedHost(extensionId: string): ExtensionHost | undefined {
  return hosts.get(extensionId);
}
