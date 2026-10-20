import { invoke } from "@tauri-apps/api/core";
import { Clipboard, ClipboardPaste, CopyPlus, Download, FileText, Pencil, Pin, PinOff, Plus, TextQuote, Trash2, Upload } from "lucide-react";
import { useCallback, useEffect, useId, useMemo, useRef, useState } from "react";
import { fail, Footer } from "../ActionHost";
import { matchesShortcut } from "../actions";
import { defineExtension, type Action, type IconSpec, type Item, type ViewProps } from "../extension-api";
import { GlyphTile } from "../icons";
import { BackButton, ListView, Metadata } from "../ListView";
import { copyAndClose, frontmostApp, pasteTitle, type AppInfo } from "../output";
import { exportSnippets, keywordInUse, mergeImport, parseSnippetImport, resolveSnippet, type Snippet } from "../snippets";
import { showToast } from "../toast";

const ICON: IconSpec = { glyph: TextQuote, tone: "blue" };

export async function loadSnippets(): Promise<Snippet[]> {
  const value = await invoke<Snippet[] | null>("data_get", { name: "snippets" });
  return Array.isArray(value) ? value : [];
}

function saveSnippets(list: Snippet[]) {
  return invoke("data_set", { name: "snippets", value: list });
}

async function clipboardAt(offset: number): Promise<string> {
  if (offset === 0) return (await invoke<string | null>("read_clipboard_text").catch(() => null)) ?? "";
  const entries = await invoke<{ kind: string; text?: string }[]>("clipboard_list").catch(() => []);
  return entries.filter((e) => e.kind === "text")[offset]?.text ?? "";
}

async function resolve(snippet: Snippet, all: Snippet[]) {
  return resolveSnippet(snippet.text, { clipboard: clipboardAt, snippets: all });
}

async function markUsed(id: string) {
  const list = await loadSnippets();
  await saveSnippets(list.map((s) => (s.id === id ? { ...s, copiedAt: Date.now(), copyCount: (s.copyCount ?? 0) + 1 } : s)));
}

export async function expandSnippet(id: string) {
  const list = await loadSnippets();
  const snippet = list.find((s) => s.id === id);
  if (!snippet) return;
  const { text, cursorBack } = await resolve(snippet, list);
  await invoke("snippet_type", { text, cursorBack });
}

function SnippetForm({ initial, title, onDone, onBack }: { initial?: Partial<Snippet>; title: string; onDone: () => void; onBack: () => void }) {
  const [text, setText] = useState(initial?.text ?? "");
  const [name, setName] = useState(initial?.name ?? "");
  const [keyword, setKeyword] = useState(initial?.keyword ?? "");
  const [errors, setErrors] = useState<{ text?: string; name?: string; keyword?: string }>({});
  const textRef = useRef<HTMLTextAreaElement>(null);
  const nameRef = useRef<HTMLInputElement>(null);
  const keywordRef = useRef<HTMLInputElement>(null);
  const ids = useId();
  const editing = !!initial?.id;

  async function save() {
    try {
      const list = await loadSnippets();
      const next = {
        text: text ? undefined : "Snippet cannot be empty",
        name: name.trim() ? undefined : "Name cannot be empty",
        keyword: keywordInUse(keyword, list, initial?.id) ? "This keyword is already in use" : undefined,
      };
      setErrors(next);
      if (next.text) return textRef.current?.focus();
      if (next.name) return nameRef.current?.focus();
      if (next.keyword) return keywordRef.current?.focus();
      const now = Date.now();
      const base = editing ? list.find((s) => s.id === initial?.id) : undefined;
      const snippet: Snippet = {
        ...base,
        id: base?.id ?? crypto.randomUUID(),
        name: name.trim(),
        text,
        keyword: keyword.trim() || undefined,
        createdAt: base?.createdAt ?? now,
        modifiedAt: now,
      };
      await saveSnippets(base ? list.map((s) => (s.id === snippet.id ? snippet : s)) : [...list, snippet]);
      showToast({ variant: "success", title: base ? "Snippet updated" : "Created snippet" });
      onDone();
    } catch (e) {
      fail(editing ? "Failed to update snippet" : "Failed to create snippet", e);
    }
  }

  function onKeyDown(event: React.KeyboardEvent) {
    if (matchesShortcut(event, { key: "Enter", ctrl: true })) {
      event.preventDefault();
      save();
    } else if (event.key === "Escape" && !event.defaultPrevented) {
      event.preventDefault();
      onBack();
    }
  }

  const field = "w-full rounded-lg border bg-card px-3 text-13 text-fg caret-fg outline-none placeholder:text-fg-40";
  const border = (error?: string) => (error ? "border-red" : "border-key-border focus:border-fg-40");
  return (
    <div className="flex min-h-0 flex-1 flex-col" onKeyDown={onKeyDown}>
      <header className="flex h-header shrink-0 items-center gap-3 border-b border-separator px-search-inset">
        <BackButton onBack={onBack} />
        <h2 className="text-13 font-medium text-fg-60">{title}</h2>
      </header>
      <form
        className="grid min-h-0 flex-1 auto-rows-min grid-cols-[175px_360px] content-start gap-x-5 gap-y-5 overflow-y-auto pt-1.5 pb-4"
        onSubmit={(e) => {
          e.preventDefault();
          save();
        }}
      >
        <label htmlFor={`${ids}-text`} className="pt-2 text-right text-13 text-fg-60">
          Snippet
        </label>
        <div className="flex flex-col gap-1.5">
          <textarea
            id={`${ids}-text`}
            ref={textRef}
            autoFocus={!editing || !initial?.name}
            spellCheck={false}
            value={text}
            onChange={(e) => setText(e.target.value)}
            aria-invalid={!!errors.text}
            aria-describedby={`${ids}-text-help`}
            className={`${field} h-[120px] resize-none py-2 font-mono ${border(errors.text)}`}
          />
          <p id={`${ids}-text-help`} className={`text-12 ${errors.text ? "text-red" : "text-fg-60"}`}>
            {errors.text ?? "Include {clipboard}, {date}, {time}, {datetime}, {day}, {uuid}, {cursor} or {snippet name=\"…\"} to fill them in when it's inserted."}
          </p>
        </div>
        <label htmlFor={`${ids}-name`} className="pt-2 text-right text-13 text-fg-60">
          Name
        </label>
        <div className="flex flex-col gap-1.5">
          <input
            id={`${ids}-name`}
            ref={nameRef}
            autoFocus={editing && !!initial?.name}
            spellCheck={false}
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="Snippet name"
            aria-invalid={!!errors.name}
            className={`${field} h-[34px] ${border(errors.name)}`}
          />
          {errors.name && <p className="text-12 text-red">{errors.name}</p>}
        </div>
        <label htmlFor={`${ids}-keyword`} className="pt-2 text-right text-13 text-fg-60">
          Keyword
        </label>
        <div className="flex flex-col gap-1.5">
          <input
            id={`${ids}-keyword`}
            ref={keywordRef}
            spellCheck={false}
            value={keyword}
            onChange={(e) => setKeyword(e.target.value)}
            placeholder="Optional keyword"
            aria-invalid={!!errors.keyword}
            aria-describedby={`${ids}-keyword-help`}
            className={`${field} h-[34px] font-mono ${border(errors.keyword)}`}
          />
          <p id={`${ids}-keyword-help`} className={`text-12 ${errors.keyword ? "text-red" : "text-fg-60"}`}>
            {errors.keyword ?? "Type it in any app to replace it with the snippet."}
          </p>
        </div>
      </form>
      <Footer
        left={
          <span className="flex items-center gap-2 text-fg-60">
            <GlyphTile spec={ICON} small />
            {title}
          </span>
        }
        primaryTitle="Save Snippet"
        primaryKeys={["Ctrl", "↵"]}
        showActions={false}
        panelOpen={false}
        onTogglePanel={() => {}}
      />
    </div>
  );
}

function when(ms?: number): string {
  return ms ? new Date(ms).toLocaleString("en-US", { dateStyle: "medium", timeStyle: "short" }) : "Never";
}

type Editing = { snippet?: Partial<Snippet>; title: string } | null;

function SearchSnippets({ pop }: ViewProps) {
  const [list, setList] = useState<Snippet[] | null>(null);
  const [editing, setEditing] = useState<Editing>(null);
  const [app, setApp] = useState<AppInfo | null>(null);

  const reload = useCallback(() => {
    loadSnippets()
      .then(setList)
      .catch((e) => {
        setList([]);
        fail("Could not load Snippets", e);
      });
  }, []);
  useEffect(() => {
    reload();
    frontmostApp().then(setApp);
  }, [reload]);

  const sections = useMemo(() => {
    const all = list ?? [];
    const update = async (next: Snippet[], title: string) => {
      await saveSnippets(next);
      showToast({ variant: "success", title });
      reload();
    };
    const toItem = (snippet: Snippet): Item => {
      const actions: Action[][] = [
        [
          {
            id: "paste",
            title: pasteTitle(app),
            icon: ClipboardPaste,
            run: async () => {
              const { text, cursorBack } = await resolve(snippet, all);
              await invoke("paste_text", { text });
              if (cursorBack > 0) window.setTimeout(() => invoke("snippet_type", { text: "", cursorBack }), 250);
              await markUsed(snippet.id);
            },
          },
          {
            id: "copy",
            title: "Copy to Clipboard",
            icon: Clipboard,
            shortcut: { key: "Enter", ctrl: true },
            run: async () => {
              const { text } = await resolve(snippet, all);
              await markUsed(snippet.id);
              await copyAndClose(text, "Copied snippet to clipboard");
            },
          },
        ],
        [
          snippet.pinned
            ? { id: "unpin", title: "Unpin Snippet", icon: PinOff, shortcut: { key: ".", ctrl: true }, keepOpen: true, run: () => update(all.map((s) => (s.id === snippet.id ? { ...s, pinned: false } : s)), "Unpinned Snippet") }
            : { id: "pin", title: "Pin Snippet", icon: Pin, shortcut: { key: ".", ctrl: true }, keepOpen: true, run: () => update(all.map((s) => (s.id === snippet.id ? { ...s, pinned: true } : s)), "Pinned Snippet") },
          { id: "edit", title: "Edit Snippet", icon: Pencil, shortcut: { key: "e", ctrl: true }, keepOpen: true, run: () => setEditing({ snippet, title: "Edit Snippet" }) },
          {
            id: "duplicate",
            title: "Duplicate Snippet",
            icon: CopyPlus,
            keepOpen: true,
            run: () => setEditing({ snippet: { name: `${snippet.name} Copy`, text: snippet.text }, title: "Create Snippet" }),
          },
          { id: "create", title: "Create Snippet", icon: Plus, shortcut: { key: "n", ctrl: true }, keepOpen: true, run: () => setEditing({ title: "Create Snippet" }) },
        ],
        [
          {
            id: "delete",
            title: "Delete Snippet",
            icon: Trash2,
            destructive: true,
            keepOpen: true,
            shortcut: { key: "d", ctrl: true },
            confirm: { title: "Delete Snippet?", message: `“${snippet.name}” will be deleted. This can't be undone.`, confirmTitle: "Delete" },
            run: () => update(all.filter((s) => s.id !== snippet.id), "Deleted Snippet"),
          },
          {
            id: "delete-all",
            title: "Delete All Snippets",
            icon: Trash2,
            destructive: true,
            keepOpen: true,
            shortcut: { key: "d", ctrl: true, shift: true },
            confirm: { title: "Delete All Snippets?", message: `All ${all.length} snippets will be deleted. This can't be undone.`, confirmTitle: "Delete All" },
            run: () => update([], "Deleted All Snippets"),
          },
        ],
      ];
      return { id: snippet.id, title: snippet.name, keywords: [snippet.keyword ?? "", snippet.text], icon: <GlyphTile spec={{ glyph: FileText, tone: "slate" }} />, accessory: snippet.keyword, actions };
    };
    const used = (s: Snippet) => s.copiedAt ?? s.modifiedAt;
    const pinned = all.filter((s) => s.pinned);
    const rest = all.filter((s) => !s.pinned).sort((a, b) => used(b) - used(a));
    return [
      { title: "Pinned", items: pinned.map(toItem) },
      { title: "Snippets", items: rest.map(toItem) },
    ].filter((section) => section.items.length > 0);
  }, [list, app, reload]);

  if (editing) {
    const done = () => {
      setEditing(null);
      reload();
    };
    return <SnippetForm initial={editing.snippet} title={editing.title} onDone={done} onBack={() => setEditing(null)} />;
  }

  return (
    <ListView
      placeholder="Search snippets…"
      sections={sections}
      onBack={pop}
      footerTitle="Search Snippets"
      footerIcon={<GlyphTile spec={ICON} small />}
      isLoading={list === null}
      emptyText={list?.length ? "No Snippets Found" : "Write faster by storing and inserting frequently used text."}
      emptyDescription={list?.length ? undefined : "Press Ctrl+N to create your first snippet."}
      detail={(item) => {
        const snippet = list?.find((s) => s.id === item.id);
        if (!snippet) return null;
        const rows = [
          { label: "Name", value: <span className="truncate">{snippet.name}</span> },
          ...(snippet.keyword ? [{ label: "Keyword", value: <span className="truncate font-mono">{snippet.keyword}</span> }] : []),
          { label: "Content Type", value: "Text" },
          { label: "Times Copied", value: snippet.copyCount ?? 0 },
          { label: "Last Copied", value: when(snippet.copiedAt) },
          { label: "Modified", value: when(snippet.modifiedAt) },
          { label: "Created", value: when(snippet.createdAt) },
        ];
        return (
          <>
            <div className="min-h-0 flex-1 overflow-y-auto p-4 font-mono text-13 break-words whitespace-pre-wrap text-fg">{snippet.text}</div>
            <Metadata rows={rows} />
          </>
        );
      }}
    />
  );
}

function CreateSnippet({ pop }: ViewProps) {
  return <SnippetForm title="Create Snippet" onDone={pop} onBack={pop} />;
}

async function importSnippets() {
  const [path] = await invoke<string[]>("flowkey_pick_path", { directory: false, multiple: false, extensions: ["json"] });
  if (!path) return;
  let incoming: ReturnType<typeof parseSnippetImport>;
  try {
    incoming = parseSnippetImport(await invoke<string>("read_json_file", { path }));
  } catch {
    throw new Error("Invalid snippet import format");
  }
  const { snippets, added, skipped } = mergeImport(await loadSnippets(), incoming, Date.now(), () => crypto.randomUUID());
  await saveSnippets(snippets);
  const plural = (n: number) => (n === 1 ? "snippet" : "snippets");
  showToast({ variant: added > 0 ? "success" : "failure", title: `${added} ${plural(added)} added`, message: skipped > 0 ? `${skipped} ${plural(skipped)} skipped` : undefined });
}

async function exportAll() {
  const list = await loadSnippets();
  if (list.length === 0) throw new Error("There are no snippets to export");
  const [folder] = await invoke<string[]>("flowkey_pick_path", { directory: true, multiple: false });
  if (!folder) return;
  const path = `${folder.replace(/\\$/, "")}\\snippets.json`;
  await invoke("flowkey_fs", { op: "writeText", path, content: exportSnippets(list) });
  await invoke("show_in_explorer", { path });
}

export default defineExtension({
  id: "snippets",
  title: "Snippets",
  icon: ICON,
  commands: [
    {
      id: "create",
      title: "Create Snippet",
      subtitle: "Snippets",
      icon: { glyph: Plus, tone: "blue" },
      keywords: ["new snippet", "text", "expand"],
      kind: "Command",
      primaryActionTitle: "Open Command",
      view: CreateSnippet,
    },
    {
      id: "search",
      title: "Search Snippets",
      subtitle: "Snippets",
      keywords: ["snippet", "text", "expand", "keyword"],
      kind: "Command",
      primaryActionTitle: "Open Command",
      view: SearchSnippets,
    },
    {
      id: "import",
      title: "Import Snippets",
      subtitle: "Snippets",
      icon: { glyph: Download, tone: "blue" },
      keywords: ["snippet", "json"],
      kind: "Command",
      primaryActionTitle: "Run Command",
      run: importSnippets,
    },
    {
      id: "export",
      title: "Export Snippets",
      subtitle: "Snippets",
      icon: { glyph: Upload, tone: "blue" },
      keywords: ["snippet", "json", "backup"],
      kind: "Command",
      primaryActionTitle: "Run Command",
      run: exportAll,
    },
  ],
});
