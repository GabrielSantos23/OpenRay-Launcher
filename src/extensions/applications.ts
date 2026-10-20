import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { Clipboard, Folder, LayoutGrid, Shield, Trash2 } from "lucide-react";
import { defineExtension, type Action } from "../extension-api";
import { showHUD, showToast } from "../toast";

interface InstalledApp {
  name: string;
  path: string;
}

function appActions(app: InstalledApp): Action[][] {
  const copy = (text: string, what: string) => async () => {
    await navigator.clipboard.writeText(text);
    showToast({ variant: "success", title: `Copied ${what} to clipboard` });
  };
  const copyName: Action = { id: "copy-name", title: "Copy Name", icon: Clipboard, keepOpen: true, run: copy(app.name, "name") };
  const uninstall: Action = {
    id: "uninstall",
    title: "Uninstall Application",
    icon: Trash2,
    destructive: true,
    confirm: async () => {
      const target = await invoke<string | null>("app_uninstaller", { name: app.name, path: app.path });
      if (target === null) return { title: "No Uninstaller Found", message: `Windows has no uninstaller registered for ${app.name}. Open Settings › Installed apps to remove it there.`, confirmTitle: "Open Settings" };
      return { title: `Uninstall ${app.name}?`, message: target ? `This starts the uninstaller registered for “${target}”.` : `${app.name} will be removed from this PC.`, confirmTitle: "Uninstall" };
    },
    run: async () => {
      const result = await invoke<string>("app_uninstall", { name: app.name, path: app.path });
      if (result === "removed") showHUD({ variant: "success", title: `Uninstalled ${app.name}` });
      else if (result === "requested") showHUD({ variant: "success", title: `Uninstall requested for ${app.name}` });
    },
  };
  if (app.path.startsWith("shell:AppsFolder\\")) return [[], [copyName], [uninstall]];
  return [
    [
      {
        id: "run-as-admin",
        title: "Run as Administrator",
        icon: Shield,
        shortcut: { key: "Enter", ctrl: true, shift: true },
        run: () => invoke("launch_app", { path: app.path, asAdmin: true }),
      },
      { id: "show-in-explorer", title: "Show in File Explorer", icon: Folder, run: () => invoke("show_in_explorer", { path: app.path }) },
    ],
    [
      {
        id: "copy-path",
        title: "Copy Path",
        icon: Clipboard,
        shortcut: { key: "c", ctrl: true, alt: true },
        keepOpen: true,
        run: copy(app.path, "path"),
      },
      copyName,
    ],
    [uninstall],
  ];
}

export default defineExtension({
  id: "applications",
  title: "Applications",
  icon: { glyph: LayoutGrid, tone: "blue" },
  items: async () => {
    const apps = await invoke<InstalledApp[]>("list_apps");
    const iconSize = Math.round(22 * window.devicePixelRatio);
    return apps.map((app) => ({
      id: app.path,
      title: app.name,
      icon: `${convertFileSrc(app.path, "appicon")}?size=${iconSize}`,
      kind: "Application",
      primaryActionTitle: "Open Application",
      actions: appActions(app),
      run: () => invoke("launch_app", { path: app.path }),
    }));
  },
});
