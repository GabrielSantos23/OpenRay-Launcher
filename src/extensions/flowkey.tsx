import { invoke } from "@tauri-apps/api/core";
import { emitTo, listen } from "@tauri-apps/api/event";
import type { Arguments, CommandArgument, ManifestCommand, UiItem } from "@flowkey-cli/native-sdk";
import { useEffect, useRef, useState } from "react";
import { Footer } from "../ActionHost";
import { Dropdown } from "../Dropdown";
import type { Command, Extension, ViewProps } from "../extension-api";
import { NOTCH_ACTION_EVENT, NOTCH_READY_EVENT, NOTCH_WIDGETS_EVENT, notchItems, type NotchWidget } from "../notch";
import { isEnabled, type Settings } from "../settings";
import { BackButton } from "../ListView";
import { showToast } from "../toast";
import { ask } from "../flowkey/dialogs";
import { CHANGED_EVENT, REMOVED_EVENT, describeGrants, forgetCached, grantConsent, isNotchCommand, listExtensions, missingPreferences, pendingConsent, preferencesOf, type Failure } from "../flowkey/extensions";
import { FlowKeyIcon } from "../flowkey/FlowKeyIcon";
import { FlowKeyView, LoadingView, WINDOW_EVENT, type WindowSignal } from "../flowkey/FlowKeyView";
import { dispatchNative, forgetStores } from "../flowkey/natives";
import { hostFor, installHostHooks, loadedHost, newViewId, unloadHost, type CallContext, type InstalledExtension } from "../flowkey/runtime";
import { WebCommandView } from "../flowkey/WebCommand";

export const LAUNCH_EVENT = "flowkey-launch";
const BACKGROUND_LIFETIME_MS = 60_000;
const BACKGROUND_LINGER_MS = 5_000;

export function extensionKey(extension: InstalledExtension): string {
  return `flowkey.${extension.manifest.id}`;
}

interface BackgroundView {
  extension: InstalledExtension;
  pending: number;
  linger: boolean;
  timer?: number;
}

const background = new Map<string, BackgroundView>();
const launchQueries = new Map<string, string>();

function closeBackground(viewId: string) {
  const entry = background.get(viewId);
  if (!entry) return;
  window.clearTimeout(entry.timer);
  background.delete(viewId);
  loadedHost(entry.extension.manifest.id)?.close(viewId);
}

function backgroundView(extension: InstalledExtension, viewId?: string): BackgroundView | undefined {
  if (!viewId?.startsWith("bg:")) return undefined;
  let entry = background.get(viewId);
  if (!entry) {
    entry = { extension, pending: 0, linger: false };
    background.set(viewId, entry);
  }
  return entry;
}

function arm(viewId: string, entry: BackgroundView) {
  window.clearTimeout(entry.timer);
  if (entry.pending > 0) return;
  entry.timer = window.setTimeout(() => closeBackground(viewId), entry.linger ? BACKGROUND_LINGER_MS : BACKGROUND_LIFETIME_MS);
}

function lingerBackground(context: CallContext) {
  const entry = backgroundView(context.extension, context.viewId);
  if (!entry || !context.viewId) return;
  entry.linger = true;
  arm(context.viewId, entry);
}

installHostHooks({
  native: async (context, method, params, signal) => {
    const entry = backgroundView(context.extension, context.viewId);
    if (entry) {
      entry.pending++;
      window.clearTimeout(entry.timer);
    }
    try {
      return await dispatchNative(context, method, params, signal);
    } finally {
      if (entry && context.viewId) {
        entry.pending--;
        if (method === "hud.show") entry.linger = true;
        arm(context.viewId, entry);
      }
    }
  },
  windowCommand: (context, command) => {
    if (command === "closeMainWindow") invoke("hide_launcher", { restoreFocus: true }).catch((e) => console.error("hide_launcher failed", e));
    if (command !== "clearSearchBar") lingerBackground(context);
    const detail: WindowSignal = { extensionId: context.extension.manifest.id, viewId: context.viewId?.startsWith("bg:") ? undefined : context.viewId, command };
    window.dispatchEvent(new CustomEvent(WINDOW_EVENT, { detail }));
  },
  launchCommand: (extension, commandId, query) => {
    const key = `${extensionKey(extension)}/${commandId}`;
    if (query) launchQueries.set(key, query);
    else launchQueries.delete(key);
    window.dispatchEvent(new CustomEvent(LAUNCH_EVENT, { detail: { key, query } }));
  },
  preferences: (extension) => preferencesOf(extension.manifest),
});

listen<string>(REMOVED_EVENT, (event) => {
  forgetCached(event.payload);
  forgetStores(event.payload);
  unloadHost(event.payload);
}).catch((e) => console.error("could not watch FlowKey removals", e));

listen<string>(CHANGED_EVENT, (event) => {
  forgetCached(event.payload);
  loadedHost(event.payload)
    ?.updatePreferences()
    .catch((e) => console.error("preference update failed", e));
}).catch((e) => console.error("could not watch FlowKey changes", e));

const readiness = new Map<string, Promise<boolean>>();

export function ensureReady(extension: InstalledExtension, interactive = true): Promise<boolean> {
  const key = `${extension.manifest.id}:${interactive}`;
  const running = readiness.get(key);
  if (running) return running;
  const check = checkReady(extension, interactive).finally(() => readiness.delete(key));
  readiness.set(key, check);
  return check;
}

async function checkReady(extension: InstalledExtension, interactive: boolean): Promise<boolean> {
  const missing = await pendingConsent(extension);
  if (missing) {
    if (!interactive) return false;
    await invoke("show_launcher");
    const allowed = await ask({
      title: `Allow “${extension.manifest.name}”?`,
      message: "This extension was installed outside Openray and needs your permission to use:",
      items: describeGrants(missing),
      confirmTitle: "Allow",
      cancelTitle: "Cancel",
      destructive: false,
    });
    if (!allowed) return false;
    await grantConsent(extension.manifest);
  }
  const required = missingPreferences(extension.manifest, await preferencesOf(extension.manifest));
  if (required.length) {
    if (interactive) {
      showToast({ variant: "failure", title: `${extension.manifest.name} needs setup`, message: `Set ${required.map((p) => p.title).join(", ")} in Settings → Extensions` });
      invoke("open_settings").catch((e) => console.error("open_settings failed", e));
    }
    return false;
  }
  return true;
}

export async function runBackground(extension: InstalledExtension, commandId: string, args?: Arguments, interactive = true) {
  if (!(await ensureReady(extension, interactive))) return;
  const host = hostFor(extension);
  try {
    const viewId = await host.background(commandId, args);
    const entry = backgroundView(extension, viewId);
    if (entry) arm(viewId, entry);
  } catch (error) {
    const message = (error as { message?: string })?.message ?? String(error);
    showToast({ variant: "failure", title: `${extension.manifest.commands.find((c) => c.id === commandId)?.title ?? commandId} failed`, message });
  }
}

function initialArguments(definitions: CommandArgument[]): Record<string, string> {
  return Object.fromEntries(definitions.map((a) => [a.name, a.type === "dropdown" ? (a.data?.[0]?.value ?? "") : ""]));
}

function ArgumentsView({ extension, command, onSubmit, onBack }: { extension: InstalledExtension; command: ManifestCommand; onSubmit: (values: Arguments) => void; onBack: () => void }) {
  const definitions = command.arguments ?? [];
  const [values, setValues] = useState(() => initialArguments(definitions));
  const refs = useRef<(HTMLInputElement | null)[]>([]);
  const submit = () => {
    const missing = definitions.findIndex((a) => a.required && !values[a.name]?.trim());
    if (missing >= 0) {
      refs.current[missing]?.focus();
      showToast({ variant: "failure", title: `${definitions[missing].placeholder} is required` });
      return;
    }
    onSubmit(Object.fromEntries(Object.entries(values).filter(([, v]) => v !== "")));
  };
  return (
    <div
      className="flex min-h-0 flex-1 flex-col"
      onKeyDown={(e) => {
        if (e.key === "Enter") {
          e.preventDefault();
          submit();
        } else if (e.key === "Escape" || (e.key === "Backspace" && e.target instanceof HTMLInputElement && !e.target.value && e.target === refs.current[0])) {
          e.preventDefault();
          onBack();
        }
      }}
    >
      <header className="relative flex h-header shrink-0 items-center gap-3 border-b border-separator px-search-inset">
        <BackButton onBack={onBack} />
        <FlowKeyIcon fields={{ icon: command.icon ?? extension.manifest.icon, iconColor: command.iconColor }} extension={extension} title={command.title} />
        <span className="shrink-0 text-18 text-fg">{command.title}</span>
        {definitions.map((definition, i) =>
          definition.type === "dropdown" ? (
            <span key={definition.name} className="shrink-0">
              <span id={`argument-${definition.name}`} className="sr-only">
                {definition.placeholder}
              </span>
              <Dropdown
                value={values[definition.name]}
                options={(definition.data ?? []).map((d) => ({ value: d.value, label: d.title }))}
                onChange={(value) => setValues((v) => ({ ...v, [definition.name]: value }))}
                labelledBy={`argument-${definition.name}`}
                size="small"
              />
            </span>
          ) : (
            <input
              key={definition.name}
              ref={(el) => {
                refs.current[i] = el;
              }}
              autoFocus={i === 0}
              type={definition.type === "password" ? "password" : "text"}
              spellCheck={false}
              value={values[definition.name]}
              onChange={(e) => setValues((v) => ({ ...v, [definition.name]: e.target.value }))}
              placeholder={definition.required ? definition.placeholder : `${definition.placeholder} (optional)`}
              aria-label={definition.placeholder}
              className="h-6 w-[150px] min-w-0 rounded-[5px] bg-control px-2 text-13 text-fg caret-fg outline-none placeholder:text-fg-60 focus:bg-control-hover"
            />
          ),
        )}
      </header>
      <div className="flex min-h-0 flex-1 items-center justify-center px-6 text-center text-13 text-fg-60">Fill in the arguments, then press Enter.</div>
      <Footer left={<span className="truncate text-fg-60">{extension.manifest.name}</span>} primaryTitle={command.mode === "background" ? "Run Command" : "Open Command"} panelOpen={false} showActions={false} onTogglePanel={() => {}} />
    </div>
  );
}

function CommandScreen({ extension, command, pop }: { extension: InstalledExtension; command: ManifestCommand } & ViewProps) {
  const [ready, setReady] = useState(false);
  const [initialQuery] = useState(() => {
    const key = `${extensionKey(extension)}/${command.id}`;
    const query = launchQueries.get(key);
    launchQueries.delete(key);
    return query;
  });
  const [args, setArgs] = useState<Arguments | undefined>(command.arguments?.length ? undefined : {});
  useEffect(() => {
    let alive = true;
    ensureReady(extension).then((ok) => {
      if (!alive) return;
      if (ok) setReady(true);
      else pop();
    });
    return () => {
      alive = false;
    };
  }, [extension]);
  if (!ready) return <LoadingView command={command} onBack={pop} />;
  if (!args) {
    return (
      <ArgumentsView
        extension={extension}
        command={command}
        onBack={pop}
        onSubmit={(values) => {
          if (command.mode === "background") {
            void runBackground(extension, command.id, values);
            invoke("hide_launcher", { restoreFocus: true }).catch((e) => console.error("hide_launcher failed", e));
          } else setArgs(values);
        }}
      />
    );
  }
  if (command.ui === "web") return <WebCommandView extension={extension} command={command} args={args} pop={pop} />;
  return <FlowKeyView extension={extension} command={command} args={command.arguments?.length ? args : undefined} initialQuery={initialQuery} pop={pop} />;
}

function commandsFor(extension: InstalledExtension): Command[] {
  return extension.manifest.commands.map((command) => {
    const base = {
      id: command.id,
      title: command.title,
      subtitle: command.subtitle ?? extension.manifest.name,
      icon: <FlowKeyIcon fields={{ icon: command.icon ?? extension.manifest.icon, iconColor: command.iconColor }} extension={extension} title={command.title} />,
      keywords: command.keywords,
      kind: "Command",
    };
    if (command.mode === "background" && !command.arguments?.length) return { ...base, primaryActionTitle: "Run Command", run: () => runBackground(extension, command.id) };
    return { ...base, primaryActionTitle: "Open Command", view: ({ pop }: ViewProps) => <CommandScreen extension={extension} command={command} pop={pop} /> };
  });
}

const intervals = new Map<string, number>();

function scheduleIntervals(extensions: InstalledExtension[]) {
  const wanted = new Set<string>();
  for (const extension of extensions) {
    for (const command of extension.manifest.commands) {
      if (command.mode !== "background" || !command.interval || command.interval < 60) continue;
      const key = `${extensionKey(extension)}/${command.id}@${extension.manifest.version}:${command.interval}`;
      wanted.add(key);
      if (!intervals.has(key)) intervals.set(key, window.setInterval(() => void runBackground(extension, command.id, undefined, false), command.interval * 1000));
    }
  }
  for (const [key, timer] of intervals) {
    if (!wanted.has(key)) {
      window.clearInterval(timer);
      intervals.delete(key);
    }
  }
}

const notchViews = new Map<string, NotchWidget & { extension: InstalledExtension; viewId: string }>();

function publishNotch() {
  const widgets: NotchWidget[] = [...notchViews.values()].map(({ key, title, icon, tree, error }) => ({ key, title, icon, tree, error }));
  emitTo("notch", NOTCH_WIDGETS_EVENT, widgets).catch(() => {});
}

async function bridgeNotch(extensions: InstalledExtension[]) {
  const settings = await invoke<Settings>("settings_get");
  const wanted = new Set<string>();
  for (const extension of extensions) {
    for (const command of extension.manifest.commands.filter(isNotchCommand)) {
      const key = `${extensionKey(extension)}/${command.id}`;
      if (!isEnabled(settings, key, extensionKey(extension)) || !(await ensureReady(extension, false))) continue;
      wanted.add(key);
      if (notchViews.get(key)?.extension.manifest.version === extension.manifest.version) continue;
      const entry = { key, title: command.title, icon: command.icon ?? extension.manifest.icon, tree: null, extension, viewId: newViewId() };
      notchViews.set(key, entry);
      hostFor(extension)
        .render(entry.viewId, command.id, { query: "" }, {
          onTree: (tree) => {
            Object.assign(entry, { tree, error: undefined });
            publishNotch();
          },
          onError: (error) => {
            Object.assign(entry, { error: error.message });
            publishNotch();
          },
        })
        .catch((error) => console.error("notch widget failed", key, error));
    }
  }
  for (const [key, entry] of notchViews) {
    if (wanted.has(key)) continue;
    loadedHost(entry.extension.manifest.id)?.close(entry.viewId);
    notchViews.delete(key);
  }
  publishNotch();
}

listen(NOTCH_READY_EVENT, publishNotch).catch((e) => console.error("could not watch the notch", e));

listen<{ key: string; itemId?: string; actionId: string }>(NOTCH_ACTION_EVENT, ({ payload }) => {
  const entry = notchViews.get(payload.key);
  if (!entry) return;
  const item: UiItem | undefined = entry.tree ? notchItems(entry.tree).find((i) => i.id === payload.itemId) : undefined;
  hostFor(entry.extension)
    .action(entry.viewId, payload.actionId, item)
    .catch((error) => console.error("notch action failed", payload.key, error));
}).catch((e) => console.error("could not watch notch actions", e));

export let flowKeyFailures: Failure[] = [];
export const flowKeyInstalled = new Map<string, InstalledExtension>();

export async function loadFlowKeyExtensions(schedule = false): Promise<Extension[]> {
  try {
    const { extensions, failures } = await listExtensions();
    flowKeyFailures = failures;
    flowKeyInstalled.clear();
    extensions.forEach((extension) => flowKeyInstalled.set(extensionKey(extension), extension));
    failures.forEach((f) => console.warn(`FlowKey extension ${f.id} (${f.source}) failed to load: ${f.message}`));
    if (schedule) {
      scheduleIntervals(extensions);
      bridgeNotch(extensions).catch((e) => console.error("could not load notch widgets", e));
    }
    return extensions.map((extension) => ({ id: extensionKey(extension), title: extension.manifest.name, commands: commandsFor(extension) }));
  } catch (error) {
    console.error("could not list FlowKey extensions", error);
    return [];
  }
}
