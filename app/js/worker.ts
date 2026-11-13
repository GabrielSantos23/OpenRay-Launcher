/// <reference lib="webworker" />
import * as React from "react";
import * as ReactJsxRuntime from "react/jsx-runtime";
import {
  Action,
  ActionPanel,
  Color,
  Detail,
  Form,
  Grid,
  Icon,
  List,
  ReactRoot,
  defineReactExtension,
  useCachedPromise,
  useCachedState,
  useDebounce,
  useFetch,
  useLocalStorage,
  usePromise,
  withCache,
  type ActionRegistry,
  type CommandProps,
  type ReactExtensionModule,
} from "@flowkey-cli/react-ui";
import {
  captureException,
  createCapabilities,
  defineExtension,
  randomId,
  type Arguments,
  type ExtensionManifest,
  type FlowKeyCapabilities,
  type HudOptions,
  type SearchHandlers,
  type UiItem,
} from "@flowkey-cli/native-sdk";
import { environmentFor, toProtocolError, type FromWorker, type ToWorker } from "./protocol";

const scope = self as unknown as DedicatedWorkerGlobalScope;
const post = scope.postMessage.bind(scope) as (message: FromWorker) => void;

(globalThis as Record<string, unknown>).__FLOWKEY_HOST__ = {
  react: React,
  reactJsx: ReactJsxRuntime,
  reactUi: {
    List,
    Detail,
    Grid,
    Form,
    ActionPanel,
    Action,
    defineReactExtension,
    usePromise,
    useFetch,
    useDebounce,
    useCachedState,
    useLocalStorage,
    useCachedPromise,
    withCache,
    Color,
    Icon,
  },
  nativeSdk: { defineExtension, createCapabilities, randomId, captureException },
};

for (const name of ["fetch", "XMLHttpRequest", "WebSocket", "WebSocketStream", "EventSource", "Worker", "SharedWorker", "indexedDB", "caches", "BroadcastChannel", "WebTransport"]) {
  try {
    Object.defineProperty(scope, name, { value: undefined, writable: false, configurable: false });
  } catch {
    continue;
  }
}

const DEFAULT_TIMEOUT_MS = 15_000;
const TIMEOUTS: Record<string, number | null> = {
  "oauth.authorize": 180_000,
  "alert.confirm": null,
  "http.fetch": 60_000,
  "http.upload": 300_000,
  "image.fetch": 30_000,
  "system.selectedText": 5_000,
};
const RETAINED_REGISTRIES = 4;

interface Extension {
  manifest: ExtensionManifest;
  preferences: Record<string, unknown>;
  isDevelopment: boolean;
  component?: ReactExtensionModule["component"];
  handlers?: SearchHandlers;
}

interface View {
  viewId: string;
  commandId: string;
  controller: AbortController;
  capabilities: FlowKeyCapabilities;
  native: CommandProps["native"];
  root?: ReactRoot;
  registries: ActionRegistry[];
  lastJson: string | null;
  pushTimer: ReturnType<typeof setTimeout> | null;
  query: string;
  filterValue?: string;
  arguments?: Arguments;
  destroyed: boolean;
}

let extension: Extension | null = null;
const views = new Map<string, View>();
const pending = new Map<string, { resolve: (value: unknown) => void; reject: (error: unknown) => void }>();
let nextCall = 1;

function nativeCall<T>(viewId: string | undefined, method: string, params?: Record<string, unknown>, options?: { signal?: AbortSignal; timeoutMs?: number }): Promise<T> {
  const requestId = `c${nextCall++}`;
  const signal = options?.signal;
  if (signal?.aborted) return Promise.reject({ code: "aborted", message: `native method ${method} aborted` });
  return new Promise<T>((resolve, reject) => {
    const timeoutMs = options?.timeoutMs ?? (method in TIMEOUTS ? TIMEOUTS[method] : DEFAULT_TIMEOUT_MS);
    let timer: ReturnType<typeof setTimeout> | undefined;
    const settle = (fn: () => void) => {
      if (!pending.has(requestId)) return;
      pending.delete(requestId);
      clearTimeout(timer);
      signal?.removeEventListener("abort", onAbort);
      fn();
    };
    const onAbort = () => {
      settle(() => reject({ code: "aborted", message: `native method ${method} aborted` }));
      post({ type: "nativeAbort", requestId });
    };
    pending.set(requestId, { resolve: (value) => settle(() => resolve(value as T)), reject: (error) => settle(() => reject(error)) });
    if (timeoutMs !== null) timer = setTimeout(() => settle(() => reject({ code: "nativeTimeout", message: `native method ${method} timed out` })), timeoutMs);
    signal?.addEventListener("abort", onAbort, { once: true });
    post({ type: "nativeCall", requestId, extensionId: extension?.manifest.id ?? "", viewId, method, params });
  });
}

function windowFor(viewId?: string): CommandProps["window"] {
  const id = extension?.manifest.id ?? "";
  return {
    closeMainWindow: () => post({ type: "windowCommand", extensionId: id, viewId, command: "closeMainWindow" }),
    popToRoot: () => post({ type: "windowCommand", extensionId: id, viewId, command: "popToRoot" }),
    clearSearchBar: () => post({ type: "windowCommand", extensionId: id, viewId, command: "clearSearchBar" }),
    launchCommand: (commandId: string, query?: string) => post({ type: "launchCommand", extensionId: id, commandId, query }),
  };
}

function createView(viewId: string, commandId: string): View {
  const controller = new AbortController();
  const native: CommandProps["native"] = {
    call: (method, params, options) => {
      if (controller.signal.aborted) return Promise.reject({ code: "aborted", message: `native method ${method} aborted` });
      return nativeCall(viewId, method, params, { timeoutMs: options?.timeoutMs, signal: anySignal(controller.signal, options?.signal) });
    },
    showHud: (options: HudOptions) => nativeCall(viewId, "hud.show", { ...options }, { signal: controller.signal }),
  };
  const view: View = {
    viewId,
    commandId,
    controller,
    native,
    capabilities: createCapabilities(native.call),
    registries: [],
    lastJson: null,
    pushTimer: null,
    query: "",
    destroyed: false,
  };
  if (extension?.component) {
    view.root = new ReactRoot(extension.component, {
      onCommit: () => schedulePush(view),
      onError: (error) => post({ type: "viewError", viewId, error: toProtocolError(error) }),
    });
  }
  views.set(viewId, view);
  return view;
}

function anySignal(a: AbortSignal, b?: AbortSignal): AbortSignal {
  return b ? AbortSignal.any([a, b]) : a;
}

function pushNow(view: View) {
  if (view.destroyed || !view.root) return;
  const generation = view.root.current;
  if (!generation || generation.json === view.lastJson) return;
  view.lastJson = generation.json;
  view.registries.unshift(generation.registry);
  view.registries.length = Math.min(view.registries.length, RETAINED_REGISTRIES);
  post({ type: "tree", viewId: view.viewId, tree: generation.tree });
}

function schedulePush(view: View) {
  if (view.pushTimer !== null || view.destroyed) return;
  view.pushTimer = setTimeout(() => {
    view.pushTimer = null;
    pushNow(view);
  }, 16);
}

function destroyView(view: View) {
  if (view.destroyed) return;
  view.destroyed = true;
  views.delete(view.viewId);
  if (view.pushTimer !== null) clearTimeout(view.pushTimer);
  view.controller.abort();
  try {
    view.root?.unmount();
  } catch (error) {
    post({ type: "log", level: "warn", message: `unmount failed: ${String(error)}` });
  }
}

function functionalContext(view: View) {
  const ext = extension!;
  return {
    preferences: ext.preferences,
    commandId: view.commandId,
    filterValue: view.filterValue,
    arguments: view.arguments,
    environment: environmentFor(ext.manifest, ext.isDevelopment, view.commandId),
    window: windowFor(view.viewId),
    native: view.native,
    capabilities: view.capabilities,
  };
}

async function render(view: View) {
  const ext = extension!;
  if (view.root) {
    view.root.update({
      query: view.query,
      filterValue: view.filterValue,
      commandId: view.commandId,
      arguments: view.arguments,
      preferences: ext.preferences,
      environment: environmentFor(ext.manifest, ext.isDevelopment, view.commandId),
      window: windowFor(view.viewId),
      native: view.native,
      capabilities: view.capabilities,
      signal: view.controller.signal,
    });
    pushNow(view);
    return;
  }
  if (!ext.handlers?.search) {
    post({ type: "tree", viewId: view.viewId, tree: { type: "list", sections: [] } });
    return;
  }
  const tree = await ext.handlers.search(view.query, functionalContext(view));
  if (!view.destroyed) post({ type: "tree", viewId: view.viewId, tree });
}

async function runAction(view: View, actionId: string, item?: UiItem, formValues?: Record<string, unknown>) {
  if (view.root) {
    const handler = view.registries.map((registry) => registry.resolve(actionId)).find(Boolean);
    if (!handler) throw { code: "unknownAction", message: `unknown action '${actionId}'` };
    await handler(formValues);
    pushNow(view);
    return;
  }
  const handlers = extension!.handlers!;
  const tree = handlers.onAction ? await handlers.onAction(actionId, item, functionalContext(view), formValues) : null;
  if (view.destroyed) return;
  if (tree) post({ type: "tree", viewId: view.viewId, tree });
  else await render(view);
}

async function load(message: Extract<ToWorker, { type: "load" }>) {
  const imported = (await import(/* @vite-ignore */ message.url)) as { default?: { component?: unknown; handlers?: unknown } };
  const exposed = imported.default ?? {};
  const base = { manifest: message.manifest, preferences: message.preferences, isDevelopment: message.isDevelopment };
  if (typeof exposed.component === "function") extension = { ...base, component: exposed.component as ReactExtensionModule["component"] };
  else if (typeof exposed.handlers === "object" && exposed.handlers !== null) extension = { ...base, handlers: exposed.handlers as SearchHandlers };
  else throw { code: "invalidBundle", message: "entry bundle must export a component or handlers object as its default export" };
  return extension.component ? "react" : "functional";
}

async function handle(message: ToWorker) {
  switch (message.type) {
    case "load":
      try {
        post({ type: "loaded", requestId: message.requestId, ok: true, kind: await load(message) });
      } catch (error) {
        post({ type: "loaded", requestId: message.requestId, ok: false, error: toProtocolError(error) });
      }
      return;
    case "preferences":
      if (extension) extension.preferences = message.values;
      return;
    case "nativeResult": {
      const call = pending.get(message.requestId);
      if (call) message.ok ? call.resolve(message.result) : call.reject(message.error);
      return;
    }
    case "close": {
      const view = views.get(message.viewId);
      if (view) destroyView(view);
      return;
    }
    case "render": {
      if (!extension) return;
      let view = views.get(message.viewId);
      if (view && message.reset) {
        destroyView(view);
        view = undefined;
      }
      view ??= createView(message.viewId, message.commandId);
      view.query = message.query;
      view.filterValue = message.filterValue;
      view.arguments = message.arguments;
      try {
        await render(view);
      } catch (error) {
        post({ type: "viewError", viewId: view.viewId, error: toProtocolError(error) });
      }
      return;
    }
    case "action": {
      const view = views.get(message.viewId);
      try {
        if (!view) throw { code: "unknownView", message: "the view is no longer open" };
        await runAction(view, message.actionId, message.item, message.formValues);
        post({ type: "done", requestId: message.requestId, ok: true });
      } catch (error) {
        if (view) pushNow(view);
        post({ type: "done", requestId: message.requestId, ok: false, error: toProtocolError(error) });
      }
      return;
    }
    case "background": {
      if (!extension) return;
      const view = createView(`bg:${message.requestId}`, message.commandId);
      view.arguments = message.arguments;
      try {
        if (extension.handlers) {
          if (!extension.handlers.command) throw { code: "unknownCommand", message: `command '${message.commandId}' has no handler` };
          await extension.handlers.command(message.commandId, functionalContext(view));
          destroyView(view);
        } else await render(view);
        post({ type: "done", requestId: message.requestId, ok: true });
      } catch (error) {
        destroyView(view);
        post({ type: "done", requestId: message.requestId, ok: false, error: toProtocolError(error) });
      }
      return;
    }
  }
}

scope.addEventListener("message", (event: MessageEvent<ToWorker>) => {
  void handle(event.data);
});
