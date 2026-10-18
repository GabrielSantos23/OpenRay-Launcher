import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { TaskItem, TaskList } from "@tiptap/extension-list";
import { Markdown } from "@tiptap/markdown";
import { EditorContent, useEditor, useEditorState, type Editor } from "@tiptap/react";
import StarterKit from "@tiptap/starter-kit";
import {
  ArrowLeftCircle,
  ArrowRightCircle,
  Bold,
  ChevronDown,
  CircleX,
  Code,
  Copy,
  CopyPlus,
  FileCode,
  FileText,
  Files,
  Heading,
  Heading1,
  Heading2,
  Heading3,
  Italic,
  Link,
  List,
  ListChecks,
  ListOrdered,
  Pilcrow,
  Pin,
  PinOff,
  Plus,
  SquareCode,
  Strikethrough,
  Terminal,
  TextQuote,
  Trash2,
  Type,
  Underline,
  X,
} from "lucide-react";
import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { fail, useActionHost } from "./ActionHost";
import type { PanelPlacement } from "./ActionPanel";
import { matchesShortcut } from "./actions";
import type { Action, Shortcut } from "./extension-api";
import { updatedAgo } from "./extensions/calculator";
import { browseSections, countLabel, currentNote, emptyNote, loadNotes, newNote, noteTitle, saveNotes, type Note, type NotesRequest } from "./notes";
import { matchScore } from "./search";
import { useSettings } from "./settings";
import { showToast } from "./toast";
import { TipButton } from "./Tooltip";

const SAVE_DELAY_MS = 300;
const TOP_PANEL: PanelPlacement = { className: "absolute top-[118px] left-1/2 w-[320px] -translate-x-1/2", searchFirst: true };
const FORMAT_PANEL: PanelPlacement = { className: "absolute bottom-10 left-1/2 w-[260px] -translate-x-1/2", searchFirst: true };

const SHORTCUTS = {
  actions: { key: "k", ctrl: true },
  browse: { key: "p", ctrl: true },
  create: { key: "n", ctrl: true },
  pin: { key: "p", ctrl: true, shift: true },
  back: { key: "[", ctrl: true },
  forward: { key: "]", ctrl: true },
  copy: { key: "c", ctrl: true, shift: true },
  formatBar: { key: ",", ctrl: true, alt: true },
  remove: { key: "d", ctrl: true },
  removeAll: { key: "d", ctrl: true, alt: true },
} satisfies Record<string, Shortcut>;

function readLocal<T extends string>(key: string, fallback: T): T {
  try {
    return (localStorage.getItem(key) as T | null) ?? fallback;
  } catch {
    return fallback;
  }
}

function writeLocal(key: string, value: string) {
  try {
    localStorage.setItem(key, value);
  } catch {
    return;
  }
}

function keepOpen(groups: Action[][]): Action[][] {
  return groups.map((group) =>
    group.map((action) => ({ ...action, keepOpen: true, submenu: action.submenu && (async () => keepOpen([await action.submenu!()])[0]) })),
  );
}

const iconButton = "flex size-6 shrink-0 items-center justify-center rounded-[5px] text-fg hover:bg-control";
const formatButton = "flex h-6 shrink-0 items-center justify-center gap-0.5 rounded-[5px] px-1 text-fg-60 hover:bg-control hover:text-fg aria-pressed:text-fg";

function Divider() {
  return <span aria-hidden="true" className="mx-1 h-4 w-px bg-separator" />;
}

function FormatButton({ label, shortcut, onClick, active, menu = false, children }: { label: string; shortcut?: Shortcut; onClick: () => void; active?: boolean; menu?: boolean; children: ReactNode }) {
  return (
    <TipButton label={label} shortcut={shortcut} onClick={onClick} active={active} className={`${formatButton} ${menu ? "" : "w-6"}`}>
      {children}
      {menu && <ChevronDown className="size-3" />}
    </TipButton>
  );
}

function BrowseNotes({
  notes,
  currentId,
  onOpen,
  onPin,
  onDelete,
  onClose,
}: {
  notes: Note[];
  currentId?: string;
  onOpen: (id: string) => void;
  onPin: (note: Note) => void;
  onDelete: (note: Note) => void;
  onClose: () => void;
}) {
  const [query, setQuery] = useState("");
  const [selected, setSelected] = useState(0);
  const sections = useMemo(
    () =>
      browseSections(notes)
        .map((s) => ({ ...s, notes: s.notes.filter((n) => matchScore(query, noteTitle(n.text), [n.text]) > 0) }))
        .filter((s) => s.notes.length > 0),
    [notes, query],
  );
  const flat = sections.flatMap((s) => s.notes);
  const active = Math.min(selected, flat.length - 1);
  const now = Date.now();

  function onKeyDown(event: React.KeyboardEvent) {
    if (event.key === "Escape" || matchesShortcut(event, SHORTCUTS.browse)) {
      event.preventDefault();
      onClose();
    } else if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      setSelected(Math.max(0, Math.min(flat.length - 1, active + (event.key === "ArrowDown" ? 1 : -1))));
    } else if (event.key === "Enter" && flat[active]) {
      event.preventDefault();
      onOpen(flat[active].id);
    } else if (matchesShortcut(event, SHORTCUTS.pin) && flat[active]) {
      event.preventDefault();
      onPin(flat[active]);
    } else if (matchesShortcut(event, SHORTCUTS.remove) && flat[active]) {
      event.preventDefault();
      onDelete(flat[active]);
    }
  }

  let index = -1;
  return (
    <>
      <div className="fixed inset-0 z-10" onMouseDown={onClose} />
      <div role="dialog" aria-label="Browse Notes" className={`z-20 flex flex-col rounded-panel border border-panel-border bg-panel shadow-panel ${TOP_PANEL.className}`}>
        <input
          autoFocus
          spellCheck={false}
          value={query}
          onChange={(e) => {
            setQuery(e.target.value);
            setSelected(0);
          }}
          onKeyDown={onKeyDown}
          placeholder="Search for notes..."
          aria-label="Search for notes"
          role="combobox"
          aria-expanded="true"
          aria-controls="browse-notes"
          aria-activedescendant={flat[active] ? `note-${active}` : undefined}
          className="h-[38px] shrink-0 border-b border-panel-separator bg-transparent px-[15px] text-13 text-fg caret-fg outline-none placeholder:text-fg-40"
        />
        <div id="browse-notes" role="listbox" aria-label="Notes" className="max-h-[320px] overflow-y-auto [scrollbar-width:none] px-[7px] pt-[7px] pb-2">
          {flat.length === 0 && <p className="flex h-8 items-center px-2 text-13 text-fg-60">No matching notes</p>}
          {sections.map((section) => (
            <div key={section.title} role="group" aria-label={section.title}>
              <div className="flex h-6 items-center px-2 text-11 font-semibold text-fg-60">{section.title}</div>
              {section.notes.map((note) => {
                index += 1;
                const i = index;
                const current = note.id === currentId;
                const count = countLabel(note.text, "characters");
                return (
                  <div
                    key={note.id}
                    id={`note-${i}`}
                    role="option"
                    aria-selected={i === active}
                    onMouseMove={() => i !== active && setSelected(i)}
                    onClick={() => onOpen(note.id)}
                    className={`group flex h-[52px] cursor-default items-center gap-2 rounded-row px-2 ${i === active ? "bg-panel-selection" : ""}`}
                  >
                    <div className="flex min-w-0 flex-1 flex-col">
                      <span className="truncate text-13 font-medium text-fg">{noteTitle(note.text)}</span>
                      <span className="flex items-center gap-1 truncate text-12 text-fg-60">
                        {current && <span aria-hidden="true" className="size-[5px] shrink-0 rounded-full bg-red" />}
                        {current ? `Current • ${count}` : `${updatedAgo(note.lastAccessedAt, now, "Opened")} • ${count}`}
                      </span>
                    </div>
                    <div className={`flex gap-1 ${i === active ? "" : "invisible"}`}>
                      <button
                        type="button"
                        tabIndex={-1}
                        aria-label={note.pinned ? "Unpin note" : "Pin note"}
                        onClick={(e) => {
                          e.stopPropagation();
                          onPin(note);
                        }}
                        className="flex size-6 items-center justify-center rounded-[5px] text-fg-60 hover:bg-control hover:text-fg"
                      >
                        {note.pinned ? <PinOff className="size-4" /> : <Pin className="size-4" />}
                      </button>
                      <button
                        type="button"
                        tabIndex={-1}
                        aria-label="Delete note"
                        onClick={(e) => {
                          e.stopPropagation();
                          onDelete(note);
                        }}
                        className="flex size-6 items-center justify-center rounded-[5px] text-fg-60 hover:bg-control hover:text-fg"
                      >
                        <Trash2 className="size-4" />
                      </button>
                    </div>
                  </div>
                );
              })}
            </div>
          ))}
        </div>
      </div>
    </>
  );
}

function LinkField({ initial, onSubmit, onClose }: { initial: string; onSubmit: (url: string) => void; onClose: () => void }) {
  const [url, setUrl] = useState(initial);
  return (
    <>
      <div className="fixed inset-0 z-10" onMouseDown={onClose} />
      <form
        onSubmit={(e) => {
          e.preventDefault();
          onSubmit(url.trim());
        }}
        className={`z-20 flex rounded-panel border border-panel-border bg-panel shadow-panel ${FORMAT_PANEL.className}`}
      >
        <input
          autoFocus
          spellCheck={false}
          value={url}
          onChange={(e) => setUrl(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Escape") {
              e.preventDefault();
              onClose();
            }
          }}
          placeholder="Paste or type a link..."
          aria-label="Link URL"
          className="h-[38px] min-w-0 flex-1 bg-transparent px-[15px] text-13 text-fg caret-fg outline-none placeholder:text-fg-40"
        />
      </form>
    </>
  );
}

function withProtocol(url: string): string {
  return /^[a-z][a-z0-9+.-]*:/i.test(url) ? url : `https://${url}`;
}

export function NotesWindow() {
  const [settings] = useSettings();
  const [notes, setNotes] = useState<Note[]>([]);
  const [currentId, setCurrentId] = useState<string>();
  const [history, setHistory] = useState<{ stack: string[]; index: number }>({ stack: [], index: -1 });
  const [browsing, setBrowsing] = useState(false);
  const [linking, setLinking] = useState(false);
  const [formatBar, setFormatBar] = useState(() => readLocal("notes.formatBar", "shown") === "shown");
  const [unit, setUnit] = useState(() => readLocal<"characters" | "words">("notes.counter", "characters"));
  const [scrolled, setScrolled] = useState(false);
  const notesRef = useRef<Note[]>([]);
  const currentRef = useRef<string | undefined>(undefined);
  const saveTimer = useRef(0);
  const ready = useRef<Promise<void> | null>(null);
  const escapeCloses = useRef(true);
  escapeCloses.current = settings?.escapeKeyClosesWindow ?? true;

  function flush() {
    if (!saveTimer.current) return;
    window.clearTimeout(saveTimer.current);
    saveTimer.current = 0;
    saveNotes(notesRef.current).catch((e) => fail("Could not save note", e));
  }

  function commit(next: Note[]) {
    notesRef.current = next;
    setNotes(next);
    window.clearTimeout(saveTimer.current);
    saveTimer.current = window.setTimeout(flush, SAVE_DELAY_MS);
  }

  const editor = useEditor({
    extensions: [
      StarterKit.configure({ link: { openOnClick: false, autolink: true, defaultProtocol: "https" } }),
      TaskList,
      TaskItem.configure({ nested: true }),
      Markdown,
    ],
    editorProps: {
      attributes: { "aria-label": "Note", spellcheck: "false" },
      handleClick(_view, _pos, event) {
        const anchor = (event.target as HTMLElement).closest("a");
        if (!anchor?.href) return false;
        invoke("open_url", { url: anchor.href }).catch((e) => fail("Could not open link", e));
        return true;
      },
    },
    onUpdate({ editor }) {
      const id = currentRef.current;
      if (!id) return;
      const now = Date.now();
      const markdown = editor.getMarkdown();
      const text = editor.getText();
      commit(notesRef.current.map((n) => (n.id === id ? { ...n, markdown, text, updatedAt: now } : n)));
    },
  });

  const active = useEditorState({
    editor,
    selector: ({ editor }) => ({
      heading: editor?.isActive("heading") ?? false,
      inline: ["bold", "italic", "underline", "strike"].some((mark) => editor?.isActive(mark)),
      link: editor?.isActive("link") ?? false,
      code: editor?.isActive("code") ?? false,
      quote: editor?.isActive("blockquote") ?? false,
      codeBlock: editor?.isActive("codeBlock") ?? false,
      list: ["bulletList", "orderedList", "taskList"].some((node) => editor?.isActive(node)),
      selection: !(editor?.state.selection.empty ?? true),
    }),
  });

  const focusEditor = () => editor?.commands.focus();
  const host = useActionHost(focusEditor, TOP_PANEL);
  const formatHost = useActionHost(focusEditor, FORMAT_PANEL);

  function open(id: string, record = true) {
    const note = notesRef.current.find((n) => n.id === id);
    if (!note || !editor) return;
    flush();
    const now = Date.now();
    currentRef.current = id;
    setCurrentId(id);
    commit(notesRef.current.map((n) => (n.id === id ? { ...n, lastAccessedAt: now } : n)));
    editor.commands.setContent(note.markdown, { contentType: "markdown", emitUpdate: false });
    editor.commands.focus("end");
    if (record) setHistory((h) => ({ stack: [...h.stack.slice(0, h.index + 1).filter((x) => x !== id), id], index: Math.min(h.index + 1, h.stack.length) }));
  }

  function create() {
    const empty = emptyNote(notesRef.current);
    if (empty?.id === currentRef.current) {
      showToast({ variant: "failure", title: "You already have an empty note" });
      return;
    }
    if (empty) return open(empty.id);
    const note = newNote(Date.now());
    commit([...notesRef.current, note]);
    open(note.id);
  }

  function remove(id: string) {
    const rest = notesRef.current.filter((n) => n.id !== id);
    setHistory((h) => {
      const stack = h.stack.filter((x) => x !== id);
      return { stack, index: Math.min(h.index, stack.length - 1) };
    });
    if (rest.length === 0) {
      const note = newNote(Date.now());
      commit([note]);
      return open(note.id);
    }
    commit(rest);
    if (id === currentRef.current) open(currentNote(rest)!.id);
  }

  function togglePin(note: Note) {
    commit(notesRef.current.map((n) => (n.id === note.id ? { ...n, pinned: !n.pinned } : n)));
    showToast({ variant: "success", title: note.pinned ? "Unpinned Note" : "Pinned Note" });
  }

  function step(delta: number) {
    const index = history.index + delta;
    const id = history.stack[index];
    if (!id) return;
    setHistory((h) => ({ ...h, index }));
    open(id, false);
  }

  function copyAs(title: string, text: string) {
    invoke("copy_text", { text }).then(
      () => showToast({ variant: "success", title: `Copied as ${title}` }),
      (e) => fail("Could not copy note", e),
    );
  }

  function toggleFormatBar() {
    setFormatBar((shown) => {
      writeLocal("notes.formatBar", shown ? "hidden" : "shown");
      return !shown;
    });
  }

  function deleteConfirmation(note: Note): Action {
    return {
      id: "delete",
      title: "Delete Note",
      icon: Trash2,
      destructive: true,
      shortcut: SHORTCUTS.remove,
      confirm: { title: "Delete Note?", message: `“${noteTitle(note.text)}” will be deleted. This can't be undone.`, confirmTitle: "Delete Note" },
      run: () => {
        remove(note.id);
        showToast({ variant: "success", title: "Note deleted" });
      },
    };
  }

  const current = notes.find((n) => n.id === currentId);
  const empty = !current?.text.trim();

  function groups(): Action[][] {
    if (!current || !editor) return [];
    const selection = !editor.state.selection.empty;
    const selected = () => editor.state.doc.cut(editor.state.selection.from, editor.state.selection.to);
    return keepOpen([
      [
        { id: "new", title: "New Note", icon: Plus, shortcut: SHORTCUTS.create, disabled: empty, run: create },
        {
          id: "duplicate",
          title: "Duplicate Note",
          icon: CopyPlus,
          disabled: empty,
          run: () => {
            const note = newNote(Date.now(), current.markdown, current.text);
            commit([...notesRef.current, note]);
            open(note.id);
            showToast({ variant: "success", title: "Note duplicated" });
          },
        },
        { id: "pin", title: current.pinned ? "Unpin Note" : "Pin Note", icon: current.pinned ? PinOff : Pin, shortcut: SHORTCUTS.pin, run: () => togglePin(current) },
      ],
      [
        { id: "browse", title: "Browse Notes", icon: Files, shortcut: SHORTCUTS.browse, run: () => setBrowsing(true) },
        { id: "back", title: "Go Back", icon: ArrowLeftCircle, shortcut: SHORTCUTS.back, disabled: history.index <= 0, run: () => step(-1) },
        { id: "forward", title: "Go Forward", icon: ArrowRightCircle, shortcut: SHORTCUTS.forward, disabled: history.index >= history.stack.length - 1, run: () => step(1) },
      ],
      [
        {
          id: "copy",
          title: selection ? "Copy Selection As…" : "Copy Note As…",
          icon: Copy,
          shortcut: SHORTCUTS.copy,
          run: () => undefined,
          submenu: () => {
            const json = selection ? { type: "doc", content: selected().toJSON().content } : editor.getJSON();
            const markdown = selection ? editor.markdown!.serialize(json) : editor.getMarkdown();
            const text = selection ? editor.state.doc.textBetween(editor.state.selection.from, editor.state.selection.to, "\n\n") : editor.getText();
            const html = selection ? new XMLSerializer().serializeToString(editor.view.serializeForClipboard(editor.state.selection.content()).dom) : editor.getHTML();
            return [
              { id: "markdown", title: "Markdown", icon: FileCode, run: () => copyAs("Markdown", markdown) },
              { id: "text", title: "Plain Text", icon: FileText, run: () => copyAs("Plain Text", text) },
              { id: "html", title: "HTML", icon: Code, run: () => copyAs("HTML", html) },
            ];
          },
        },
      ],
      [
        { id: "format", title: "Format…", icon: Type, run: () => undefined, submenu: () => inlineFormats() },
        { id: "heading", title: "Heading…", icon: Heading, run: () => undefined, submenu: () => headings() },
        { id: "list", title: "List…", icon: List, run: () => undefined, submenu: () => lists() },
        { id: "format-bar", title: formatBar ? "Hide Format Bar" : "Show Format Bar", icon: Type, shortcut: SHORTCUTS.formatBar, run: toggleFormatBar },
      ],
      [
        deleteConfirmation(current),
        {
          id: "delete-all",
          title: "Delete All Notes",
          icon: Trash2,
          destructive: true,
          shortcut: SHORTCUTS.removeAll,
          confirm: { title: "Delete All Notes?", message: "All notes will be deleted. This can't be undone.", confirmTitle: "Delete All Notes" },
          run: () => {
            const note = newNote(Date.now());
            setHistory({ stack: [], index: -1 });
            commit([note]);
            open(note.id);
            showToast({ variant: "success", title: "All notes deleted" });
          },
        },
      ],
    ]);
  }

  function chain(editor: Editor) {
    return editor.chain().focus();
  }

  function headings(): Action[] {
    if (!editor) return [];
    return [
      { id: "text", title: "Text", icon: Pilcrow, shortcut: { key: "0", ctrl: true, alt: true }, run: () => void chain(editor).setParagraph().run() },
      { id: "h1", title: "Heading 1", icon: Heading1, shortcut: { key: "1", ctrl: true, alt: true }, run: () => void chain(editor).toggleHeading({ level: 1 }).run() },
      { id: "h2", title: "Heading 2", icon: Heading2, shortcut: { key: "2", ctrl: true, alt: true }, run: () => void chain(editor).toggleHeading({ level: 2 }).run() },
      { id: "h3", title: "Heading 3", icon: Heading3, shortcut: { key: "3", ctrl: true, alt: true }, run: () => void chain(editor).toggleHeading({ level: 3 }).run() },
    ];
  }

  function inlineFormats(): Action[] {
    if (!editor) return [];
    return [
      { id: "bold", title: "Bold", icon: Bold, shortcut: { key: "b", ctrl: true }, run: () => void chain(editor).toggleBold().run() },
      { id: "italic", title: "Italic", icon: Italic, shortcut: { key: "i", ctrl: true }, run: () => void chain(editor).toggleItalic().run() },
      { id: "underline", title: "Underline", icon: Underline, shortcut: { key: "u", ctrl: true }, run: () => void chain(editor).toggleUnderline().run() },
      { id: "strike", title: "Strikethrough", icon: Strikethrough, shortcut: { key: "s", ctrl: true, shift: true }, run: () => void chain(editor).toggleStrike().run() },
    ];
  }

  function lists(): Action[] {
    if (!editor) return [];
    return [
      { id: "bullet", title: "Bulleted List", icon: List, shortcut: { key: "8", ctrl: true, shift: true }, run: () => void chain(editor).toggleBulletList().run() },
      { id: "ordered", title: "Numbered List", icon: ListOrdered, shortcut: { key: "7", ctrl: true, shift: true }, run: () => void chain(editor).toggleOrderedList().run() },
      { id: "task", title: "Task List", icon: ListChecks, shortcut: { key: "9", ctrl: true, shift: true }, run: () => void chain(editor).toggleTaskList().run() },
    ];
  }

  function openFormat(title: string, actions: Action[]) {
    formatHost.openPanel(title, keepOpen([actions]));
  }

  function runAction(action: Action) {
    host.run(action, { failTitle: `${action.title.replace(/…$/, "")} failed` });
  }

  function handle(request: NotesRequest) {
    ready.current?.then(() => {
      if (request.id) open(request.id);
      else if (request.create) create();
      else focusEditor();
    });
  }

  const handleRef = useRef(handle);
  handleRef.current = handle;
  const groupsRef = useRef(groups);
  groupsRef.current = groups;
  const runRef = useRef(runAction);
  runRef.current = runAction;
  const overlayOpen = host.panelOpen || formatHost.panelOpen || browsing || linking;
  const overlayRef = useRef(overlayOpen);
  overlayRef.current = overlayOpen;

  useEffect(() => {
    if (!editor) return;
    ready.current = loadNotes()
      .catch((e) => {
        fail("Could not load notes", e);
        return [];
      })
      .then((list) => {
        const initial = list.length ? list : [newNote(Date.now())];
        notesRef.current = initial;
        setNotes(initial);
        open(currentNote(initial)!.id);
      });
    const unlisten = listen<NotesRequest>("notes-open", (event) => handleRef.current(event.payload ?? {}));
    const onBlur = () => flush();
    window.addEventListener("blur", onBlur);
    return () => {
      unlisten.then((off) => off());
      window.removeEventListener("blur", onBlur);
      flush();
    };
  }, [editor]);

  useEffect(() => {
    const onShortcut = (event: KeyboardEvent) => {
      if (overlayRef.current) return;
      if (matchesShortcut(event, SHORTCUTS.actions)) {
        event.preventDefault();
        host.openPanel("Actions", groupsRef.current());
        return;
      }
      const action = groupsRef.current().flat().find((a) => a.shortcut && matchesShortcut(event, a.shortcut));
      if (!action) return;
      event.preventDefault();
      event.stopPropagation();
      if (!action.disabled) runRef.current(action);
    };
    const onEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape" && !event.defaultPrevented && !overlayRef.current && escapeCloses.current) getCurrentWindow().hide();
    };
    window.addEventListener("keydown", onShortcut, true);
    window.addEventListener("keydown", onEscape);
    return () => {
      window.removeEventListener("keydown", onShortcut, true);
      window.removeEventListener("keydown", onEscape);
    };
  }, []);

  const title = current ? noteTitle(current.text) : "Notes";

  return (
    <div className="flex h-full flex-col bg-bg text-fg">
      <header data-tauri-drag-region className="relative flex h-10 shrink-0 items-center gap-2 p-2">
        <TipButton label="Action Panel" shortcut={SHORTCUTS.actions} onClick={() => host.openPanel("Actions", groups())} className={iconButton}>
          <Terminal className="size-4" />
        </TipButton>
        <TipButton label="Browse Notes" shortcut={SHORTCUTS.browse} onClick={() => setBrowsing(true)} className={iconButton}>
          <Files className="size-4" />
        </TipButton>
        <TipButton label="Create Note" shortcut={SHORTCUTS.create} onClick={create} className={iconButton}>
          <Plus className="size-4" />
        </TipButton>
        <h1 className="pointer-events-none absolute inset-y-2 right-[108px] left-[108px] flex items-center justify-center font-['Segoe_UI','Segoe_UI_Variable',sans-serif] text-13 text-fg-60">
          <span className="truncate">{title}</span>
        </h1>
        <span data-tauri-drag-region className="flex-1 self-stretch" />
        <button type="button" tabIndex={-1} aria-label="Close" title="Close" onClick={() => getCurrentWindow().hide()} className={iconButton}>
          <X className="size-4" />
        </button>
      </header>
      <div aria-hidden="true" className={`h-px shrink-0 bg-separator transition-opacity duration-150 ${scrolled ? "opacity-100" : "opacity-0"}`} />
      <div className="notes-editor min-h-0 flex-1 cursor-text overflow-y-auto" onScroll={(e) => setScrolled(e.currentTarget.scrollTop > 0)} onMouseDown={(e) => e.target === e.currentTarget && (e.preventDefault(), editor?.commands.focus("end"))}>
        <EditorContent editor={editor} className="min-h-full" />
      </div>
      <footer className="relative flex h-8 shrink-0 items-center px-2">
        {formatBar ? (
          <>
            <div className="flex flex-1 items-center justify-center gap-1">
              <FormatButton label="Heading" menu active={active?.heading} onClick={() => openFormat("Heading", headings())}>
                <Heading className="size-4" />
              </FormatButton>
              <FormatButton label="Format" menu active={active?.inline} onClick={() => openFormat("Format", inlineFormats())}>
                <Type className="size-4" />
              </FormatButton>
              <FormatButton label="Link" active={active?.link} onClick={() => (active?.link ? editor && chain(editor).extendMarkRange("link").unsetLink().run() : setLinking(true))}>
                <Link className="size-4" />
              </FormatButton>
              <FormatButton label="Code" shortcut={{ key: "e", ctrl: true }} active={active?.code} onClick={() => editor && chain(editor).toggleCode().run()}>
                <Code className="size-4" />
              </FormatButton>
              <Divider />
              <FormatButton label="Quote" shortcut={{ key: "b", ctrl: true, shift: true }} active={active?.quote} onClick={() => editor && chain(editor).toggleBlockquote().run()}>
                <TextQuote className="size-4" />
              </FormatButton>
              <FormatButton label="Code Block" shortcut={{ key: "c", ctrl: true, alt: true }} active={active?.codeBlock} onClick={() => editor && chain(editor).toggleCodeBlock().run()}>
                <SquareCode className="size-4" />
              </FormatButton>
              <Divider />
              <FormatButton label="List" menu active={active?.list} onClick={() => openFormat("List", lists())}>
                <List className="size-4" />
              </FormatButton>
            </div>
            <TipButton label="Hide Format Buttons" shortcut={SHORTCUTS.formatBar} onClick={toggleFormatBar} className={`${formatButton} absolute right-2 w-6`}>
              <CircleX className="size-4" />
            </TipButton>
          </>
        ) : (
          <>
            <TipButton
              label="Toggle between character count and word count"
              onClick={() =>
                setUnit((u) => {
                  const next = u === "characters" ? "words" : "characters";
                  writeLocal("notes.counter", next);
                  return next;
                })
              }
              className="mx-auto h-6 rounded-[5px] px-2 text-12 text-fg-40 tabular-nums hover:text-fg-60"
            >
              {countLabel(current?.text ?? "", unit)}
            </TipButton>
            <TipButton label="Show Format Buttons" shortcut={SHORTCUTS.formatBar} onClick={toggleFormatBar} className={`${formatButton} absolute right-2 w-6`}>
              <Type className="size-4" />
            </TipButton>
          </>
        )}
      </footer>
      {browsing && (
        <BrowseNotes
          notes={notes}
          currentId={currentId}
          onOpen={(id) => {
            setBrowsing(false);
            open(id);
          }}
          onPin={togglePin}
          onDelete={(note) => {
            setBrowsing(false);
            runAction(keepOpen([[deleteConfirmation(note)]])[0][0]);
          }}
          onClose={() => {
            setBrowsing(false);
            focusEditor();
          }}
        />
      )}
      {linking && editor && (
        <LinkField
          initial={editor.getAttributes("link").href ?? ""}
          onSubmit={(url) => {
            setLinking(false);
            const c = chain(editor).extendMarkRange("link");
            if (!url) return void c.unsetLink().run();
            if (editor.state.selection.empty) c.insertContent({ type: "text", text: url, marks: [{ type: "link", attrs: { href: withProtocol(url) } }] }).run();
            else c.setLink({ href: withProtocol(url) }).run();
          }}
          onClose={() => {
            setLinking(false);
            focusEditor();
          }}
        />
      )}
      {host.render(runAction)}
      {formatHost.render((action) => formatHost.run(action, { failTitle: `${action.title} failed` }))}
    </div>
  );
}
