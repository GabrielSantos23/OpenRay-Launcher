import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { defineExtension } from "../extension-api";

interface InstalledApp {
  name: string;
  path: string;
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
      run: () => invoke("launch_app", { path: app.path }),
    }));
  },
});
