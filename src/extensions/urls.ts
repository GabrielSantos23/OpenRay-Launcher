import { invoke } from "@tauri-apps/api/core";
import { Globe } from "lucide-react";
import { defineExtension } from "../extension-api";
import { toUrl } from "../urls";

export default defineExtension({
  id: "urls",
  title: "URL",
  icon: { glyph: Globe, tone: "blue" },
  commands: [
    {
      id: "quick-open",
      title: "Open in Browser",
      kind: "Command",
      primaryActionTitle: "Open in Browser",
      query: {
        matches: (text) => toUrl(text) !== null,
        run: (text) => invoke<void>("open_url", { url: toUrl(text), appPath: null }),
      },
    },
  ],
});
