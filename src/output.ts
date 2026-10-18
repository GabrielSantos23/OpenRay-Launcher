import { invoke } from "@tauri-apps/api/core";
import { Clipboard, ClipboardPaste } from "lucide-react";
import type { Action } from "./extension-api";
import { showHUD } from "./toast";

export interface AppInfo {
  name: string;
  path: string;
}

export function frontmostApp(): Promise<AppInfo | null> {
  return invoke<AppInfo | null>("frontmost_app").catch(() => null);
}

export async function copyAndClose(text: string, title = "Copied to Clipboard") {
  await invoke("copy_text", { text });
  await invoke("hide_launcher", { restoreFocus: true });
  showHUD({ variant: "success", title });
}

export function pasteTitle(app: AppInfo | null): string {
  return app ? `Paste to ${app.name}` : "Paste";
}

export function copyPasteActions(text: string, app: AppInfo | null, copyTitle: string, pasteFirst = false): Action[] {
  const copy: Action = {
    id: "copy",
    title: copyTitle,
    icon: Clipboard,
    shortcut: pasteFirst ? { key: "Enter", ctrl: true } : { key: "c", ctrl: true, shift: true },
    run: () => copyAndClose(text),
  };
  const paste: Action = {
    id: "paste",
    title: pasteTitle(app),
    icon: ClipboardPaste,
    shortcut: pasteFirst ? undefined : { key: "Enter", ctrl: true },
    run: () => invoke("paste_text", { text }),
  };
  return pasteFirst ? [paste, copy] : [copy, paste];
}
