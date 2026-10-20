import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import {
  AppWindow,
  Camera,
  Clipboard,
  ClipboardPaste,
  Crop,
  ExternalLink,
  Film,
  Folder,
  FolderOpen,
  History,
  Image,
  ImageUp,
  List,
  Monitor,
  PenTool,
  Pin,
  Pipette,
  QrCode,
  Redo2,
  Ruler,
  ScanLine,
  ScanText,
  Square,
  Trash2,
  Upload,
  Video,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useState, type ReactNode } from "react";
import { fail } from "../ActionHost";
import { Dropdown } from "../Dropdown";
import { defineExtension, type Action, type Command, type IconSpec, type Item, type ViewProps } from "../extension-api";
import { GlyphTile } from "../icons";
import { ListView } from "../ListView";
import { copyAndClose } from "../output";
import { screenshotSections, type Screenshot } from "../screenshots";
import { showHUD, showToast } from "../toast";

const ICON: IconSpec = { glyph: Camera, tone: "blue" };

type Filter = "all" | "images" | "recordings";
const glyph = (Icon: typeof Camera) => <Icon aria-hidden="true" className="size-4 shrink-0" />;
const FILTERS: { value: Filter; label: string; icon: ReactNode }[] = [
  { value: "all", label: "All Media", icon: glyph(List) },
  { value: "images", label: "Screenshots", icon: glyph(Image) },
  { value: "recordings", label: "Recordings", icon: glyph(Video) },
];

const TASKS: { job: string; title: string; glyph: typeof Camera; keywords: string[] }[] = [
  { job: "RectangleRegion", title: "Capture Region", glyph: Crop, keywords: ["screenshot", "area", "selection", "snip"] },
  { job: "PrintScreen", title: "Capture Fullscreen", glyph: Monitor, keywords: ["screenshot", "print screen", "all screens"] },
  { job: "ActiveWindow", title: "Capture Active Window", glyph: AppWindow, keywords: ["screenshot", "window"] },
  { job: "ActiveMonitor", title: "Capture Active Monitor", glyph: Monitor, keywords: ["screenshot", "display", "screen"] },
  { job: "LastRegion", title: "Capture Last Region", glyph: Redo2, keywords: ["screenshot", "repeat", "region"] },
  { job: "ScrollingCapture", title: "Scrolling Capture", glyph: ScanLine, keywords: ["screenshot", "long", "scroll", "page"] },
  { job: "ScreenRecorder", title: "Record Screen", glyph: Video, keywords: ["recording", "video", "mp4", "capture"] },
  { job: "ScreenRecorderGIF", title: "Record Screen as GIF", glyph: Film, keywords: ["recording", "gif", "animation"] },
  { job: "StopScreenRecording", title: "Stop Screen Recording", glyph: Square, keywords: ["recording", "stop", "finish"] },
  { job: "ScreenColorPicker", title: "Pick Color from Screen", glyph: Pipette, keywords: ["color picker", "eyedropper", "hex"] },
  { job: "OCR", title: "Capture Text (OCR)", glyph: ScanText, keywords: ["ocr", "text recognition", "extract text"] },
  { job: "QRCodeScanRegion", title: "Scan QR Code", glyph: QrCode, keywords: ["qr", "barcode", "decode"] },
  { job: "Ruler", title: "Screen Ruler", glyph: Ruler, keywords: ["measure", "pixels", "ruler"] },
  { job: "PinToScreenFromScreen", title: "Pin Region to Screen", glyph: Pin, keywords: ["pin", "float", "overlay"] },
  { job: "ClipboardUpload", title: "Upload Clipboard", glyph: Upload, keywords: ["upload", "share", "clipboard"] },
  { job: "OpenHistory", title: "Open ShareX History", glyph: History, keywords: ["history", "uploads"] },
  { job: "OpenScreenshotsFolder", title: "Open Screenshots Folder", glyph: FolderOpen, keywords: ["folder", "screenshots"] },
];

const runTask = (job: string, file?: string) => invoke<void>("sharex_run", { job, file: file ?? null });

function Thumbnail({ shot }: { shot: Screenshot }) {
  const [failed, setFailed] = useState(false);
  if (failed) return <span className="text-fg-60">{shot.video ? <Video className="size-8" /> : <Image className="size-8" />}</span>;
  return (
    <img
      src={`${convertFileSrc(shot.path, "appicon")}?size=${Math.round(256 * window.devicePixelRatio)}&thumb=1`}
      alt=""
      loading="lazy"
      draggable={false}
      onError={() => setFailed(true)}
      className="size-full object-cover"
    />
  );
}

function shotActions(shot: Screenshot, reload: () => void): Action[][] {
  const copy: Action = {
    id: "copy",
    title: shot.video ? "Copy File" : "Copy to Clipboard",
    icon: Clipboard,
    shortcut: { key: "c", ctrl: true, shift: true },
    run: async () => {
      await invoke(shot.video ? "copy_rich" : "image_copy", shot.video ? { paths: [shot.path] } : { path: shot.path });
      await invoke("hide_launcher", { restoreFocus: true });
      showHUD({ variant: "success", title: "Copied to Clipboard" });
    },
  };
  return [
    [
      copy,
      ...(shot.video ? [] : [{ id: "paste", title: "Paste to Active App", icon: ClipboardPaste, shortcut: { key: "Enter", ctrl: true }, run: () => invoke<void>("image_paste", { path: shot.path }) }]),
      { id: "open", title: "Open", icon: ExternalLink, shortcut: { key: "o", ctrl: true }, run: () => invoke<void>("launch_app", { path: shot.path }) },
      { id: "show-in-explorer", title: "Show in File Explorer", icon: Folder, shortcut: { key: "f", ctrl: true, shift: true }, run: () => invoke<void>("show_in_explorer", { path: shot.path }) },
    ],
    [
      ...(shot.video ? [] : [{ id: "edit", title: "Open in ShareX Editor", icon: PenTool, shortcut: { key: "e", ctrl: true }, run: () => runTask("ImageEditor", shot.path) }]),
      { id: "upload", title: "Upload with ShareX", icon: ImageUp, shortcut: { key: "u", ctrl: true }, run: () => invoke<void>("sharex_upload", { file: shot.path }) },
      ...(shot.video ? [] : [{ id: "pin", title: "Pin to Screen", icon: Pin, shortcut: { key: "p", ctrl: true, shift: true }, run: () => runTask("PinToScreenFromFile", shot.path) }]),
    ],
    [
      { id: "copy-name", title: "Copy Name", icon: Clipboard, shortcut: { key: "c", ctrl: true, alt: true }, run: () => copyAndClose(shot.name, "Copied Name to Clipboard") },
      { id: "copy-path", title: "Copy Path", icon: Clipboard, shortcut: { key: "c", ctrl: true, alt: true, shift: true }, run: () => copyAndClose(shot.path, "Copied Path to Clipboard") },
    ],
    [
      {
        id: "delete",
        title: "Move to Recycle Bin",
        icon: Trash2,
        destructive: true,
        keepOpen: true,
        shortcut: { key: "d", ctrl: true },
        confirm: { title: "Move to Recycle Bin?", message: `${shot.name} will be moved to the Recycle Bin.`, confirmTitle: "Move to Recycle Bin" },
        run: async () => {
          await invoke("flowkey_fs", { op: "trash", path: shot.path });
          showToast({ variant: "success", title: "Moved to Recycle Bin", message: shot.name });
          reload();
        },
      },
    ],
  ];
}

function SearchScreenshots({ pop }: ViewProps) {
  const [shots, setShots] = useState<Screenshot[] | null>(null);
  const [filter, setFilter] = useState<Filter>("all");

  const reload = useCallback(() => {
    invoke<Screenshot[]>("sharex_screenshots")
      .then(setShots)
      .catch((e) => {
        setShots([]);
        fail("Could not load screenshots", e);
      });
  }, []);
  useEffect(reload, [reload]);

  const sections = useMemo(() => {
    const visible = (shots ?? []).filter((s) => filter === "all" || (filter === "recordings") === s.video);
    return screenshotSections(visible).map((section) => ({
      title: section.title,
      items: section.items.map(
        (shot): Item => ({
          id: shot.path,
          title: shot.name,
          subtitle: new Date(shot.modified).toLocaleString("en-US", { dateStyle: "medium", timeStyle: "short" }),
          keywords: [shot.name.split("_")[0], shot.video ? "recording video" : "screenshot image"],
          icon: <Thumbnail shot={shot} />,
          actions: shotActions(shot, reload),
        }),
      ),
    }));
  }, [shots, filter, reload]);

  return (
    <ListView
      placeholder="Search screenshots…"
      sections={sections}
      onBack={pop}
      layout="grid"
      columns={4}
      tileTitles
      footerTitle="Search Screenshots"
      footerIcon={<GlyphTile spec={ICON} small />}
      isLoading={shots === null}
      emptyText="No Screenshots"
      emptyDescription="Screenshots taken with ShareX show up here."
      accessory={
        <div className="w-[180px] shrink-0">
          <span id="screenshots-filter" className="sr-only">
            Filter Media
          </span>
          <Dropdown size="field" labelledBy="screenshots-filter" value={filter} options={FILTERS} onChange={setFilter} />
        </div>
      }
    />
  );
}

export default defineExtension({
  id: "sharex",
  title: "ShareX",
  icon: ICON,
  commands: [
    { id: "search-screenshots", title: "Search Screenshots", subtitle: "ShareX", keywords: ["screenshots", "images", "captures", "recordings"], kind: "Command", primaryActionTitle: "Open Command", view: SearchScreenshots },
    {
      id: "paste-latest",
      title: "Paste Latest Screenshot",
      subtitle: "ShareX",
      icon: { glyph: ClipboardPaste, tone: "blue" },
      keywords: ["screenshot", "paste", "recent", "last"],
      kind: "Command",
      primaryActionTitle: "Paste Latest Screenshot",
      run: () => invoke<void>("sharex_paste_latest"),
    },
    ...TASKS.map(
      (task): Command => ({
        id: task.job,
        title: task.title,
        subtitle: "ShareX",
        icon: { glyph: task.glyph, tone: "blue" },
        keywords: ["sharex", ...task.keywords],
        kind: "Command",
        primaryActionTitle: "Run Command",
        run: () => runTask(task.job),
      }),
    ),
  ],
});
