import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import {
  AppWindow,
  Camera,
  Clipboard,
  ClipboardPaste,
  Crop,
  ExternalLink,
  Folder,
  FolderOpen,
  Image,
  Monitor,
  MonitorUp,
  PenTool,
  Pin,
  Pipette,
  QrCode,
  Redo2,
  Ruler,
  ScanText,
  Trash2,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { fail } from "../ActionHost";
import { defineExtension, type Action, type Command, type IconSpec, type Item, type ViewProps } from "../extension-api";
import { GlyphTile } from "../icons";
import { ListView } from "../ListView";
import { copyAndClose } from "../output";
import { screenshotSections, type Screenshot } from "../screenshots";
import { showHUD, showToast } from "../toast";

const ICON: IconSpec = { glyph: Camera, tone: "blue" };

const TASKS: { id: string; title: string; glyph: typeof Camera; keywords: string[]; run: () => Promise<void> }[] = [
  { id: "region", title: "Capture Region", glyph: Crop, keywords: ["screenshot", "area", "selection", "snip", "window"], run: () => invoke("capture_start", { mode: "region" }) },
  { id: "fullscreen", title: "Capture Fullscreen", glyph: MonitorUp, keywords: ["screenshot", "print screen", "all screens"], run: () => invoke("capture_direct", { kind: "fullscreen" }) },
  { id: "window", title: "Capture Active Window", glyph: AppWindow, keywords: ["screenshot", "window"], run: () => invoke("capture_direct", { kind: "window" }) },
  { id: "monitor", title: "Capture Active Monitor", glyph: Monitor, keywords: ["screenshot", "display", "screen"], run: () => invoke("capture_direct", { kind: "monitor" }) },
  { id: "last-region", title: "Capture Last Region", glyph: Redo2, keywords: ["screenshot", "repeat", "region"], run: () => invoke("capture_direct", { kind: "last-region" }) },
  { id: "color", title: "Pick Color from Screen", glyph: Pipette, keywords: ["color picker", "eyedropper", "hex"], run: () => invoke("capture_start", { mode: "color" }) },
  { id: "ocr", title: "Capture Text (OCR)", glyph: ScanText, keywords: ["ocr", "text recognition", "extract text", "copy text"], run: () => invoke("capture_start", { mode: "ocr" }) },
  { id: "qr", title: "Scan QR Code", glyph: QrCode, keywords: ["qr", "barcode", "decode"], run: () => invoke("capture_start", { mode: "qr" }) },
  { id: "pin", title: "Pin Region to Screen", glyph: Pin, keywords: ["pin", "float", "overlay", "sticky"], run: () => invoke("capture_start", { mode: "pin" }) },
  { id: "ruler", title: "Screen Ruler", glyph: Ruler, keywords: ["measure", "pixels", "ruler", "size"], run: () => invoke("capture_start", { mode: "ruler" }) },
];

function Thumbnail({ shot }: { shot: Screenshot }) {
  const [failed, setFailed] = useState(false);
  if (failed) return <Image aria-hidden="true" className="size-8 text-fg-60" />;
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
  const png = shot.path.toLowerCase().endsWith(".png");
  return [
    [
      {
        id: "copy",
        title: "Copy to Clipboard",
        icon: Clipboard,
        shortcut: { key: "c", ctrl: true, shift: true },
        run: async () => {
          await invoke("image_copy", { path: shot.path });
          await invoke("hide_launcher", { restoreFocus: true });
          showHUD({ variant: "success", title: "Copied to Clipboard" });
        },
      },
      { id: "paste", title: "Paste to Active App", icon: ClipboardPaste, shortcut: { key: "Enter", ctrl: true }, run: () => invoke<void>("image_paste", { path: shot.path }) },
      { id: "open", title: "Open", icon: ExternalLink, shortcut: { key: "o", ctrl: true }, run: () => invoke<void>("launch_app", { path: shot.path }) },
      { id: "show-in-explorer", title: "Show in File Explorer", icon: Folder, shortcut: { key: "f", ctrl: true, shift: true }, run: () => invoke<void>("show_in_explorer", { path: shot.path }) },
    ],
    [
      { id: "edit", title: "Edit Image", icon: PenTool, shortcut: { key: "e", ctrl: true }, run: () => invoke<void>("image_edit", { path: shot.path }) },
      ...(png
        ? [
            { id: "pin", title: "Pin to Screen", icon: Pin, shortcut: { key: "p", ctrl: true, shift: true }, run: () => invoke<void>("pin_file", { path: shot.path }) },
            {
              id: "copy-text",
              title: "Copy Text (OCR)",
              icon: ScanText,
              shortcut: { key: "t", ctrl: true, shift: true },
              run: async () => {
                const text = await invoke<string>("image_text", { path: shot.path });
                if (!text.trim()) throw new Error("No text found in this image");
                await copyAndClose(text, "Copied Text to Clipboard");
              },
            },
          ]
        : []),
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

  const reload = useCallback(() => {
    invoke<Screenshot[]>("screenshots_list")
      .then(setShots)
      .catch((e) => {
        setShots([]);
        fail("Could not load screenshots", e);
      });
  }, []);
  useEffect(reload, [reload]);

  const sections = useMemo(
    () =>
      screenshotSections(shots ?? []).map((section) => ({
        title: section.title,
        items: section.items.map(
          (shot): Item => ({
            id: shot.path,
            title: shot.name,
            subtitle: new Date(shot.modified).toLocaleString("en-US", { dateStyle: "medium", timeStyle: "short" }),
            keywords: [shot.name.split("_")[0]],
            icon: <Thumbnail shot={shot} />,
            actions: shotActions(shot, reload),
          }),
        ),
      })),
    [shots, reload],
  );

  const capture: Action = { id: "capture", title: "Capture Region", icon: Crop, shortcut: { key: "n", ctrl: true }, run: () => invoke<void>("capture_start", { mode: "region" }) };

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
      emptyDescription="Screenshots you take with Openray or Win+Print Screen show up here. Press Ctrl+N to capture a region, or change the folder in Settings → Extensions → Screenshots."
      emptyActions={[[capture]]}
    />
  );
}

export default defineExtension({
  id: "screenshots",
  title: "Screenshots",
  icon: ICON,
  commands: [
    { id: "search-screenshots", title: "Search Screenshots", subtitle: "Screenshots", keywords: ["screenshots", "images", "captures"], kind: "Command", primaryActionTitle: "Open Command", view: SearchScreenshots },
    {
      id: "paste-latest",
      title: "Paste Latest Screenshot",
      subtitle: "Screenshots",
      icon: { glyph: ClipboardPaste, tone: "blue" },
      keywords: ["screenshot", "paste", "recent", "last"],
      kind: "Command",
      primaryActionTitle: "Paste Latest Screenshot",
      run: () => invoke("screenshot_paste_latest"),
    },
    {
      id: "open-folder",
      title: "Open Screenshots Folder",
      subtitle: "Screenshots",
      icon: { glyph: FolderOpen, tone: "blue" },
      keywords: ["screenshots", "folder", "pictures"],
      kind: "Command",
      primaryActionTitle: "Open Folder",
      run: async () => {
        const { path } = await invoke<{ path: string }>("screenshots_folder");
        await invoke("flowkey_fs", { op: "mkdir", path });
        await invoke("launch_app", { path });
      },
    },
    ...TASKS.map(
      (task): Command => ({
        id: task.id,
        title: task.title,
        subtitle: "Screenshots",
        icon: { glyph: task.glyph, tone: "blue" },
        keywords: ["capture", ...task.keywords],
        kind: "Command",
        primaryActionTitle: "Run Command",
        run: task.run,
      }),
    ),
  ],
});
