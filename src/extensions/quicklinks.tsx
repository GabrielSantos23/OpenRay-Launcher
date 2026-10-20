import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { Clipboard, CopyPlus, Download, ExternalLink, Link, Link2, Pencil, Plus, Trash2, Upload } from "lucide-react";
import { useCallback, useEffect, useId, useMemo, useRef, useState } from "react";
import { fail, Footer } from "../ActionHost";
import { matchesShortcut } from "../actions";
import { Dropdown } from "../Dropdown";
import { GlyphTile } from "../icons";
import { defineExtension, type Command, type IconSpec, type Item, type ViewProps } from "../extension-api";
import { BackButton, ListView, Metadata } from "../ListView";
import { copyAndClose } from "../output";
import { displayLink, exportQuicklinks, faviconUrl, fillLink, linkArguments, mergeQuicklinks, parseQuicklinkImport, type Quicklink } from "../quicklinks";
import { showToast } from "../toast";

const ICON: IconSpec = { glyph: Link2, tone: "purple" };

const BROWSERS = /\b(chrome|chromium|edge|firefox|brave|opera|vivaldi|helium|arc|zen|librewolf|waterfox|thorium|floorp|tor browser|zen browser)\b/i;

export async function loadQuicklinks(): Promise<Quicklink[]> {
  const value = await invoke<Quicklink[] | null>("data_get", { name: "quicklinks" });
  return Array.isArray(value) ? value : [];
}

function saveQuicklinks(list: Quicklink[]) {
  return invoke("data_set", { name: "quicklinks", value: list });
}

async function openQuicklink(quicklink: Quicklink, values: Record<string, string>) {
  const missing = linkArguments(quicklink.link).find((arg) => !arg.defaultValue && !values[arg.name]?.trim());
  if (missing) throw new Error(`Enter a value for ${missing.name}`);
  const clipboard = /\{clipboard/i.test(quicklink.link) ? await invoke<string>("read_clipboard_text").catch(() => "") : undefined;
  await invoke("open_url", { url: fillLink(quicklink.link, values, { clipboard }), appPath: quicklink.appPath ?? null });
}

function QuicklinkIcon({ link, size = 22 }: { link: string; size?: number }) {
  const [failed, setFailed] = useState(false);
  const src = faviconUrl(link);
  if (src && !failed) {
    return <img src={src} alt="" width={size} height={size} draggable={false} onError={() => setFailed(true)} className="shrink-0 rounded-[5px]" />;
  }
  return (
    <span aria-hidden="true" style={{ width: size, height: size }} className="flex shrink-0 items-center justify-center rounded-[5px] bg-key-bg text-fg-60">
      <Link className="size-3.5" />
    </span>
  );
}

interface BrowserOption {
  value: string;
  label: string;
}

function useBrowsers(): BrowserOption[] {
  const [browsers, setBrowsers] = useState<BrowserOption[]>([]);
  useEffect(() => {
    invoke<{ name: string; path: string }[]>("list_apps")
      .then((apps) => setBrowsers(apps.filter((app) => BROWSERS.test(app.name) && !app.path.startsWith("shell:")).map((app) => ({ value: app.path, label: app.name }))))
      .catch((e) => console.error("list_apps failed", e));
  }, []);
  return browsers;
}

function QuicklinkForm({ initial, initialLink, onDone, onBack }: { initial?: Quicklink; initialLink?: string; onDone: () => void; onBack: () => void }) {
  const [link, setLink] = useState(initial?.link ?? initialLink ?? "");
  const [name, setName] = useState(initial?.name ?? "");
  const [appPath, setAppPath] = useState(initial?.appPath ?? "");
  const [errors, setErrors] = useState<{ link?: string; name?: string }>({});
  const linkRef = useRef<HTMLTextAreaElement>(null);
  const nameRef = useRef<HTMLInputElement>(null);
  const browsers = useBrowsers();
  const ids = useId();
  const options = useMemo(() => {
    const list = [{ value: "", label: "Default Browser" }, ...browsers];
    if (initial?.appPath && !list.some((o) => o.value === initial.appPath)) list.push({ value: initial.appPath, label: initial.appName ?? initial.appPath });
    return list.map((o) => ({ ...o, icon: <AppIcon path={o.value} /> }));
  }, [browsers, initial]);
  const title = initial ? "Edit Quicklink" : "Create Quicklink";

  async function save() {
    const next = { link: link.trim() ? undefined : "Link is required", name: name.trim() ? undefined : "Name is required" };
    setErrors(next);
    if (next.link) return linkRef.current?.focus();
    if (next.name) return nameRef.current?.focus();
    try {
      const list = await loadQuicklinks();
      const app = options.find((o) => o.value === appPath);
      const quicklink: Quicklink = {
        id: initial?.id ?? crypto.randomUUID(),
        name: name.trim(),
        link: link.trim(),
        ...(appPath ? { appPath, appName: app?.label } : {}),
      };
      const exists = list.some((q) => q.id === quicklink.id);
      await saveQuicklinks(exists ? list.map((q) => (q.id === quicklink.id ? quicklink : q)) : [...list, quicklink]);
      showToast({ variant: "success", title: initial ? "Updated Quicklink" : "Created Quicklink" });
      onDone();
    } catch (e) {
      fail("Could not save Quicklink", e);
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
  return (
    <div className="flex min-h-0 flex-1 flex-col" onKeyDown={onKeyDown}>
      <header className="flex h-header shrink-0 items-center gap-3 border-b border-separator px-search-inset">
        <BackButton onBack={onBack} />
        <h2 className="text-13 font-medium text-fg-60">{title}</h2>
      </header>
      <form
        className="grid min-h-0 flex-1 auto-rows-min grid-cols-[175px_360px] content-start gap-x-5 gap-y-5 overflow-y-auto pt-1.5"
        onSubmit={(e) => {
          e.preventDefault();
          save();
        }}
      >
        <label htmlFor={`${ids}-link`} className="pt-2 text-right text-13 text-fg-60">
          Link
        </label>
        <div className="flex flex-col gap-1.5">
          <textarea
            id={`${ids}-link`}
            ref={linkRef}
            autoFocus
            spellCheck={false}
            value={link}
            onChange={(e) => setLink(e.target.value)}
            placeholder="https://www.google.com/search?q={argument}"
            aria-invalid={!!errors.link}
            aria-describedby={`${ids}-link-help`}
            className={`${field} h-[100px] resize-none py-2 ${errors.link ? "border-red" : "border-key-border focus:border-fg-40"}`}
          />
          <p id={`${ids}-link-help`} className={`text-12 ${errors.link ? "text-red" : "text-fg-60"}`}>
            {errors.link ?? "Use {argument} to ask for text when you open it, or {clipboard}, {date}, {time} and {uuid} to insert them."}
          </p>
        </div>
        <label htmlFor={`${ids}-name`} className="pt-2 text-right text-13 text-fg-60">
          Name
        </label>
        <div className="flex flex-col gap-1.5">
          <input
            id={`${ids}-name`}
            ref={nameRef}
            spellCheck={false}
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="Quicklink name"
            aria-invalid={!!errors.name}
            className={`${field} h-[34px] ${errors.name ? "border-red" : "border-key-border focus:border-fg-40"}`}
          />
          {errors.name && <p className="text-12 text-red">{errors.name}</p>}
        </div>
        <span id={`${ids}-open-with`} className="pt-2 text-right text-13 text-fg-60">
          Open With
        </span>
        <Dropdown size="field" labelledBy={`${ids}-open-with`} value={appPath} options={options} onChange={setAppPath} />
      </form>
      <Footer
        left={
          <span className="flex items-center gap-2 text-fg-60">
            <GlyphTile spec={ICON} small />
            {title}
          </span>
        }
        primaryTitle="Save Quicklink"
        primaryKeys={["Ctrl", "↵"]}
        showActions={false}
        panelOpen={false}
        onTogglePanel={() => {}}
      />
    </div>
  );
}

function AppIcon({ path }: { path: string }) {
  const [failed, setFailed] = useState(false);
  if (!path || failed) return <ExternalLink aria-hidden="true" className="size-4 shrink-0 text-fg-60" />;
  return (
    <img
      src={`${convertFileSrc(path, "appicon")}?size=${Math.round(16 * window.devicePixelRatio)}`}
      alt=""
      onError={() => setFailed(true)}
      className="size-4 shrink-0"
    />
  );
}

function SearchQuicklinks({ pop, selectId }: ViewProps & { selectId?: string }) {
  const [list, setList] = useState<Quicklink[] | null>(null);
  const [editing, setEditing] = useState<Quicklink | "new" | null>(null);
  const [current, setCurrent] = useState<Item | undefined>();
  const [values, setValues] = useState<Record<string, Record<string, string>>>({});

  const reload = useCallback(() => {
    loadQuicklinks()
      .then(setList)
      .catch((e) => {
        setList([]);
        fail("Could not load Quicklinks", e);
      });
  }, []);
  useEffect(reload, [reload]);

  const selected = list?.find((q) => q.id === current?.id);
  const firstArgument = selected ? linkArguments(selected.link)[0] : undefined;

  const items = useMemo<Item[]>(
    () =>
      (list ?? []).map((quicklink) => ({
        id: quicklink.id,
        title: quicklink.name,
        keywords: [quicklink.link],
        icon: <QuicklinkIcon link={quicklink.link} />,
        actions: [
          [
            { id: "open", title: "Open Quicklink", icon: ExternalLink, run: () => openQuicklink(quicklink, values[quicklink.id] ?? {}) },
            {
              id: "copy-link",
              title: "Copy Link",
              icon: Clipboard,
              shortcut: { key: "c", ctrl: true, shift: true },
              run: () => copyAndClose(fillLink(quicklink.link, values[quicklink.id] ?? {}), "Copied Link to Clipboard"),
            },
          ],
          [
            { id: "edit", title: "Edit Quicklink", icon: Pencil, shortcut: { key: "e", ctrl: true }, keepOpen: true, run: () => setEditing(quicklink) },
            {
              id: "duplicate",
              title: "Duplicate Quicklink",
              icon: CopyPlus,
              keepOpen: true,
              run: () => setEditing({ ...quicklink, id: crypto.randomUUID(), name: `${quicklink.name} Copy` }),
            },
            { id: "create", title: "Create Quicklink", icon: Plus, shortcut: { key: "n", ctrl: true }, keepOpen: true, run: () => setEditing("new") },
          ],
          [
            {
              id: "delete",
              title: "Delete Quicklink",
              icon: Trash2,
              destructive: true,
              keepOpen: true,
              shortcut: { key: "d", ctrl: true },
              confirm: { title: "Delete Quicklink?", message: `“${quicklink.name}” will be deleted. This can't be undone.`, confirmTitle: "Delete Quicklink" },
              run: async () => {
                await saveQuicklinks((list ?? []).filter((q) => q.id !== quicklink.id));
                reload();
              },
            },
          ],
        ],
      })),
    [list, values, reload],
  );

  if (editing) {
    const done = () => {
      setEditing(null);
      reload();
    };
    return <QuicklinkForm initial={editing === "new" ? undefined : editing} onDone={done} onBack={() => setEditing(null)} />;
  }

  return (
    <ListView
      placeholder="Search for Quicklinks…"
      sections={[{ title: "Quicklinks", items }]}
      onBack={pop}
      footerTitle="Search Quicklinks"
      footerIcon={<GlyphTile spec={ICON} small />}
      isLoading={list === null}
      emptyText="No Quicklinks yet. Press Ctrl+N to create one."
      emptyActions={[[{ id: "create", title: "Create Quicklink", icon: Plus, shortcut: { key: "n", ctrl: true }, keepOpen: true, run: () => setEditing("new") }]]}
      selectedId={selectId}
      onSelectionChange={setCurrent}
      argument={
        selected && firstArgument
          ? {
              placeholder: firstArgument.name,
              value: values[selected.id]?.[firstArgument.name] ?? "",
              onChange: (value) => setValues((v) => ({ ...v, [selected.id]: { ...v[selected.id], [firstArgument.name]: value } })),
            }
          : undefined
      }
      detail={(item) => {
        const quicklink = list?.find((q) => q.id === item.id);
        if (!quicklink) return null;
        const preview = displayLink(quicklink.link, values[quicklink.id]);
        return (
          <>
            <div className="min-h-0 flex-1 overflow-y-auto p-4 font-mono text-13 break-all whitespace-pre-wrap text-fg">{preview}</div>
            <Metadata
              rows={[
                { label: "Name", value: quicklink.name },
                { label: "Link", value: <span className="truncate">{quicklink.link}</span> },
                { label: "Open With", value: quicklink.appName ?? "Default Browser" },
              ]}
            />
          </>
        );
      }}
    />
  );
}

function CreateQuicklink({ pop }: ViewProps) {
  return <QuicklinkForm onDone={pop} onBack={pop} />;
}

async function importQuicklinks() {
  const [path] = await invoke<string[]>("flowkey_pick_path", { directory: false, multiple: false, extensions: ["json"] });
  if (!path) return;
  let incoming: ReturnType<typeof parseQuicklinkImport>;
  try {
    incoming = parseQuicklinkImport(await invoke<string>("read_json_file", { path }));
  } catch {
    throw new Error("JSON file not in the correct format");
  }
  const apps = await invoke<{ name: string; path: string }[]>("list_apps").catch(() => []);
  const { list, added, skipped } = mergeQuicklinks(await loadQuicklinks(), incoming, apps, () => crypto.randomUUID());
  await saveQuicklinks(list);
  const plural = (n: number, word: string) => `${n} ${word}${n === 1 ? "" : "s"}`;
  showToast({ variant: "success", title: "Import successful", message: [added > 0 && `${plural(added, "quicklink")} added`, skipped > 0 && `${plural(skipped, "duplicate")} skipped`].filter(Boolean).join(", ") || undefined });
}

async function exportAll() {
  const list = await loadQuicklinks();
  if (list.length === 0) throw new Error("There are no quicklinks to export");
  const [folder] = await invoke<string[]>("flowkey_pick_path", { directory: true, multiple: false });
  if (!folder) return;
  const date = new Date().toLocaleDateString("en-CA");
  const path = `${folder.replace(/\\$/, "")}\\Quicklinks ${date}.json`;
  await invoke("flowkey_fs", { op: "writeText", path, content: exportQuicklinks(list) });
  await invoke("show_in_explorer", { path });
  showToast({ variant: "success", title: "Exported quicklinks" });
}

function quicklinkCommand(quicklink: Quicklink): Command {
  const args = linkArguments(quicklink.link);
  const icon = faviconUrl(quicklink.link) ?? ICON;
  const base: Command = {
    id: quicklink.id,
    title: quicklink.name,
    subtitle: "Quicklink",
    icon,
    keywords: [quicklink.link],
    kind: "Quicklink",
    primaryActionTitle: "Open Quicklink",
  };
  if (args.length === 0) return { ...base, run: () => openQuicklink(quicklink, {}) };
  return {
    ...base,
    view: (props: ViewProps) => <SearchQuicklinks {...props} selectId={quicklink.id} />,
    fallback: (text) => openQuicklink(quicklink, { [args[0].name]: text }),
  };
}

export default defineExtension({
  id: "quicklinks",
  title: "Quicklinks",
  icon: ICON,
  commands: [
    {
      id: "create",
      title: "Create Quicklink",
      subtitle: "Quicklinks",
      keywords: ["link", "url", "bookmark"],
      kind: "Command",
      primaryActionTitle: "Open Command",
      view: CreateQuicklink,
      fallbackView: (text) => (props: ViewProps) => <QuicklinkForm initialLink={text} onDone={props.pop} onBack={props.pop} />,
    },
    {
      id: "search",
      title: "Search Quicklinks",
      subtitle: "Quicklinks",
      keywords: ["link", "url", "bookmark"],
      kind: "Command",
      primaryActionTitle: "Open Command",
      view: SearchQuicklinks,
    },
    {
      id: "import",
      title: "Import Quicklinks",
      subtitle: "Quicklinks",
      icon: { glyph: Download, tone: "purple" },
      keywords: ["link", "json", "import"],
      kind: "Command",
      primaryActionTitle: "Run Command",
      run: importQuicklinks,
    },
    {
      id: "export",
      title: "Export Quicklinks",
      subtitle: "Quicklinks",
      icon: { glyph: Upload, tone: "purple" },
      keywords: ["link", "json", "export", "backup"],
      kind: "Command",
      primaryActionTitle: "Run Command",
      run: exportAll,
    },
  ],
  items: async () => (await loadQuicklinks()).map(quicklinkCommand),
});
