import { invoke } from "@tauri-apps/api/core";
import { BookOpen, BookText, Clipboard, ClipboardPaste, ExternalLink, PanelRight, RotateCw, Trash2 } from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { fail, Footer, Loader, useActionHost } from "../ActionHost";
import { matchesShortcut } from "../actions";
import { definitionText, guessUrl, parseEntries, parseSuggestions, remember, suggestionUrl, type Entry, type Suggestion } from "../dictionary";
import { defineExtension, type Action, type IconSpec, type ViewProps } from "../extension-api";
import { GlyphTile } from "../icons";
import { BackButton, ListView, type ListSection } from "../ListView";
import { copyAndClose, frontmostApp, pasteTitle, type AppInfo } from "../output";
import { showToast } from "../toast";

const ICON: IconSpec = { glyph: BookOpen, tone: "orange" };
const STORE = "dictionary";
const LIMIT = 8;
const DEBOUNCE_MS = 200;
const RETRY_MS = 600;

async function getJson(url: string, attempts = 3): Promise<unknown> {
  try {
    const response = await fetch(url);
    if (response.status === 404) return [];
    if (!response.ok) throw new Error(`The dictionary service returned an error (${response.status}).`);
    return await response.json();
  } catch (e) {
    if (attempts > 1) {
      await new Promise((r) => window.setTimeout(r, RETRY_MS));
      return getJson(url, attempts - 1);
    }
    throw e instanceof TypeError ? new Error("The dictionary service didn't respond. Check your connection and try again.") : e;
  }
}

async function suggest(text: string): Promise<Suggestion[]> {
  const results = parseSuggestions(text, await getJson(suggestionUrl(text, LIMIT)), false);
  return results.length ? results : parseSuggestions(text, await getJson(guessUrl(text, LIMIT)), true);
}

function footer(title: string) {
  return (
    <span className="flex min-w-0 items-center gap-2 text-fg-60">
      <GlyphTile spec={ICON} small />
      <span className="truncate">{title}</span>
    </span>
  );
}

function Definition({ word, app, onBack }: { word: string; app: AppInfo | null; onBack: () => void }) {
  const [entries, setEntries] = useState<Entry[] | null>(null);
  const [error, setError] = useState<string>();
  const [attempt, setAttempt] = useState(0);
  const rootRef = useRef<HTMLDivElement>(null);
  const host = useActionHost(() => rootRef.current?.focus());

  useEffect(() => {
    rootRef.current?.focus();
  }, []);

  useEffect(() => {
    setEntries(null);
    setError(undefined);
    invoke<unknown>("dictionary_define", { word })
      .then((body) => setEntries(parseEntries(body)))
      .catch((e) => {
        setEntries([]);
        setError(String(e));
      });
  }, [word, attempt]);

  const source = entries?.find((e) => e.sourceUrl)?.sourceUrl;
  const groups = useMemo<Action[][]>(
    () => [
      [
        { id: "copy-word", title: "Copy Word to Clipboard", icon: Clipboard, shortcut: { key: "c", ctrl: true, shift: true }, run: () => copyAndClose(word, "Copied word to clipboard") },
        { id: "paste", title: pasteTitle(app), icon: ClipboardPaste, shortcut: { key: "Enter", ctrl: true }, run: () => invoke<void>("paste_text", { text: word }) },
        ...(entries?.length ? [{ id: "copy-definition", title: "Copy Definition", icon: BookText, shortcut: { key: "c", ctrl: true, alt: true }, run: () => copyAndClose(definitionText(entries), "Copied definition") }] : []),
      ],
      ...(error ? [[{ id: "retry", title: "Try Again", icon: RotateCw, shortcut: { key: "r", ctrl: true }, keepOpen: true, run: () => setAttempt((n) => n + 1) }]] : []),
      ...(source ? [[{ id: "open-source", title: "Open in Browser", icon: ExternalLink, shortcut: { key: "o", ctrl: true }, run: () => invoke<void>("open_url", { url: source, appPath: null }) }]] : []),
    ],
    [word, app, entries, source, error],
  );
  const primary = groups[0][0];

  function runAction(action: Action) {
    host.run(action, { failTitle: `${action.title} failed` });
  }

  function onKeyDown(event: React.KeyboardEvent) {
    if (event.defaultPrevented || host.panelOpen) return;
    if (matchesShortcut(event, { key: "k", ctrl: true })) {
      event.preventDefault();
      host.openPanel(word, groups);
    } else if (event.key === "Enter" && !event.ctrlKey && !event.altKey && !event.shiftKey) {
      event.preventDefault();
      runAction(primary);
    } else if (event.key === "Escape" || event.key === "Backspace") {
      event.preventDefault();
      onBack();
    } else {
      const action = groups.flat().find((a) => a.shortcut && matchesShortcut(event, a.shortcut));
      if (action) {
        event.preventDefault();
        runAction(action);
      }
    }
  }

  return (
    <div ref={rootRef} tabIndex={-1} onKeyDown={onKeyDown} className="flex min-h-0 flex-1 flex-col outline-none">
      <header className="flex h-header shrink-0 items-center gap-3 border-b border-separator px-search-inset">
        <BackButton onBack={onBack} />
        <h2 className="flex-1 truncate text-13 font-medium text-fg-60">{word}</h2>
        <Loader active={entries === null || host.busy} />
      </header>
      <div className="min-h-0 flex-1 overflow-y-auto px-6 py-4">
        {entries?.length === 0 && (
          <div className="flex h-full flex-col items-center justify-center gap-1 text-center text-13">
            <p className="font-medium text-fg">{error ? "Couldn't load the definition" : "No Definitions Found"}</p>
            <p className="max-w-[420px] text-fg-60">{error ?? `The dictionary has no entry for "${word}". Copy the word or try another spelling.`}</p>
          </div>
        )}
        {entries?.map((entry, i) => (
          <article key={i} className="flex flex-col gap-4 pb-6">
            <header className="flex items-baseline gap-3">
              <h1 className="text-[24px] font-semibold text-fg">{entry.word}</h1>
              {entry.phonetic && <span className="text-15 text-fg-60">{entry.phonetic}</span>}
            </header>
            {entry.meanings.map((meaning, j) => (
              <section key={j} className="flex flex-col gap-2">
                <h2 className="text-12 font-semibold text-fg-60 italic">{meaning.partOfSpeech}</h2>
                <ol className="flex list-decimal flex-col gap-2 pl-5 text-13 text-fg marker:text-fg-40">
                  {meaning.senses.map((sense, k) => (
                    <li key={k}>
                      {sense.definition}
                      {sense.example && <p className="mt-0.5 text-fg-60 italic">“{sense.example}”</p>}
                    </li>
                  ))}
                </ol>
                {meaning.synonyms.length > 0 && (
                  <p className="text-12 text-fg-60">
                    <span className="font-medium">Synonyms: </span>
                    {meaning.synonyms.slice(0, 8).join(", ")}
                  </p>
                )}
              </section>
            ))}
          </article>
        ))}
      </div>
      <Footer
        left={footer(word)}
        primaryTitle={primary.title}
        panelOpen={host.panelOpen}
        onTogglePanel={() => (host.panelOpen ? host.closePanel() : host.openPanel(word, groups))}
      />
      {host.render(runAction)}
    </div>
  );
}

function DefineWord({ pop }: ViewProps) {
  const [text, setText] = useState("");
  const [results, setResults] = useState<Suggestion[]>([]);
  const [recent, setRecent] = useState<Suggestion[]>([]);
  const [loading, setLoading] = useState(false);
  const [detail, setDetail] = useState<string | null>(null);
  const [app, setApp] = useState<AppInfo | null>(null);
  const seq = useRef(0);

  useEffect(() => {
    invoke<{ recent?: Suggestion[] } | null>("data_get", { name: STORE })
      .then((stored) => setRecent(stored?.recent ?? []))
      .catch(() => {});
    frontmostApp().then(setApp);
  }, []);

  const saveRecent = useCallback((next: Suggestion[]) => {
    setRecent(next);
    invoke("data_set", { name: STORE, value: { recent: next } }).catch((e) => console.error("could not save recent words", e));
  }, []);

  useEffect(() => {
    const id = ++seq.current;
    if (!text.trim()) {
      setResults([]);
      setLoading(false);
      return;
    }
    setLoading(true);
    const timer = window.setTimeout(() => {
      suggest(text)
        .then((r) => id === seq.current && setResults(r))
        .catch((e) => id === seq.current && fail("Failed to get dictionary suggestions", e))
        .finally(() => id === seq.current && setLoading(false));
    }, DEBOUNCE_MS);
    return () => window.clearTimeout(timer);
  }, [text]);

  const searching = text.trim().length > 0;

  const sections = useMemo<ListSection[]>(() => {
    const item = (s: Suggestion, prefix: string) => ({
      id: `${prefix}:${s.word}`,
      title: s.word,
      subtitle: s.definition || undefined,
      actions: [
        [
          {
            id: "details",
            title: "Show Details",
            icon: PanelRight,
            keepOpen: true,
            run: () => {
              saveRecent(remember(recent, s));
              setDetail(s.word);
            },
          },
          { id: "paste", title: pasteTitle(app), icon: ClipboardPaste, shortcut: { key: "Enter", ctrl: true, shift: true }, run: () => invoke<void>("paste_text", { text: s.word }) },
          { id: "copy", title: "Copy Word to Clipboard", icon: Clipboard, shortcut: { key: "c", ctrl: true, shift: true }, run: () => copyAndClose(s.word, "Copied word to clipboard") },
        ],
        ...(searching
          ? []
          : [
              [
                {
                  id: "remove",
                  title: "Remove from Recent",
                  icon: Trash2,
                  destructive: true,
                  keepOpen: true,
                  shortcut: { key: "d", ctrl: true },
                  run: () => {
                    saveRecent(recent.filter((w) => w.word !== s.word));
                    showToast({ variant: "success", title: "Removed from recent words" });
                  },
                },
                {
                  id: "clear",
                  title: "Clear All Recent Words",
                  icon: Trash2,
                  destructive: true,
                  keepOpen: true,
                  shortcut: { key: "d", ctrl: true, alt: true },
                  run: () => {
                    saveRecent([]);
                    showToast({ variant: "success", title: "Cleared recent words" });
                  },
                },
              ],
            ]),
      ],
    });
    if (!searching) return recent.length ? [{ title: "Recent Words", items: recent.map((s) => item(s, "recent")) }] : [];
    const found = results.filter((r) => !r.isGuess);
    const guesses = results.filter((r) => r.isGuess);
    return [
      ...(found.length ? [{ title: "Results", items: found.map((s) => item(s, "result")) }] : []),
      ...(guesses.length ? [{ title: "Did you mean?", items: guesses.map((s) => item(s, "guess")) }] : []),
    ];
  }, [searching, recent, results, app, saveRecent]);

  if (detail) return <Definition word={detail} app={app} onBack={() => setDetail(null)} />;

  return (
    <ListView
      placeholder="Search word…"
      sections={sections}
      onBack={pop}
      searchText={text}
      onSearchTextChange={setText}
      filtering={false}
      footerTitle="Define Word"
      footerIcon={<GlyphTile spec={ICON} small />}
      isLoading={loading}
      emptyText={searching ? "No Results Found" : "Type a word to look up in dictionary"}
      emptyDescription={searching ? "Check the spelling or try another word." : "English definitions, pronunciations and examples."}
    />
  );
}

export default defineExtension({
  id: "dictionary",
  title: "Dictionary",
  icon: ICON,
  commands: [
    {
      id: "defineWord",
      title: "Define Word",
      subtitle: "Dictionary",
      keywords: ["dictionary", "define", "lexicon", "thesaurus", "glossary", "spell", "meaning"],
      kind: "Command",
      primaryActionTitle: "Open Command",
      view: DefineWord,
    },
  ],
});
