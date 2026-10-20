import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Archive, Clipboard, Ellipsis, ExternalLink, Eye, File, FileSearch, FileText, Folder, Image, List, Music, PanelRight, RefreshCw, Trash2, Type, Video } from "lucide-react";
import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { fail } from "../ActionHost";
import { formatBytes } from "../clipboardHistory";
import { Dropdown } from "../Dropdown";
import { defineExtension, type Action, type IconSpec, type Item, type ViewProps } from "../extension-api";
import { shortLocation, type FileHit, type FileInfo, type FileResults } from "../fileSearch";
import { GlyphTile } from "../icons";
import { ListView, Metadata } from "../ListView";
import { copyAndClose } from "../output";
import { showHUD, showToast } from "../toast";

const ICON: IconSpec = { glyph: FileSearch, tone: "blue" };
const DETAILS_KEY = "file-search-show-details";

type Kind = "allTypes" | "directory" | "file" | "archive" | "document" | "image" | "video" | "audio" | "text" | "other";

const glyph = (Icon: typeof File) => <Icon aria-hidden="true" className="size-4 shrink-0" />;

const KINDS: { value: Kind; label: string; icon: ReactNode }[] = [
  { value: "allTypes", label: "All Types", icon: glyph(List) },
  { value: "directory", label: "Directory", icon: glyph(Folder) },
  { value: "file", label: "File", icon: glyph(File) },
  { value: "archive", label: "Archive", icon: glyph(Archive) },
  { value: "document", label: "Document", icon: glyph(FileText) },
  { value: "image", label: "Image", icon: glyph(Image) },
  { value: "video", label: "Video", icon: glyph(Video) },
  { value: "audio", label: "Audio", icon: glyph(Music) },
  { value: "text", label: "Text", icon: glyph(Type) },
  { value: "other", label: "Other", icon: glyph(Ellipsis) },
];

const iconSrc = (path: string, size: number, thumb = false) => `${convertFileSrc(path, "appicon")}?size=${Math.round(size * window.devicePixelRatio)}${thumb ? "&thumb=1" : ""}`;

function FileIcon({ path, dir }: { path: string; dir: boolean }) {
  const [failed, setFailed] = useState(false);
  if (failed) return <span className="flex size-[22px] shrink-0 items-center justify-center text-fg-60">{glyph(dir ? Folder : File)}</span>;
  return <img src={iconSrc(path, 22)} alt="" draggable={false} onError={() => setFailed(true)} className="size-[22px] shrink-0" />;
}

function readShowDetails() {
  try {
    return localStorage.getItem(DETAILS_KEY) !== "false";
  } catch {
    return true;
  }
}

function when(ms: number | null): string {
  return ms === null ? "—" : new Date(ms).toLocaleString("en-US", { dateStyle: "medium", timeStyle: "short" });
}

const TEXT_PREVIEW = /\.(txt|md|markdown|log|csv|tsv|json|jsonc|xml|ya?ml|toml|ini|cfg|conf|html?|css|scss|[cm]?jsx?|tsx?|py|rs|go|java|kt|c|h|cpp|hpp|cs|rb|php|swift|sh|ps1|bat|cmd|sql|lua|dart|vue)$/i;
const TEXT_PREVIEW_LIMIT = 1_000_000;

function Detail({ file, home }: { file: FileHit; home: string }) {
  const [info, setInfo] = useState<FileInfo | null>(null);
  const [error, setError] = useState(false);
  const [text, setText] = useState<string | null>(null);
  useEffect(() => {
    let live = true;
    setInfo(null);
    setError(false);
    setText(null);
    invoke<FileInfo>("file_info", { path: file.path })
      .then((result) => {
        if (!live) return;
        setInfo(result);
        if (!result.dir && TEXT_PREVIEW.test(file.path) && (result.size ?? 0) <= TEXT_PREVIEW_LIMIT) {
          invoke<{ content: string }>("flowkey_fs", { op: "readText", path: file.path })
            .then((r) => live && setText(r.content.slice(0, 20_000)))
            .catch(() => {});
        }
      })
      .catch(() => live && setError(true));
    return () => {
      live = false;
    };
  }, [file.path]);

  const rows: { label: string; value: ReactNode }[] = [
    { label: "Name", value: <span className="truncate">{file.name}</span> },
    { label: "Where", value: <span className="truncate">{shortLocation(file.path, home)}</span> },
  ];
  if (info) {
    rows.push({ label: "Type", value: <span className="truncate">{info.typeName}</span> });
    if (info.size !== null) rows.push({ label: "Size", value: formatBytes(info.size) });
    rows.push({ label: "Created", value: when(info.created) }, { label: "Modified", value: when(info.modified) }, { label: "Accessed", value: when(info.accessed) });
  }

  return (
    <>
      <div className="flex min-h-0 flex-1 items-center justify-center p-4">
        {error ? (
          <p className="text-13 text-fg-60">This file is no longer available.</p>
        ) : text !== null ? (
          <pre className="size-full overflow-auto font-mono text-12 break-words whitespace-pre-wrap text-fg">{text}</pre>
        ) : (
          <img key={file.path} src={iconSrc(file.path, 256, true)} alt="" draggable={false} className="max-h-full max-w-full object-contain" />
        )}
      </div>
      <Metadata rows={rows} />
    </>
  );
}

function fileActions(file: FileHit, toggleDetails: () => void, reload: () => void): Action[][] {
  return [
    [
      { id: "open", title: "Open", icon: ExternalLink, run: () => invoke("launch_app", { path: file.path }) },
      { id: "show-in-explorer", title: "Show in File Explorer", icon: Folder, shortcut: { key: "Enter", ctrl: true }, run: () => invoke("show_in_explorer", { path: file.path }) },
      { id: "toggle-details", title: "Toggle Info Panel", icon: PanelRight, shortcut: { key: "i", ctrl: true, shift: true }, keepOpen: true, run: toggleDetails },
    ],
    [
      {
        id: "copy-file",
        title: "Copy File",
        icon: Clipboard,
        shortcut: { key: "c", ctrl: true, shift: true },
        run: async () => {
          await invoke("copy_rich", { paths: [file.path] });
          await invoke("hide_launcher", { restoreFocus: true });
          showHUD({ variant: "success", title: "Copied File to Clipboard" });
        },
      },
      { id: "copy-name", title: "Copy Name", icon: Clipboard, shortcut: { key: "c", ctrl: true, alt: true }, run: () => copyAndClose(file.name, "Copied Name to Clipboard") },
      { id: "copy-path", title: "Copy Path", icon: Clipboard, shortcut: { key: "c", ctrl: true, alt: true, shift: true }, run: () => copyAndClose(`"${file.path}"`, "Copied Path to Clipboard") },
    ],
    [
      {
        id: "delete",
        title: "Delete",
        icon: Trash2,
        destructive: true,
        keepOpen: true,
        shortcut: { key: "d", ctrl: true },
        confirm: { title: "Delete", message: `Are you sure you want to delete ${file.name}? It will be moved to the Recycle Bin.`, confirmTitle: "Delete" },
        run: async () => {
          await invoke("flowkey_fs", { op: "trash", path: file.path });
          showToast({ variant: "success", title: "Moved to Recycle Bin", message: file.name });
          reload();
        },
      },
    ],
    [
      {
        id: "index-files",
        title: "Index Files",
        icon: RefreshCw,
        keepOpen: true,
        run: async () => {
          await invoke("file_reindex");
          reload();
        },
      },
    ],
  ];
}

function SearchFiles({ pop }: ViewProps) {
  const [text, setText] = useState("");
  const [kind, setKind] = useState<Kind>("allTypes");
  const [results, setResults] = useState<FileResults | null>(null);
  const [showDetails, setShowDetails] = useState(readShowDetails);
  const [version, setVersion] = useState(0);
  const latest = useRef(0);

  useEffect(() => {
    const request = ++latest.current;
    invoke<FileResults>("file_search", { query: text, kind })
      .then((result) => request === latest.current && setResults(result))
      .catch((e) => fail("Could not search files", e));
  }, [text, kind, version]);

  useEffect(() => {
    const unlisten = listen("file-index-updated", () => setVersion((v) => v + 1));
    return () => {
      unlisten.then((off) => off());
    };
  }, []);

  const toggleDetails = () =>
    setShowDetails((shown) => {
      try {
        localStorage.setItem(DETAILS_KEY, String(!shown));
      } catch {}
      return !shown;
    });
  const reload = () => setVersion((v) => v + 1);

  const files = results?.files ?? [];
  const home = results?.home ?? "";
  const byPath = useMemo(() => new Map(files.map((file) => [file.path, file])), [files]);
  const sections = useMemo(
    () => [
      {
        title: text.trim() ? "" : "Recently Used",
        items: files.map(
          (file): Item => ({
            id: file.path,
            title: file.name,
            subtitle: shortLocation(file.path, home),
            icon: <FileIcon path={file.path} dir={file.dir} />,
            actions: fileActions(file, toggleDetails, reload),
          }),
        ),
      },
    ],
    [files, home, text],
  );

  const indexingFirst = results?.firstIndex && files.length === 0 && !!text.trim();

  return (
    <ListView
      placeholder="Search files…"
      sections={sections}
      onBack={pop}
      searchText={text}
      onSearchTextChange={setText}
      filtering={false}
      footerTitle="Search Files"
      footerIcon={<GlyphTile spec={ICON} small />}
      isLoading={results === null || results.building}
      emptyText={indexingFirst ? "File Search is indexing…" : text.trim() ? "No Files Found" : "No Recent Files"}
      emptyDescription={indexingFirst ? "Results will appear as soon as your files are indexed." : undefined}
      accessory={
        <div className="w-[197px] shrink-0">
          <span id="file-search-kind" className="sr-only">
            Filter by Type
          </span>
          <Dropdown size="field" labelledBy="file-search-kind" value={kind} options={KINDS} onChange={setKind} />
        </div>
      }
      detail={
        showDetails
          ? (item) => {
              const file = byPath.get(item.id);
              return file ? <Detail file={file} home={home} /> : null;
            }
          : undefined
      }
    />
  );
}

function QuickLook({ pop }: ViewProps) {
  const [paths, setPaths] = useState<string[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    invoke<string[]>("explorer_selection")
      .then(setPaths)
      .catch((e) => {
        setPaths([]);
        setError(String(e));
      });
  }, []);
  const files = useMemo(() => (paths ?? []).map((path): FileHit => ({ path, name: path.split("\\").pop() || path, dir: false })), [paths]);
  const home = useMemo(() => files[0]?.path.match(/^[a-z]:\\users\\[^\\]+/i)?.[0] ?? "", [files]);
  const sections = useMemo(
    () => [{ title: "Selection", items: files.map((file): Item => ({ id: file.path, title: file.name, icon: <FileIcon path={file.path} dir={false} />, actions: fileActions(file, () => {}, () => {}).slice(0, 2) })) }],
    [files],
  );
  return (
    <ListView
      placeholder="Filter selection…"
      sections={sections}
      onBack={pop}
      footerTitle="Quick Look"
      footerIcon={<GlyphTile spec={ICON} small />}
      isLoading={paths === null}
      emptyText={error ?? "Nothing selected"}
      emptyDescription={error ? "Open File Explorer, select a file and run Quick Look again." : undefined}
      detail={(item) => {
        const file = files.find((f) => f.path === item.id);
        return file ? <Detail file={file} home={home} /> : null;
      }}
    />
  );
}

export default defineExtension({
  id: "file-search",
  title: "File Search",
  icon: ICON,
  items: async () => {
    const count = await invoke<number | null>("file_index_status").catch(() => null);
    if (count === null) return [];
    return [
      {
        id: "indexing",
        title: "File Search is indexing…",
        subtitle: `Indexing ${count.toLocaleString("en-US")} files`,
        keywords: ["files", "index", "search"],
        kind: "Command",
        primaryActionTitle: "Open File Search",
        view: SearchFiles,
      },
    ];
  },
  commands: [
    {
      id: "quick-look",
      title: "Quick Look",
      subtitle: "File Search",
      icon: { glyph: Eye, tone: "blue" },
      keywords: ["preview", "explorer", "selection", "peek"],
      kind: "Command",
      primaryActionTitle: "Quick Look",
      view: QuickLook,
    },
    {
      id: "search-files",
      title: "Search Files",
      subtitle: "File Search",
      keywords: ["files", "folders", "find", "explorer", "documents"],
      kind: "Command",
      primaryActionTitle: "Open Command",
      view: SearchFiles,
    },
  ],
});
