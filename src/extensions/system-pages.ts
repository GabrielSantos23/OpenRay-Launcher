import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { Clipboard, Settings } from "lucide-react";
import { defineExtension, type Command } from "../extension-api";
import { copyAndClose } from "../output";

interface SystemPage {
  name: string;
  path: string;
  keywords: string;
  kind: "System Settings" | "Control Panel";
}

const SETTINGS_APP = "shell:AppsFolder\\windows.immersivecontrolpanel_cw5n1h2txyewy!microsoft.windows.immersivecontrolpanel";

export default defineExtension({
  id: "system-pages",
  title: "System Settings",
  icon: { glyph: Settings, tone: "slate" },
  items: async () => {
    const pages = await invoke<SystemPage[]>("list_system_pages");
    const size = Math.round(22 * window.devicePixelRatio);
    return pages.map(
      (page): Command => ({
        id: `${page.path}#${page.name}`,
        title: page.name,
        icon: `${convertFileSrc(page.kind === "System Settings" ? SETTINGS_APP : page.path, "appicon")}?size=${size}`,
        keywords: [page.kind === "System Settings" ? "settings" : "control panel", ...page.keywords.split(";").filter(Boolean)],
        kind: page.kind,
        primaryActionTitle: page.kind === "System Settings" ? "Open Settings Page" : "Open",
        actions: [[], [{ id: "copy-name", title: "Copy Name", icon: Clipboard, shortcut: { key: "c", alt: true, shift: true }, run: () => copyAndClose(page.name, "Copied Name to Clipboard") }]],
        run: () => invoke("launch_app", { path: page.path }),
      }),
    );
  },
});
