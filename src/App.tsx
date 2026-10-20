import { useCallback, useEffect, useMemo, useRef, useState, type ComponentType } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { RefreshCw } from "lucide-react";
import { Footer, Loader, Logo, useActionHost } from "./ActionHost";
import { CommandIcon, GlyphTile } from "./icons";
import { actionGroups, matchesShortcut } from "./actions";
import type { Action, InlineResult, InlineSide, RegisteredCommand, ViewProps } from "./extension-api";
import { builtInExtensions } from "./extensions";
import { LAUNCH_EVENT, loadFlowKeyExtensions } from "./extensions/flowkey";
import { expandSnippet } from "./extensions/snippets";
import { FlowKeyDialog } from "./flowkey/Dialog";
import { WINDOW_EVENT, type WindowSignal } from "./flowkey/FlowKeyView";
import { loadCommands } from "./registry";
import { search, type Frecency, type Section } from "./search";
import { setLauncherVisible } from "./toast";
import { commandPrefs, isEnabled, useSettings } from "./settings";
import { useAvailableUpdate } from "./updater";

function CardSide({ side, isInput }: { side: InlineSide; isInput?: boolean }) {
  return (
    <div className="flex min-w-0 flex-col items-center px-4 pt-1 pb-[13px]">
      <div className="flex min-h-0 w-full flex-1 items-center justify-center">
        {side.swatch ? (
          <span aria-label={side.label} style={{ background: side.swatch }} className="h-[72px] w-[98px] rounded-[6px] shadow-[0_0_0_2px_var(--color-fg)]" />
        ) : (
          <span className="max-w-full truncate text-[24px] font-semibold text-fg">
            {isInput
              ? side.text.split(/(\s+(?:in|to|as|=)\s+)/i).map((part, i) => (
                  <span key={i} className={i % 2 ? "whitespace-pre text-fg-40" : ""}>
                    {part}
                  </span>
                ))
              : side.text}
          </span>
        )}
      </div>
      <span className="max-w-full truncate rounded-[4px] bg-control px-1.5 py-px text-12 font-medium text-fg">{side.label}</span>
    </div>
  );
}

function InlineCard({ result, selected, onSelect, onRun }: { result: InlineResult; selected: boolean; onSelect: () => void; onRun: () => void }) {
  return (
    <div
      id="option-inline"
      data-index={0}
      role="option"
      aria-selected={selected}
      aria-label={`${result.input.text} = ${result.output.text || result.output.label}`}
      onMouseMove={onSelect}
      onClick={onRun}
      className={`relative grid h-[124px] cursor-default grid-cols-2 rounded-row ${selected ? "bg-list-selection" : "bg-list-hover"}`}
    >
      <CardSide side={result.input} isInput />
      <CardSide side={result.output} />
      <div
        aria-hidden="true"
        className="absolute inset-y-0 left-1/2 w-px bg-separator [mask-image:linear-gradient(to_bottom,black_calc(50%_-_12px),transparent_calc(50%_-_12px),transparent_calc(50%_+_12px),black_calc(50%_+_12px))]"
      />
      <div className="absolute top-1/2 left-1/2 flex -translate-x-1/2 -translate-y-1/2 flex-col items-center">
        <span aria-hidden="true" className="text-[15px] leading-none text-fg">
          →
        </span>
        {result.note && <span className="absolute top-full mt-1 text-11 whitespace-nowrap text-fg-60">{result.note}</span>}
      </div>
    </div>
  );
}

const EXTENSION_ICONS = new Map(builtInExtensions.map((e) => [e.id, e.icon]));

export default function App() {
  const [commands, setCommands] = useState<RegisteredCommand[]>([]);
  const [frecency, setFrecency] = useState<Frecency>({});
  const [query, setQuery] = useState("");
  const [selected, setSelected] = useState(0);
  const [loading, setLoading] = useState(0);
  const [favorites, setFavorites] = useState<string[]>([]);
  const [view, setView] = useState<{ key: string; Component: ComponentType<ViewProps> } | null>(null);
  const [inline, setInline] = useState<{ query: string; result: InlineResult } | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const keepSelected = useRef<string | null>(null);
  const [settings] = useSettings();
  const host = useActionHost(() => inputRef.current?.focus());
  const update = useAvailableUpdate();

  const refresh = useCallback(async () => {
    setLoading((n) => n + 1);
    try {
      const [loaded, scores, favs] = await Promise.all([
        loadFlowKeyExtensions(true).then((flowkey) => loadCommands([...builtInExtensions, ...flowkey])),
        invoke<Frecency>("frecency_all").catch(() => ({}) as Frecency),
        invoke<string[]>("favorites_get").catch(() => []),
      ]);
      setCommands(loaded);
      setFrecency(scores);
      setFavorites(favs);
    } finally {
      setLoading((n) => n - 1);
    }
  }, []);

  useEffect(() => {
    refresh();
    const unlisteners = [
      listen("launcher-hidden", () => {
        setLauncherVisible(false);
        setQuery("");
        setSelected(0);
        setView(null);
        host.closePanel();
        refresh();
      }),
      listen("launcher-shown", () => {
        setLauncherVisible(true);
        inputRef.current?.focus();
      }),
      listen<string>("snippet-expand", (event) => expandSnippet(event.payload).catch((e) => console.error("snippet expansion failed", e))),
    ];
    return () => {
      unlisteners.forEach((p) => p.then((off) => off()));
    };
  }, [refresh]);

  useEffect(() => {
    const text = query;
    if (!text.trim()) {
      setInline(null);
      return;
    }
    let cancelled = false;
    const sources = builtInExtensions.filter((e) => e.inline && (!settings || isEnabled(settings, e.id, e.id)));
    Promise.all(sources.map((e) => e.inline!(text).catch(() => null))).then((results) => {
      if (cancelled) return;
      const result = results.find((r): r is InlineResult => !!r);
      setInline(result ? { query: text, result } : null);
    });
    return () => {
      cancelled = true;
    };
  }, [query, settings]);

  const enabled = useMemo(() => (settings ? commands.filter((c) => isEnabled(settings, c.key, c.extensionId)) : commands), [commands, settings]);
  const aliases = useMemo(
    () => Object.fromEntries(enabled.map((c) => [c.key, settings ? commandPrefs(settings, c.key).alias : ""])),
    [enabled, settings],
  );
  const sections = useMemo<Section[]>(() => {
    const results = search(enabled, query, frecency, favorites, aliases);
    const text = query.trim();
    if (!text) return results;
    const fallbacks = enabled
      .filter((c) => c.fallback)
      .map((c): RegisteredCommand => ({ ...c, key: `fallback:${c.key}`, actions: undefined, view: undefined, run: () => c.fallback!(text) }));
    return fallbacks.length ? [...results, { title: `Use “${text}” with...`, items: fallbacks }] : results;
  }, [enabled, query, frecency, favorites, aliases]);

  const inlineResult = inline && inline.query === query ? inline.result : null;
  const flat = useMemo(() => sections.flatMap((s) => s.items), [sections]);
  const showUpdate = !!update && !query.trim();
  const lead = inlineResult || showUpdate ? 1 : 0;
  const count = flat.length + lead;
  const active = Math.max(0, Math.min(selected, count - 1));
  const isInline = !!inlineResult && active === 0;
  const isUpdate = showUpdate && active === 0;
  const current = isInline || isUpdate ? undefined : flat[active - lead];

  function openView(command: RegisteredCommand) {
    if (!command.view) return;
    host.closePanel();
    setView({ key: command.key, Component: command.view });
    visit(command);
  }

  const runByKey = useRef<(key: string) => void>(() => {});
  runByKey.current = (key) => {
    const command = enabled.find((c) => c.key === key);
    if (!command) return;
    if (command.view) {
      openView(command);
      invoke("show_launcher");
    } else runAction(command, { id: "primary", title: command.primaryActionTitle, confirm: command.confirm, run: () => command.run?.() });
  };

  useEffect(() => {
    const unlisten = listen<string>("run-command", (event) => runByKey.current(event.payload));
    const onLaunch = (event: Event) => runByKey.current((event as CustomEvent<{ key: string }>).detail.key);
    const onSignal = (event: Event) => {
      const { command } = (event as CustomEvent<WindowSignal>).detail;
      if (command === "popToRoot") setView(null);
      else if (command === "clearSearchBar") setQuery("");
    };
    window.addEventListener(LAUNCH_EVENT, onLaunch);
    window.addEventListener(WINDOW_EVENT, onSignal);
    return () => {
      unlisten.then((off) => off());
      window.removeEventListener(LAUNCH_EVENT, onLaunch);
      window.removeEventListener(WINDOW_EVENT, onSignal);
    };
  }, []);

  useEffect(() => {
    const key = keepSelected.current;
    keepSelected.current = null;
    const index = key ? flat.findIndex((c) => c.key === key) : -1;
    if (index >= 0) setSelected(index + lead);
  }, [flat]);

  useEffect(() => {
    listRef.current?.querySelector(`[data-index="${active}"]`)?.scrollIntoView({ block: "nearest" });
  }, [active, sections]);

  const installUpdate: Action = { id: "install-update", title: "Install Update", shortcut: { key: "Enter" }, keepOpen: true, run: () => update!.downloadAndInstall() };
  const groups: Action[][] = isUpdate
    ? [[installUpdate]]
    : isInline
    ? inlineResult!.actions
    : current?.key.startsWith("fallback:")
      ? [[{ id: "primary", title: current.primaryActionTitle, shortcut: { key: "Enter" }, run: () => current.run?.() }]]
      : current
        ? actionGroups(current, {
            favorites,
            frecency,
            setFavorites: (keys) => {
              keepSelected.current = current.key;
              setFavorites(keys);
              invoke("favorites_set", { keys }).catch((e) => console.error("favorites_set failed", e));
            },
            resetRanking: (key) => {
              keepSelected.current = key;
              setFrecency(({ [key]: _, ...rest }) => rest);
              invoke("frecency_reset", { key }).catch((e) => console.error("frecency_reset failed", e));
            },
          })
        : [];
  const primaryTitle = isUpdate ? installUpdate.title : isInline ? inlineResult!.primaryActionTitle : current?.primaryActionTitle;
  const panelTitle = isUpdate ? "New Update Available" : isInline ? inlineResult!.output.text || inlineResult!.output.label : (current?.title ?? "");

  function visit(command: RegisteredCommand) {
    if (command.key.startsWith("fallback:")) return;
    invoke<number>("frecency_visit", { key: command.key })
      .then((score) => setFrecency((f) => ({ ...f, [command.key]: score })))
      .catch((e) => console.error("frecency_visit failed", e));
  }

  function runAction(command: RegisteredCommand | undefined, action: Action) {
    if (command && action.id === "primary" && command.view) {
      openView(command);
      return;
    }
    const title = action.id === "primary" && command ? `Could not open ${command.title}` : `${action.title.replace(/…$/, "")} failed`;
    host.run(action, { failTitle: title, onSuccess: () => command && !action.keepOpen && visit(command) });
  }

  function runSelected(action: Action) {
    runAction(isInline ? undefined : current, action);
  }

  function onKeyDown(event: React.KeyboardEvent<HTMLInputElement>) {
    if (matchesShortcut(event, { key: "k", ctrl: true })) {
      event.preventDefault();
      if (groups.length) host.openPanel(panelTitle, groups);
      return;
    }
    const action = groups.flat().find((a) => a.shortcut && matchesShortcut(event, a.shortcut));
    if (event.key === "Enter" && !event.ctrlKey && !event.altKey && !event.shiftKey && groups[0]?.[0]) {
      event.preventDefault();
      runSelected(groups[0][0]);
    } else if (action) {
      event.preventDefault();
      runSelected(action);
    } else if (event.key === "ArrowDown") {
      event.preventDefault();
      setSelected(Math.min(active + 1, count - 1));
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      setSelected(Math.max(active - 1, 0));
    } else if (event.key === "Escape") {
      event.preventDefault();
      if (query && settings?.escapeKeyBehavior !== "close-and-pop-to-root") {
        setQuery("");
        setSelected(0);
      } else {
        invoke("hide_launcher", { restoreFocus: true });
      }
    }
  }

  const keepFocus = (e: React.MouseEvent) => {
    if (!(e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement)) e.preventDefault();
  };

  if (view) {
    const pop = () => {
      setView(null);
      setQuery("");
      setSelected(0);
      refresh();
      window.setTimeout(() => inputRef.current?.focus());
    };
    return (
      <main onMouseDown={keepFocus} className="relative flex h-full flex-col text-fg">
        <h1 className="sr-only">Openray</h1>
        <view.Component key={view.key} pop={pop} />
        <FlowKeyDialog />
      </main>
    );
  }

  let index = lead ? 0 : -1;

  return (
    <main onMouseDown={keepFocus} className="relative flex h-full flex-col text-fg">
      <h1 className="sr-only">Openray</h1>
      <header className="relative flex h-header shrink-0 items-center gap-3 border-b border-separator px-search-inset">
        <Logo />
        <input
          ref={inputRef}
          autoFocus
          spellCheck={false}
          value={query}
          onChange={(e) => {
            setQuery(e.target.value);
            setSelected(0);
          }}
          onKeyDown={onKeyDown}
          placeholder="Search for apps or commands..."
          aria-label="Search for apps or commands"
          role="combobox"
          aria-expanded="true"
          aria-controls="results"
          aria-activedescendant={isUpdate ? "option-update" : isInline ? "option-inline" : current ? `option-${active}` : undefined}
          className="h-full min-w-0 flex-1 bg-transparent text-18 text-fg caret-fg outline-none placeholder:text-fg-40"
        />
        <Loader active={loading > 0 || host.busy} />
      </header>

      <div ref={listRef} id="results" role="listbox" aria-label="Results" aria-busy={loading > 0} className="min-h-0 flex-1 overflow-y-auto px-inset pb-inset">
        {count === 0 && <p className="flex h-full items-center justify-center text-13 text-fg-60">No results for “{query.trim()}”</p>}
        {showUpdate && (
          <div
            id="option-update"
            data-index={0}
            role="option"
            aria-selected={isUpdate}
            onMouseMove={() => active !== 0 && setSelected(0)}
            onClick={() => runSelected(installUpdate)}
            className={`mt-2 flex h-[56px] cursor-default items-center gap-3 rounded-row px-2 text-13 ${isUpdate ? "bg-list-selection" : "bg-list-hover"}`}
          >
            <GlyphTile spec={{ glyph: RefreshCw, tone: "red" }} />
            <span className="truncate font-medium">New Update Available</span>
          </div>
        )}
        {inlineResult && (
          <div role="group" aria-label={inlineResult.section}>
            <div className="flex h-8 items-end px-2 pb-[5px] text-11 font-semibold text-fg-60">{inlineResult.section}</div>
            <InlineCard
              result={inlineResult}
              selected={isInline}
              onSelect={() => active !== 0 && setSelected(0)}
              onRun={() => inlineResult.actions[0]?.[0] && runSelected(inlineResult.actions[0][0])}
            />
          </div>
        )}
        {sections.map((section) => (
          <div key={section.title} role="group" aria-label={section.title}>
            <div className="flex h-8 items-end px-2 pb-[5px] text-11 font-semibold text-fg-60">{section.title}</div>
            {section.items.map((command) => {
              index += 1;
              const i = index;
              const isSelected = i === active;
              return (
                <div
                  key={command.key}
                  id={`option-${i}`}
                  data-index={i}
                  role="option"
                  aria-selected={isSelected}
                  onMouseMove={() => i !== selected && setSelected(i)}
                  onClick={() => runAction(command, { id: "primary", title: command.primaryActionTitle, confirm: command.confirm, run: () => command.run?.() })}
                  className={`flex h-row cursor-default items-center gap-3 rounded-row px-2 text-13 ${isSelected ? "bg-list-selection" : ""}`}
                >
                  <CommandIcon title={command.title} icon={command.icon} fallback={EXTENSION_ICONS.get(command.extensionId)} />
                  <span className="truncate font-medium">{command.title}</span>
                  {command.subtitle && <span className="truncate text-fg-60">{command.subtitle}</span>}
                  <span className="ml-auto shrink-0 text-fg-60">{command.kind}</span>
                </div>
              );
            })}
          </div>
        ))}
      </div>

      <Footer
        left={
          <button type="button" tabIndex={-1} aria-label="Open Settings" title="Open Settings" onClick={() => invoke("open_settings")} className="-m-1 rounded-[6px] p-1 hover:bg-list-hover">
            <Logo />
          </button>
        }
        primaryTitle={primaryTitle}
        panelOpen={host.panelOpen}
        showActions={groups.flat().length > 1}
        onTogglePanel={() => (host.panelOpen ? host.closePanel() : groups.length && host.openPanel(panelTitle, groups))}
      />
      {host.render(runSelected)}
      <FlowKeyDialog />
      <div aria-live="polite" className="sr-only">
        {count} {count === 1 ? "result" : "results"}
      </div>
    </main>
  );
}
