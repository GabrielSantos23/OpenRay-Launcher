import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { Clipboard, Folder, Shield } from "lucide-react";
import { defineExtension, type Action } from "../extension-api";
import { showToast } from "../toast";

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
  if (app.path.startsWith("shell:AppsFolder\\")) return [[], [copyName]];
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
  ];
}

export default defineExtension({
  id: "applications",
  title: "Applications",
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
