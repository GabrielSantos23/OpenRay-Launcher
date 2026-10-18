import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { ActionPanel, Keys } from "./ActionPanel";
import { actionGroups, matchesShortcut } from "./actions";
import type { Action, RegisteredCommand } from "./extension-api";
import { builtInExtensions } from "./extensions";
import { loadCommands } from "./registry";
import { search, type Frecency } from "./search";
import { setLauncherVisible, showToast, useToast } from "./toast";
import { FooterToast } from "./FooterToast";
import { commandPrefs, isEnabled, useSettings } from "./settings";

function Logo() {
  return (
    <svg viewBox="0 0 24 24" className="size-[22px] shrink-0 text-fg-60" aria-hidden="true">
      <circle cx="12" cy="12" r="8" fill="none" stroke="currentColor" strokeWidth="2.5" />
      <path d="M12 12 L21 3" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" />
    </svg>
  );
}

function ItemIcon({ title, icon }: { title: string; icon?: string }) {
  const [failed, setFailed] = useState(false);
  if (icon && !failed) {
    return <img src={icon} alt="" loading="lazy" draggable={false} onError={() => setFailed(true)} className="size-[22px] shrink-0" />;
  }
  return (
    <span
      aria-hidden="true"
      className="flex size-[22px] shrink-0 items-center justify-center rounded-[5px] bg-key-bg text-11 font-semibold text-fg-60"
    >
      {title.trim().charAt(0).toUpperCase()}
    </span>
  );
}

function Loader({ active }: { active: boolean }) {
  const [shown, setShown] = useState(false);
  useEffect(() => {
    if (!active) {
      setShown(false);
      return;
    }
    const timer = window.setTimeout(() => setShown(true), 300);
    return () => window.clearTimeout(timer);
  }, [active]);
  return (
    <div
      aria-hidden="true"
      className={`loader-gradient pointer-events-none absolute inset-x-0 -bottom-px h-px animate-loader transition-opacity duration-300 ${shown ? "opacity-100" : "opacity-0 [animation-play-state:paused]"}`}
    />
  );
}

function fail(title: string, error: unknown) {
  showToast({ variant: "failure", title, message: String(error) });
}

export default function App() {
  const [commands, setCommands] = useState<RegisteredCommand[]>([]);
  const [frecency, setFrecency] = useState<Frecency>({});
  const [query, setQuery] = useState("");
  const [selected, setSelected] = useState(0);
  const [loading, setLoading] = useState(0);
  const [favorites, setFavorites] = useState<string[]>([]);
  const [panelOpen, setPanelOpen] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const keepSelected = useRef<string | null>(null);
  const toast = useToast();
  const [settings] = useSettings();

  const refresh = useCallback(async () => {
    setLoading((n) => n + 1);
    try {
      const [loaded, scores, favs] = await Promise.all([
        loadCommands(builtInExtensions),
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
        setPanelOpen(false);
        refresh();
      }),
      listen("launcher-shown", () => {
        setLauncherVisible(true);
        inputRef.current?.focus();
      }),
    ];
    return () => {
      unlisteners.forEach((p) => p.then((off) => off()));
    };
  }, [refresh]);

  const enabled = useMemo(() => (settings ? commands.filter((c) => isEnabled(settings, c.key, c.extensionId)) : commands), [commands, settings]);
  const aliases = useMemo(
    () => Object.fromEntries(enabled.map((c) => [c.key, settings ? commandPrefs(settings, c.key).alias : ""])),
    [enabled, settings],
  );
  const sections = useMemo(() => search(enabled, query, frecency, favorites, aliases), [enabled, query, frecency, favorites, aliases]);
  const runByKey = useRef<(key: string) => void>(() => {});
  runByKey.current = (key) => {
    const command = enabled.find((c) => c.key === key);
    if (command) runAction(command, { id: "primary", title: command.primaryActionTitle, run: command.run });
  };

  useEffect(() => {
    const unlisten = listen<string>("run-command", (event) => runByKey.current(event.payload));
    return () => {
      unlisten.then((off) => off());
    };
  }, []);
  const flat = useMemo(() => sections.flatMap((s) => s.items), [sections]);
  const current = flat[Math.min(selected, flat.length - 1)];

  useEffect(() => {
    const key = keepSelected.current;
    keepSelected.current = null;
    const index = key ? flat.findIndex((c) => c.key === key) : -1;
    if (index >= 0) setSelected(index);
  }, [flat]);

  useEffect(() => {
    listRef.current?.querySelector(`[data-index="${selected}"]`)?.scrollIntoView({ block: "nearest" });
  }, [selected, sections]);

  const groups = current
    ? actionGroups(current, {
        favorites,
        frecency,
        setFavorites: (keys) => {
          keepSelected.current = current.key;
          setFavorites(keys);
          invoke("favorites_set", { keys }).catch((e) => fail("Could not save favorites", e));
        },
        resetRanking: (key) => {
          keepSelected.current = key;
          setFrecency(({ [key]: _, ...rest }) => rest);
          invoke("frecency_reset", { key }).catch((e) => fail("Could not reset ranking", e));
        },
      })
    : [];

  function closePanel() {
    setPanelOpen(false);
    inputRef.current?.focus();
  }

  async function runAction(command: RegisteredCommand, action: Action) {
    closePanel();
    setLoading((n) => n + 1);
    try {
      await action.run();
    } catch (e) {
      fail(action.id === "primary" ? `Could not open ${command.title}` : `${action.title} failed`, e);
      return;
    } finally {
      setLoading((n) => n - 1);
    }
    if (action.keepOpen) return;
    invoke<number>("frecency_visit", { key: command.key })
      .then((score) => setFrecency((f) => ({ ...f, [command.key]: score })))
      .catch((e) => console.error("frecency_visit failed", e));
    await invoke("hide_launcher", { restoreFocus: true });
  }

  function onKeyDown(event: React.KeyboardEvent<HTMLInputElement>) {
    if (matchesShortcut(event, { key: "k", ctrl: true })) {
      event.preventDefault();
      if (current) setPanelOpen(true);
      return;
    }
    const action = groups.flat().find((a) => a.shortcut && matchesShortcut(event, a.shortcut));
    if (current && action) {
      event.preventDefault();
      runAction(current, action);
    } else if (event.key === "ArrowDown") {
      event.preventDefault();
      setSelected((i) => Math.min(i + 1, flat.length - 1));
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      setSelected((i) => Math.max(i - 1, 0));
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

  let index = -1;

  return (
    <main
      onMouseDown={(e) => {
        if (!(e.target instanceof HTMLInputElement)) e.preventDefault();
      }}
      className="relative flex h-full flex-col text-fg"
    >
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
          aria-activedescendant={current ? `option-${Math.min(selected, flat.length - 1)}` : undefined}
          className="h-full min-w-0 flex-1 bg-transparent text-18 text-fg caret-fg outline-none placeholder:text-fg-40"
        />
        <Loader active={loading > 0} />
      </header>

      <div ref={listRef} id="results" role="listbox" aria-label="Results" aria-busy={loading > 0} className="min-h-0 flex-1 overflow-y-auto px-inset pb-inset">
        {sections.length === 0 && (
          <p className="flex h-full items-center justify-center text-13 text-fg-60">No results for “{query.trim()}”</p>
        )}
        {sections.map((section) => (
          <div key={section.title} role="group" aria-label={section.title}>
            <div className="flex h-8 items-end px-2 pb-[5px] text-11 font-semibold text-fg-60">{section.title}</div>
            {section.items.map((command) => {
              index += 1;
              const i = index;
              const isSelected = i === Math.min(selected, flat.length - 1);
              return (
                <div
                  key={command.key}
                  id={`option-${i}`}
                  data-index={i}
                  role="option"
                  aria-selected={isSelected}
                  onMouseMove={() => i !== selected && setSelected(i)}
                  onClick={() => runAction(command, { id: "primary", title: command.primaryActionTitle, run: command.run })}
                  className={`flex h-row cursor-default items-center gap-3 rounded-row px-2 text-13 ${isSelected ? "bg-list-selection" : ""}`}
                >
                  <ItemIcon title={command.title} icon={command.icon} />
                  <span className="truncate font-medium">{command.title}</span>
                  {command.subtitle && <span className="truncate text-fg-60">{command.subtitle}</span>}
                  <span className="ml-auto shrink-0 text-fg-60">{command.kind}</span>
                </div>
              );
            })}
          </div>
        ))}
      </div>

      <footer className="relative isolate flex h-footer shrink-0 items-center gap-2 overflow-hidden pt-px pr-[9px] pb-px pl-3 text-13 before:absolute before:inset-x-0 before:top-0 before:-z-20 before:h-px before:bg-separator">
        {toast ? (
          <FooterToast />
        ) : (
          <button type="button" tabIndex={-1} aria-label="Open Settings" title="Open Settings" onClick={() => invoke("open_settings")} className="-m-1 rounded-[6px] p-1 hover:bg-list-hover">
            <Logo />
          </button>
        )}
        {current && (
          <>
            <span className="ml-auto flex shrink-0 items-center gap-2 font-medium">
              {current.primaryActionTitle}
              <Keys labels={["↵"]} />
            </span>
            <span aria-hidden="true" className={`-mr-0.5 ml-1 h-[11px] w-px ${panelOpen ? "" : "bg-footer-divider"}`} />
            <button
              type="button"
              tabIndex={-1}
              aria-expanded={panelOpen}
              onMouseDown={(e) => e.preventDefault()}
              onClick={() => (panelOpen ? closePanel() : setPanelOpen(true))}
              className={`flex h-[26px] shrink-0 cursor-default items-center gap-2 rounded-[6px] pr-[3px] pl-2 font-medium text-fg-60 ${panelOpen ? "bg-list-selection" : ""}`}
            >
              Actions
              <Keys labels={["Ctrl", "K"]} />
            </button>
          </>
        )}
      </footer>
      {panelOpen && current && (
        <ActionPanel
          key={current.key}
          title={current.title}
          groups={groups}
          onRun={(action) => runAction(current, action)}
          onClose={closePanel}
        />
      )}
      <div aria-live="polite" className="sr-only">
        {flat.length} {flat.length === 1 ? "result" : "results"}
      </div>
    </main>
  );
}
