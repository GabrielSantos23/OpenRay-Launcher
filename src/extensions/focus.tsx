import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { AppWindow, CircleCheck, CircleX, FileUp, Link, Pause, Pencil, Play, Plus, RotateCcw, Search, Target, Trash2, X } from "lucide-react";
import { useCallback, useEffect, useId, useMemo, useState, type ReactNode } from "react";
import { fail, Footer } from "../ActionHost";
import { matchesShortcut } from "../actions";
import { Dropdown } from "../Dropdown";
import { defineExtension, type Action, type Command, type Item, type ViewProps } from "../extension-api";
import {
  CATEGORY_ICONS,
  DURATION_OPTIONS,
  EMPTY_FORM,
  FOCUS_ICON,
  formatTimer,
  loadCategories,
  loadLastForm,
  loadPreferences,
  MODE_OPTIONS,
  normalizeHost,
  parseCategoryImport,
  remainingMs,
  saveCategories,
  saveLastForm,
  slugify,
  startInput,
  type FocusCategory,
  type FocusForm,
  type FocusSession,
} from "../focus";
import { GlyphTile } from "../icons";
import { BackButton, ListView } from "../ListView";
import { showToast } from "../toast";

interface Option {
  value: string;
  label: string;
  icon: ReactNode;
}

interface InstalledApp {
  name: string;
  path: string;
}

const PAUSE_CHOICES: [string, number | null][] = [
  ["1 minute", 60_000],
  ["2 minutes", 120_000],
  ["5 minutes", 300_000],
  ["10 minutes", 600_000],
  ["Indefinitely", null],
];

function categoryIcon(category: FocusCategory, small = false) {
  return <GlyphTile spec={{ glyph: CATEGORY_ICONS[category.icon] ?? Target, tone: "green" }} small={small} />;
}

function AppIcon({ path }: { path: string }) {
  const [failed, setFailed] = useState(false);
  if (failed || !path) return <AppWindow aria-hidden="true" className="size-4 shrink-0 text-fg-60" />;
  return <img src={`${convertFileSrc(path, "appicon")}?size=${Math.round(16 * window.devicePixelRatio)}`} alt="" onError={() => setFailed(true)} className="size-4 shrink-0" />;
}

function useApps(): InstalledApp[] {
  const [apps, setApps] = useState<InstalledApp[]>([]);
  useEffect(() => {
    invoke<InstalledApp[]>("list_apps")
      .then((list) => setApps([...list].sort((a, b) => a.name.localeCompare(b.name))))
      .catch((e) => console.error("list_apps failed", e));
  }, []);
  return apps;
}

function appOption(value: string, apps: InstalledApp[]): Option {
  const app = apps.find((a) => a.path === value);
  const label = app?.name ?? value.split(/[\\/]/).pop()!.replace(/\.(exe|lnk)$/i, "");
  return { value: `app:${value}`, label, icon: <AppIcon path={app ? app.path : ""} /> };
}

function siteOption(host: string): Option {
  return { value: `site:${host}`, label: host, icon: <Link aria-hidden="true" className="size-4 shrink-0 text-fg-60" /> };
}

function TokenField({
  id,
  labelledBy,
  value,
  onChange,
  options,
  placeholder,
  createLabel,
}: {
  id: string;
  labelledBy: string;
  value: string[];
  onChange: (value: string[]) => void;
  options: Option[];
  placeholder: string;
  createLabel?: string;
}) {
  const [query, setQuery] = useState("");
  const [open, setOpen] = useState(false);
  const [active, setActive] = useState(0);
  const known = useMemo(() => new Map(options.map((o) => [o.value, o])), [options]);
  const tokens = value.map((v) => known.get(v) ?? (v.startsWith("site:") ? siteOption(v.slice(5)) : { value: v, label: v.replace(/^\w+:/, ""), icon: null }));
  const host = createLabel ? normalizeHost(query) : null;
  const matches = useMemo(() => {
    const q = query.trim().toLowerCase();
    const list = options.filter((o) => !value.includes(o.value) && (!q || o.label.toLowerCase().includes(q))).slice(0, 50);
    return host && !value.includes(`site:${host}`) ? [{ ...siteOption(host), label: `${createLabel}: ${host}` }, ...list] : list;
  }, [options, value, query, host, createLabel]);
  const current = Math.min(active, matches.length - 1);

  function add(option: Option) {
    onChange([...value, option.value]);
    setQuery("");
    setActive(0);
  }

  function onKeyDown(event: React.KeyboardEvent<HTMLInputElement>) {
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      setOpen(true);
      setActive(Math.max(0, Math.min(current + (event.key === "ArrowDown" ? 1 : -1), matches.length - 1)));
    } else if (event.key === "Enter" && !event.ctrlKey && open && matches[current]) {
      event.preventDefault();
      event.stopPropagation();
      add(matches[current]);
    } else if (event.key === "Backspace" && !query && value.length) {
      onChange(value.slice(0, -1));
    } else if (event.key === "Escape" && (open || query)) {
      event.preventDefault();
      event.stopPropagation();
      setOpen(false);
      setQuery("");
    }
  }

  return (
    <div className="flex flex-col gap-1">
      <div className="flex min-h-[34px] flex-wrap items-center gap-1 rounded-lg border border-key-border bg-card p-1 focus-within:border-fg-40">
        {tokens.map((token) => (
          <span key={token.value} className="flex h-6 max-w-full items-center gap-1.5 rounded-[5px] bg-control pr-0.5 pl-1.5 text-12 text-fg">
            {token.icon}
            <span className="truncate">{token.label}</span>
            <button
              type="button"
              tabIndex={-1}
              aria-label={`Remove ${token.label}`}
              onClick={() => onChange(value.filter((v) => v !== token.value))}
              className="flex size-5 items-center justify-center rounded text-fg-60 hover:bg-control-hover hover:text-fg"
            >
              <X className="size-3" />
            </button>
          </span>
        ))}
        <input
          id={id}
          aria-labelledby={labelledBy}
          role="combobox"
          aria-expanded={open}
          aria-controls={`${id}-list`}
          aria-activedescendant={open && matches[current] ? `${id}-${current}` : undefined}
          spellCheck={false}
          value={query}
          onChange={(e) => {
            setQuery(e.target.value);
            setOpen(true);
            setActive(0);
          }}
          onFocus={() => setOpen(true)}
          onBlur={() => setOpen(false)}
          onKeyDown={onKeyDown}
          placeholder={value.length ? "" : placeholder}
          className="h-6 min-w-[140px] flex-1 bg-transparent px-1.5 text-13 text-fg caret-fg outline-none placeholder:text-fg-40"
        />
      </div>
      {open && matches.length > 0 && (
        <div id={`${id}-list`} role="listbox" className="max-h-[164px] overflow-y-auto rounded-lg border border-panel-border bg-panel p-1">
          {matches.map((option, i) => (
            <div
              key={option.value}
              id={`${id}-${i}`}
              role="option"
              aria-selected={i === current}
              onMouseDown={(e) => {
                e.preventDefault();
                add(option);
              }}
              onMouseMove={() => setActive(i)}
              className={`flex h-8 cursor-default items-center gap-2 rounded-[6px] px-2 text-13 ${i === current ? "bg-panel-selection" : ""}`}
            >
              {option.icon}
              <span className="truncate">{option.label}</span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function FormShell({ title, primaryTitle, onSave, onBack, children }: { title: string; primaryTitle: string; onSave: () => void; onBack: () => void; children: ReactNode }) {
  return (
    <div
      className="flex min-h-0 flex-1 flex-col"
      onKeyDown={(event) => {
        if (matchesShortcut(event, { key: "Enter", ctrl: true })) {
          event.preventDefault();
          onSave();
        } else if (event.key === "Escape" && !event.defaultPrevented) {
          event.preventDefault();
          onBack();
        }
      }}
    >
      <header className="flex h-header shrink-0 items-center gap-3 border-b border-separator px-search-inset">
        <BackButton onBack={onBack} />
        <h2 className="text-13 font-medium text-fg-60">{title}</h2>
      </header>
      <form
        className="grid min-h-0 flex-1 auto-rows-min grid-cols-[175px_360px] content-start gap-x-5 gap-y-5 overflow-y-auto pt-1.5 pb-4"
        onSubmit={(e) => {
          e.preventDefault();
          onSave();
        }}
      >
        {children}
      </form>
      <Footer
        left={
          <span className="flex items-center gap-2 text-fg-60">
            <GlyphTile spec={FOCUS_ICON} small />
            Focus
          </span>
        }
        primaryTitle={primaryTitle}
        primaryKeys={["Ctrl", "↵"]}
        showActions={false}
        panelOpen={false}
        onTogglePanel={() => {}}
      />
    </div>
  );
}

const label = "pt-2 text-right text-13 text-fg-60";
const field = "h-[34px] w-full rounded-lg border border-key-border bg-card px-3 text-13 text-fg caret-fg outline-none placeholder:text-fg-40 focus:border-fg-40";

async function closeWith(title: string, message: string) {
  await invoke("hide_launcher", { restoreFocus: true });
  showToast({ variant: "success", title, message });
}

function FocusSessionForm({ variant, pop }: ViewProps & { variant: "start" | "edit" }) {
  const [form, setForm] = useState<FocusForm | null>(null);
  const [categories, setCategories] = useState<FocusCategory[]>([]);
  const apps = useApps();
  const ids = useId();

  useEffect(() => {
    loadCategories().then(setCategories).catch((e) => fail("Could not load focus categories", e));
    if (variant === "start") {
      loadLastForm()
        .then(setForm)
        .catch(() => setForm(EMPTY_FORM));
      return;
    }
    invoke<FocusSession | null>("focus_get").then((session) => {
      if (!session) {
        showToast({ variant: "failure", title: "Failed to edit focus session", message: "No focus session is running." });
        pop();
        return;
      }
      setForm(session.form ?? { ...EMPTY_FORM, goal: session.title });
    });
  }, [variant, pop]);

  const options = useMemo<Option[]>(
    () => [
      ...categories.map((c) => ({ value: `category:${c.id}`, label: c.title, icon: categoryIcon(c, true) })),
      ...apps.map((a) => appOption(a.path, apps)),
    ],
    [categories, apps],
  );

  const title = variant === "start" ? "Start Focus Session" : "Update Session";

  async function save() {
    if (!form) return;
    try {
      const input = startInput(form, categories, await loadPreferences());
      if (variant === "start") {
        await invoke("focus_start", { input });
        await saveLastForm(form);
        await closeWith(title, "Focus session started.");
      } else {
        await invoke("focus_update", { input });
        await closeWith(title, "Focus session updated.");
      }
    } catch (e) {
      fail(`Failed to ${variant === "start" ? "start" : "update"} focus session`, e);
    }
  }

  if (!form) return null;
  const set = (patch: Partial<FocusForm>) => setForm({ ...form, ...patch });
  const blocking = form.mode === "block";
  return (
    <FormShell title={title} primaryTitle={title} onSave={save} onBack={pop}>
      <label htmlFor={`${ids}-goal`} className={label}>
        Goal
      </label>
      <input
        id={`${ids}-goal`}
        autoFocus
        spellCheck={false}
        value={form.goal}
        onChange={(e) => set({ goal: e.target.value })}
        placeholder="What are you focusing on?"
        className={field}
      />
      <span id={`${ids}-duration`} className={label}>
        Duration
      </span>
      <Dropdown
        size="field"
        labelledBy={`${ids}-duration`}
        value={form.durationMs ? String(form.durationMs) : ""}
        options={
          form.durationMs && !DURATION_OPTIONS.some((o) => o.value === String(form.durationMs))
            ? [...DURATION_OPTIONS, { value: String(form.durationMs), label: formatTimer(form.durationMs) }]
            : DURATION_OPTIONS
        }
        onChange={(value) => set({ durationMs: value ? Number(value) : null })}
      />
      <span id={`${ids}-mode`} className={label}>
        Mode
      </span>
      <Dropdown size="field" labelledBy={`${ids}-mode`} value={form.mode} options={MODE_OPTIONS} onChange={(mode) => set({ mode })} />
      <span id={`${ids}-items`} className={label}>
        {blocking ? "Block" : "Allow"}
      </span>
      <TokenField
        id={`${ids}-items-input`}
        labelledBy={`${ids}-items`}
        value={form.items}
        onChange={(items) => set({ items })}
        options={options}
        placeholder={blocking ? "What apps or websites do you want to block?" : "What apps or websites do you want to allow?"}
        createLabel={blocking ? "Block website" : "Allow website"}
      />
    </FormShell>
  );
}

function CategoryForm({ initial, categories, onDone, onBack }: { initial?: FocusCategory; categories: FocusCategory[]; onDone: () => void; onBack: () => void }) {
  const [title, setTitle] = useState(initial?.title ?? "");
  const [icon, setIcon] = useState(initial?.icon ?? "target");
  const [items, setItems] = useState<string[]>([...(initial?.apps ?? []).map((a) => `app:${a}`), ...(initial?.websites ?? []).map((w) => `site:${w}`)]);
  const [error, setError] = useState<string>();
  const apps = useApps();
  const ids = useId();
  const options = useMemo(() => {
    const list = apps.map((a) => appOption(a.path, apps));
    const extra = items.filter((i) => i.startsWith("app:") && !list.some((o) => o.value === i)).map((i) => appOption(i.slice(4), apps));
    return [...list, ...extra];
  }, [apps, items]);
  const iconOptions = Object.entries(CATEGORY_ICONS).map(([value, Glyph]) => ({ value, label: value[0].toUpperCase() + value.slice(1), icon: <Glyph aria-hidden="true" className="size-4 text-fg-60" /> }));

  async function save() {
    if (!title.trim()) {
      setError("Title is required");
      return;
    }
    const category: FocusCategory = {
      id: initial?.id ?? slugify(title, categories.map((c) => c.id)),
      title: title.trim(),
      icon,
      apps: items.filter((i) => i.startsWith("app:")).map((i) => i.slice(4)),
      websites: items.filter((i) => i.startsWith("site:")).map((i) => i.slice(5)),
    };
    try {
      await saveCategories(initial ? categories.map((c) => (c.id === initial.id ? category : c)) : [...categories, category]);
      showToast({ variant: "success", title: initial ? "Updated Focus Category" : "Created Focus Category" });
      onDone();
    } catch (e) {
      fail("Could not save focus category", e);
    }
  }

  const heading = initial ? "Edit Focus Category" : "Create Focus Category";
  return (
    <FormShell title={heading} primaryTitle={initial ? "Save Category" : "Create Category"} onSave={save} onBack={onBack}>
      <label htmlFor={`${ids}-title`} className={label}>
        Title
      </label>
      <div className="flex flex-col gap-1.5">
        <input
          id={`${ids}-title`}
          autoFocus
          spellCheck={false}
          value={title}
          onChange={(e) => setTitle(e.target.value)}
          placeholder="Category name"
          aria-invalid={!!error}
          className={`${field} ${error ? "border-red" : ""}`}
        />
        {error && <p className="text-12 text-red">{error}</p>}
      </div>
      <span id={`${ids}-icon`} className={label}>
        Icon
      </span>
      <Dropdown size="field" labelledBy={`${ids}-icon`} value={icon} options={iconOptions} onChange={setIcon} />
      <span id={`${ids}-items`} className={label}>
        Apps & Websites
      </span>
      <TokenField
        id={`${ids}-items-input`}
        labelledBy={`${ids}-items`}
        value={items}
        onChange={setItems}
        options={options}
        placeholder="Add apps or type a website"
        createLabel="Add website"
      />
    </FormShell>
  );
}

function plural(count: number, word: string) {
  return `${count} ${word}${count === 1 ? "" : "s"}`;
}

function SearchFocusCategories({ pop, startNew = false }: ViewProps & { startNew?: boolean }) {
  const [categories, setCategories] = useState<FocusCategory[] | null>(null);
  const [editing, setEditing] = useState<FocusCategory | "new" | null>(startNew ? "new" : null);

  const reload = useCallback(() => {
    loadCategories()
      .then(setCategories)
      .catch((e) => {
        setCategories([]);
        fail("Could not load focus categories", e);
      });
  }, []);
  useEffect(reload, [reload]);

  const items = useMemo<Item[]>(
    () =>
      (categories ?? []).map((category) => ({
        id: category.id,
        title: category.title,
        keywords: [...category.apps, ...category.websites],
        icon: categoryIcon(category),
        accessory: [category.apps.length && plural(category.apps.length, "app"), category.websites.length && plural(category.websites.length, "website")].filter(Boolean).join(", "),
        actions: [
          [{ id: "edit", title: "Edit Category", icon: Pencil, keepOpen: true, run: () => setEditing(category) }],
          [{ id: "create", title: "Create Focus Category", icon: Plus, shortcut: { key: "n", ctrl: true }, keepOpen: true, run: () => setEditing("new") }],
          [
            {
              id: "delete",
              title: "Delete Category",
              icon: Trash2,
              destructive: true,
              keepOpen: true,
              shortcut: { key: "d", ctrl: true },
              confirm: { title: "Delete Focus Category?", message: `“${category.title}” will be deleted. This can't be undone.`, confirmTitle: "Delete Category" },
              run: async () => {
                await saveCategories((categories ?? []).filter((c) => c.id !== category.id));
                reload();
              },
            },
          ],
        ],
      })),
    [categories, reload],
  );

  if (editing && categories) {
    const done = () => {
      if (startNew) return pop();
      setEditing(null);
      reload();
    };
    return <CategoryForm initial={editing === "new" ? undefined : editing} categories={categories} onDone={done} onBack={startNew ? pop : () => setEditing(null)} />;
  }
  if (startNew) return null;

  return (
    <ListView
      placeholder="Search focus categories…"
      sections={[{ title: "Categories", items }]}
      onBack={pop}
      footerTitle="Search Focus Categories"
      footerIcon={<GlyphTile spec={FOCUS_ICON} small />}
      isLoading={categories === null}
      emptyText="No focus categories yet. Press Ctrl+N to create one."
    />
  );
}

async function importCategories() {
  const [path] = await invoke<string[]>("flowkey_pick_path", { directory: false, multiple: false, extensions: ["json"] });
  if (!path) return;
  let imported: Omit<FocusCategory, "id">[];
  try {
    imported = parseCategoryImport(await invoke<string>("read_json_file", { path }));
  } catch {
    throw new Error("JSON file not in the correct format");
  }
  if (imported.length === 0) throw new Error("No focus categories found to import");
  const categories = await loadCategories();
  const titles = new Set(categories.map((c) => c.title.toLowerCase()));
  const added = imported.filter((c) => !titles.has(c.title.toLowerCase()));
  const next = [...categories];
  for (const category of added) next.push({ ...category, id: slugify(category.title, next.map((c) => c.id)) });
  await saveCategories(next);
  const skipped = imported.length - added.length;
  showToast({ variant: "success", title: `Imported ${added.length} ${added.length === 1 ? "category" : "categories"}`, message: skipped ? `${skipped} already existed` : undefined });
}

const getSession = () => invoke<FocusSession | null>("focus_get");

async function toggleSession() {
  if (await getSession()) {
    await invoke("focus_complete");
    return;
  }
  const form = await loadLastForm();
  await invoke("focus_start", { input: startInput(form, await loadCategories(), await loadPreferences()) });
}

function pauseActions(): Action[] {
  return PAUSE_CHOICES.map(([title, durationMs]) => ({ id: `pause-${title}`, title, run: () => invoke("focus_pause", { durationMs }) }));
}

function sessionSubtitle(session: FocusSession) {
  if (session.status === "paused") return "Paused";
  if (session.durationMs === null) return "Running";
  return `${formatTimer(remainingMs(session, Date.now()))} left`;
}

function sessionCommand(session: FocusSession): Command {
  const paused = session.status === "paused";
  return {
    id: "session",
    title: session.title.trim() || "Focus Session",
    subtitle: sessionSubtitle(session),
    icon: FOCUS_ICON,
    kind: "Focus Session",
    primaryActionTitle: "Edit Session",
    view: EditFocusSession,
    actions: [
      [
        { id: "restart", title: "Restart Session", icon: RotateCcw, run: () => invoke("focus_restart") },
        { id: "complete", title: "Complete Session", icon: CircleCheck, run: () => invoke("focus_complete") },
        paused
          ? { id: "resume", title: "Resume Session", icon: Play, run: () => invoke("focus_resume") }
          : { id: "pause", title: "Pause Session", icon: Pause, submenu: pauseActions, run: () => {} },
        { id: "cancel", title: "Cancel Session", icon: CircleX, destructive: true, run: () => invoke("focus_cancel") },
      ],
    ],
  };
}

function StartFocusSession(props: ViewProps) {
  return <FocusSessionForm {...props} variant="start" />;
}

function EditFocusSession(props: ViewProps) {
  return <FocusSessionForm {...props} variant="edit" />;
}

export default defineExtension({
  id: "focus",
  title: "Focus",
  icon: FOCUS_ICON,
  commands: [
    { id: "start", title: "Start Focus Session", subtitle: "Focus", keywords: ["focus", "block", "distraction"], kind: "Command", primaryActionTitle: "Open Command", view: StartFocusSession },
    { id: "toggle", title: "Toggle Focus Session", subtitle: "Focus", keywords: ["focus"], kind: "Command", primaryActionTitle: "Run Command", run: toggleSession },
    { id: "pause", title: "Pause Focus Session", icon: { glyph: Pause, tone: "green" }, subtitle: "Focus", kind: "Command", primaryActionTitle: "Run Command", run: () => invoke("focus_pause", { durationMs: null }) },
    { id: "resume", title: "Resume Focus Session", icon: { glyph: Play, tone: "green" }, subtitle: "Focus", kind: "Command", primaryActionTitle: "Run Command", run: () => invoke("focus_resume") },
    { id: "complete", title: "Complete Focus Session", icon: { glyph: CircleCheck, tone: "green" }, subtitle: "Focus", kind: "Command", primaryActionTitle: "Run Command", run: () => invoke("focus_complete") },
    { id: "edit", title: "Edit Focus Session", icon: { glyph: Pencil, tone: "green" }, subtitle: "Focus", kind: "Command", primaryActionTitle: "Open Command", view: EditFocusSession },
    { id: "search-categories", title: "Search Focus Categories", icon: { glyph: Search, tone: "green" }, subtitle: "Focus", kind: "Command", primaryActionTitle: "Open Command", view: SearchFocusCategories },
    {
      id: "create-category",
      title: "Create Focus Category",
      icon: { glyph: Plus, tone: "green" },
      subtitle: "Focus",
      keywords: ["new focus category"],
      kind: "Command",
      primaryActionTitle: "Open Command",
      view: (props: ViewProps) => <SearchFocusCategories {...props} startNew />,
    },
    {
      id: "import-categories",
      title: "Import Focus Categories",
      icon: { glyph: FileUp, tone: "green" },
      subtitle: "Focus",
      keywords: ["import focus categories"],
      kind: "Command",
      primaryActionTitle: "Run Command",
      run: importCategories,
    },
  ],
  items: async () => {
    const session = await getSession().catch(() => null);
    return session ? [sessionCommand(session)] : [];
  },
});
