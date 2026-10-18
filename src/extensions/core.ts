import { invoke } from "@tauri-apps/api/core";
import { Power, Settings } from "lucide-react";
import { defineExtension } from "../extension-api";
import { OpenrayGlyph } from "../icons";

export default defineExtension({
  id: "openray-core",
  title: "Openray",
  icon: { glyph: OpenrayGlyph, tone: "slate" },
  commands: [
    {
      id: "settings",
      title: "Settings",
      icon: { glyph: Settings, tone: "slate" },
      subtitle: "Openray",
      keywords: ["preferences", "hotkey", "options"],
      kind: "Command",
      primaryActionTitle: "Open Command",
      run: () => invoke("open_settings"),
    },
    {
      id: "quit",
      title: "Quit Openray",
      icon: { glyph: Power, tone: "red" },
      subtitle: "Openray",
      keywords: ["exit", "close"],
      kind: "Command",
      primaryActionTitle: "Run Command",
      run: () => invoke("quit_app"),
    },
  ],
});
