import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  AtSign,
  Clipboard,
  ClipboardList,
  ClipboardPaste,
  Clock,
  ExternalLink,
  File,
  FileText,
  Folder,
  Image,
  Link,
  List,
  ListOrdered,
  Pin,
  PinOff,
  Pipette,
  RotateCcw,
  Trash2,
  Type,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useState, type ReactNode } from "react";
import { fail } from "../ActionHost";
import { colorFormats, parseColor } from "../calculator";
import {
  entryTitle,
  entryType,
  formatBytes,
  groupEntries,
  matchesFilter,
  wordCount,
  type ClipboardEntry,
  type TypeFilter,
} from "../clipboardHistory";
import { Dropdown } from "../Dropdown";
import { GlyphTile } from "../icons";
import { defineExtension, type Action, type IconSpec, type Item, type ViewProps } from "../extension-api";
import { t } from "../i18n";
import { ListView, Metadata } from "../ListView";
import { copyAndClose, frontmostApp, pasteTitle, type AppInfo } from "../output";
import { showHUD, showToast } from "../toast";

const FILTERS: { value: TypeFilter; label: string; icon: ReactNode }[] = [
  { value: "all", label: "All Types", icon: <List aria-hidden="true" className="size-4 shrink-0" /> },
  { value: "text", label: "Text Only", icon: <Type aria-hidden="true" className="size-4 shrink-0" /> },
  { value: "image", label: "Images Only", icon: <Image aria-hidden="true" className="size-4 shrink-0" /> },
  { value: "file", label: "Files Only", icon: <File aria-hidden="true" className="size-4 shrink-0" /> },
  { value: "link", label: "Links Only", icon: <Link aria-hidden="true" className="size-4 shrink-0" /> },
  { value: "email", label: "Emails Only", icon: <AtSign aria-hidden="true" className="size-4 shrink-0" /> },
  { value: "color", label: "Colors Only", icon: <Pipette aria-hidden="true" className="size-4 shrink-0" /> },
];

const ICON: IconSpec = { glyph: ClipboardList, tone: "red" };

const RECENT = [
  ["Last 5 Minutes", 5],
  ["Last 15 Minutes", 15],
  ["Last 30 Minutes", 30],
  ["Last Hour", 60],
  ["Last 24 Hours", 1440],
] as const;

const imageSrc = (entry: ClipboardEntry) => convertFileSrc(entry.id, "clipimg");
const fileIcon = (path: string, size: number) => `${convertFileSrc(path, "appicon")}?size=${Math.round(size * window.devicePixelRatio)}`;

function Tile({ children }: { children: ReactNode }) {
  return <span className="flex size-[22px] shrink-0 items-center justify-center rounded-[5px] bg-key-bg text-fg-60">{children}</span>;
}

function FileIcon({ path }: { path: string }) {
  const [failed, setFailed] = useState(false);
  if (failed)
    return (
      <Tile>
        <File aria-hidden="true" className="size-3.5" />
      </Tile>
    );
  return <img src={fileIcon(path, 22)} alt="" draggable={false} onError={() => setFailed(true)} className="size-[22px] shrink-0" />;
}

function EntryIcon({ entry }: { entry: ClipboardEntry }) {
  const type = entryType(entry);
  if (type === "Image") return <img src={imageSrc(entry)} alt="" draggable={false} className="size-[22px] shrink-0 rounded-[4px] object-cover" />;
  if (type === "File") return <FileIcon path={entry.paths?.[0] ?? ""} />;
  if (type === "Color") {
    const hex = colorFormats(parseColor(entry.text ?? "")!.rgba)[1].value;
    return <span aria-hidden="true" style={{ background: hex }} className="size-[22px] shrink-0 rounded-[5px] shadow-[inset_0_0_0_1px_var(--color-key-border)]" />;
  }
  const Glyph = type === "Link" ? Link : type === "Email" ? AtSign : FileText;
  return (
    <Tile>
      <Glyph aria-hidden="true" className="size-3.5" />
    </Tile>
  );
}

function when(ms: number): string {
  return new Date(ms).toLocaleString("en-US", { dateStyle: "medium", timeStyle: "short" });
}

function Detail({ entry }: { entry: ClipboardEntry }) {
  const type = entryType(entry);
  const text = entry.text ?? "";
  const rows: { label: string; value: ReactNode }[] = [];
  if (entry.sourceName) {
    rows.push({
      label: t("Source"),
      value: (
        <>
          {entry.sourcePath && <img src={fileIcon(entry.sourcePath, 16)} alt="" className="size-4 shrink-0" />}
          <span className="truncate">{entry.sourceName}</span>
        </>
      ),
    });
  }
  rows.push({ label: t("Type"), value: type });
  if (entry.kind === "text") {
    rows.push({ label: t("Characters"), value: [...text].length.toLocaleString("en-US") }, { label: t("Words"), value: wordCount(text).toLocaleString("en-US") });
  } else if (entry.kind === "image") {
    rows.push({ label: t("Dimensions"), value: `${entry.width}×${entry.height}` }, { label: t("Image size"), value: formatBytes(entry.bytes ?? 0) });
  } else {
    rows.push({ label: entry.paths?.length === 1 ? t("Path") : t("Files"), value: <span className="truncate">{entry.paths?.length === 1 ? entry.paths[0] : entry.paths?.length}</span> });
  }
  if (entry.copyCount === 1) rows.push({ label: t("Copied"), value: when(entry.createdAt) });
  else rows.push({ label: t("Times copied"), value: entry.copyCount }, { label: t("Last copied"), value: when(entry.updatedAt) }, { label: t("First copied"), value: when(entry.createdAt) });

  let preview: ReactNode;
  if (entry.kind === "image") preview = <img src={imageSrc(entry)} alt={entryTitle(entry)} className="max-h-full max-w-full object-contain" />;
  else if (entry.kind === "files")
    preview = (
      <ul className="flex flex-col gap-2 self-stretch">
        {entry.paths?.map((path) => (
          <li key={path} className="flex items-center gap-2 font-mono text-13 break-all">
            <img src={fileIcon(path, 16)} alt="" className="size-4 shrink-0" />
            {path}
          </li>
        ))}
      </ul>
    );
  else if (type === "Color")
    preview = <span aria-label={text} style={{ background: colorFormats(parseColor(text)!.rgba)[1].value }} className="size-32 rounded-xl shadow-[inset_0_0_0_1px_var(--color-key-border)]" />;
  else preview = <pre className="self-stretch font-mono text-13 break-all whitespace-pre-wrap text-fg">{text}</pre>;

  return (
    <>
      <div className={`flex min-h-0 flex-1 overflow-y-auto p-4 ${entry.kind === "text" && type !== "Color" ? "" : "items-center justify-center"}`}>{preview}</div>
      <Metadata rows={rows} />
    </>
  );
}

function entryActions(entry: ClipboardEntry, app: AppInfo | null, reload: () => void): Action[][] {
  const type = entryType(entry);
  const text = entry.text ?? "";
  const changed = (title: string) => async (promise: Promise<unknown>) => {
    await promise;
    showToast({ variant: "success", title });
    reload();
  };
  const secondary: Action[] = [];
  if (type === "Color") {
    secondary.push({
      id: "copy-color",
      title: t("Copy Color as…"),
      icon: Clipboard,
      shortcut: { key: "c", ctrl: true, alt: true },
      submenu: () =>
        colorFormats(parseColor(text)!.rgba).map((format) => ({
          id: format.id,
          title: format.label,
          accessory: format.value,
          run: () => copyAndClose(format.value, t("Copied {label} to Clipboard", { label: format.label })),
        })),
      run: () => {},
    });
  }
  if (type === "Link") secondary.push({ id: "open-link", title: t("Open in Browser"), icon: ExternalLink, shortcut: { key: "o", ctrl: true }, run: () => invoke("open_url", { url: text.trim() }) });
  if (type === "Email") secondary.push({ id: "send-email", title: t("Compose Email"), icon: AtSign, shortcut: { key: "o", ctrl: true }, run: () => invoke("open_url", { url: text.trim().startsWith("mailto:") ? text.trim() : `mailto:${text.trim()}` }) });
  if (entry.kind === "files" && entry.paths?.length === 1) {
    secondary.push(
      { id: "open-file", title: t("Open"), icon: ExternalLink, shortcut: { key: "o", ctrl: true }, run: () => invoke("launch_app", { path: entry.paths![0] }) },
      { id: "show-in-explorer", title: t("Show in File Explorer"), icon: Folder, shortcut: { key: "o", ctrl: true, shift: true }, run: () => invoke("show_in_explorer", { path: entry.paths![0] }) },
    );
  }
  secondary.push(
    entry.pinned === null
      ? { id: "pin", title: t("Pin Entry"), icon: Pin, shortcut: { key: ".", ctrl: true }, keepOpen: true, run: () => changed(t("Pinned Entry"))(invoke("clipboard_pin", { id: entry.id, pinned: true })) }
      : { id: "unpin", title: t("Unpin Entry"), icon: PinOff, shortcut: { key: ".", ctrl: true }, keepOpen: true, run: () => changed(t("Unpinned Entry"))(invoke("clipboard_pin", { id: entry.id, pinned: false })) },
  );
  return [
    [
      { id: "paste", title: pasteTitle(app), icon: ClipboardPaste, run: () => invoke("clipboard_paste", { id: entry.id }) },
      {
        id: "copy",
        title: t("Copy to Clipboard"),
        icon: Clipboard,
        shortcut: { key: "Enter", ctrl: true },
        run: async () => {
          await invoke("clipboard_copy", { id: entry.id });
          await invoke("hide_launcher", { restoreFocus: true });
          showHUD({ variant: "success", title: t("Copied Entry to Clipboard") });
        },
      },
    ],
    secondary,
    [
      { id: "delete", title: t("Delete Entry"), icon: Trash2, destructive: true, keepOpen: true, shortcut: { key: "d", ctrl: true }, run: () => changed(t("Deleted Entry"))(invoke("clipboard_delete", { id: entry.id })) },
      {
        id: "delete-recent",
        title: t("Delete Recent Entries…"),
        icon: Clock,
        destructive: true,
        shortcut: { key: "d", ctrl: true, shift: true },
        submenu: () =>
          RECENT.map(([title, minutes]) => ({
            id: `delete-${minutes}`,
            title: t(title),
            keepOpen: true,
            run: () => changed(t("Deleted Recent Entries"))(invoke("clipboard_delete_since", { since: Date.now() - minutes * 60_000 })),
          })),
        run: () => {},
      },
      {
        id: "delete-all",
        title: t("Delete All Entries"),
        icon: Trash2,
        destructive: true,
        keepOpen: true,
        shortcut: { key: "d", ctrl: true, alt: true },
        confirm: { title: t("Delete All Entries?"), message: t("Every entry except pinned ones will be removed from Clipboard History. This can't be undone."), confirmTitle: t("Delete All Entries") },
        run: () => changed(t("Deleted All Entries"))(invoke("clipboard_delete_all")),
      },
    ],
  ];
}

function ClipboardHistory({ pop }: ViewProps) {
  const [entries, setEntries] = useState<ClipboardEntry[] | null>(null);
  const [filter, setFilter] = useState<TypeFilter>("all");
  const [app, setApp] = useState<AppInfo | null>(null);

  const reload = useCallback(() => {
    invoke<ClipboardEntry[]>("clipboard_list")
      .then(setEntries)
      .catch((e) => {
        setEntries([]);
        fail(t("Could not load Clipboard History"), e);
      });
  }, []);

  useEffect(() => {
    reload();
    frontmostApp().then(setApp);
    const unlisten = listen("clipboard-changed", reload);
    return () => {
      unlisten.then((off) => off());
    };
  }, [reload]);

  const sections = useMemo(() => {
    return groupEntries((entries ?? []).filter((e) => matchesFilter(e, filter)), Date.now()).map((group) => ({
      title: t(group.title),
      items: group.entries.map(
        (entry): Item => ({
          id: entry.id,
          title: entryTitle(entry) || t("Empty Text"),
          keywords: [entry.text ?? "", ...(entry.paths ?? [])],
          icon: <EntryIcon entry={entry} />,
          actions: entryActions(entry, app, reload),
        }),
      ),
    }));
  }, [entries, filter, app, reload]);

  const byId = useMemo(() => new Map((entries ?? []).map((e) => [e.id, e])), [entries]);

  return (
    <ListView
      placeholder={t("Type to filter entries…")}
      sections={sections}
      onBack={pop}
      footerTitle={t("Clipboard History")}
      footerIcon={<GlyphTile spec={ICON} small />}
      isLoading={entries === null}
      emptyText={filter === "all" ? t("Copy text, images or files and they will show up here.") : t("No entries of this type.")}
      accessory={
        <div className="w-[197px] shrink-0">
          <span id="clipboard-filter" className="sr-only">
            {t("Filter by Type")}
          </span>
          <Dropdown size="field" labelledBy="clipboard-filter" value={filter} options={FILTERS.map((f) => ({ ...f, label: t(f.label) }))} onChange={setFilter} />
        </div>
      }
      detail={(item) => {
        const entry = byId.get(item.id);
        return entry ? <Detail entry={entry} /> : null;
      }}
    />
  );
}

export default defineExtension({
  id: "clipboard-history",
  title: "Clipboard History",
  icon: ICON,
  commands: [
    {
      id: "history",
      title: "Clipboard History",
      subtitle: "Clipboard History",
      keywords: ["clipboard", "copy", "paste", "history", "histórico", "área de transferência", "colar"],
      kind: "Command",
      primaryActionTitle: "Open Command",
      view: ClipboardHistory,
    },
    {
      id: "paste-sequentially",
      title: "Paste Sequentially",
      icon: { glyph: ListOrdered, tone: ICON.tone },
      subtitle: "Clipboard History",
      keywords: ["clipboard", "paste", "sequence", "next", "colar", "sequência", "próximo"],
      kind: "Command",
      primaryActionTitle: "Run Command",
      run: async () => {
        if (!(await invoke<boolean>("clipboard_paste_next"))) showHUD({ variant: "failure", title: t("No clipboard item found to paste.") });
      },
    },
    {
      id: "reset-paste-sequence",
      title: "Reset Paste Sequence",
      icon: { glyph: RotateCcw, tone: ICON.tone },
      subtitle: "Clipboard History",
      keywords: ["clipboard", "paste", "sequence", "colar", "sequência", "redefinir"],
      kind: "Command",
      primaryActionTitle: "Run Command",
      run: async () => {
        await invoke("clipboard_reset_sequence");
        await invoke("hide_launcher", { restoreFocus: true });
        showHUD({ variant: "success", title: t("Paste sequence reset") });
      },
    },
  ],
});
