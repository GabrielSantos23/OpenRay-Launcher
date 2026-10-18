import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { RegisteredCommand } from "./extension-api";
import { builtInExtensions } from "./extensions";
import { loadCommands } from "./registry";
import { search, type Frecency } from "./search";

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

export default function App() {
  const [commands, setCommands] = useState<RegisteredCommand[]>([]);
  const [frecency, setFrecency] = useState<Frecency>({});
  const [query, setQuery] = useState("");
  const [selected, setSelected] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  const refresh = useCallback(async () => {
    const [loaded, scores] = await Promise.all([
      loadCommands(builtInExtensions),
      invoke<Frecency>("frecency_all").catch(() => ({}) as Frecency),
    ]);
    setCommands(loaded);
    setFrecency(scores);
  }, []);

  useEffect(() => {
    refresh();
    const unlisteners = [
      listen("launcher-hidden", () => {
        setQuery("");
        setSelected(0);
        setError(null);
        refresh();
      }),
      listen("launcher-shown", () => inputRef.current?.focus()),
    ];
    return () => {
      unlisteners.forEach((p) => p.then((off) => off()));
    };
  }, [refresh]);

  const sections = useMemo(() => search(commands, query, frecency), [commands, query, frecency]);
  const flat = useMemo(() => sections.flatMap((s) => s.items), [sections]);
  const current = flat[Math.min(selected, flat.length - 1)];

  useEffect(() => {
    listRef.current?.querySelector(`[data-index="${selected}"]`)?.scrollIntoView({ block: "nearest" });
  }, [selected, sections]);

  async function run(command: RegisteredCommand) {
    try {
      await command.run();
    } catch (e) {
      setError(`Could not open ${command.title}: ${String(e)}`);
      return;
    }
    invoke<number>("frecency_visit", { key: command.key })
      .then((score) => setFrecency((f) => ({ ...f, [command.key]: score })))
      .catch((e) => console.error("frecency_visit failed", e));
    await invoke("hide_launcher", { restoreFocus: true });
  }

  function onKeyDown(event: React.KeyboardEvent<HTMLInputElement>) {
    if (event.key === "ArrowDown") {
      event.preventDefault();
      setSelected((i) => Math.min(i + 1, flat.length - 1));
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      setSelected((i) => Math.max(i - 1, 0));
    } else if (event.key === "Enter") {
      event.preventDefault();
      if (current) run(current);
    } else if (event.key === "Escape") {
      event.preventDefault();
      invoke("hide_launcher", { restoreFocus: true });
    }
  }

  let index = -1;

  return (
    <main className="flex h-full flex-col bg-bg text-fg">
      <h1 className="sr-only">Openray</h1>
      <header className="flex h-header shrink-0 items-center gap-3 border-b border-separator px-search-inset">
        <Logo />
        <input
          ref={inputRef}
          autoFocus
          spellCheck={false}
          value={query}
          onChange={(e) => {
            setQuery(e.target.value);
            setSelected(0);
            setError(null);
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
      </header>

      <div ref={listRef} id="results" role="listbox" aria-label="Results" className="min-h-0 flex-1 overflow-y-auto px-inset pb-inset">
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
                  onClick={() => run(command)}
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

      <footer className="flex h-footer shrink-0 items-center gap-2 border-t border-separator px-3 text-13">
        {error ? (
          <span role="alert" className="truncate text-fg-60">
            {error}
          </span>
        ) : (
          <Logo />
        )}
        {current && (
          <span className="ml-auto flex shrink-0 items-center gap-2 font-medium">
            {current.primaryActionTitle}
            <kbd className="flex h-5 min-w-5 items-center justify-center rounded-[4px] bg-key-bg px-1 font-sans text-11 text-fg-60">↵</kbd>
          </span>
        )}
      </footer>
      <div aria-live="polite" className="sr-only">
        {flat.length} {flat.length === 1 ? "result" : "results"}
      </div>
    </main>
  );
}
