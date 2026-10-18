import { invoke } from "@tauri-apps/api/core";
import { defineExtension } from "../extension-api";

export default defineExtension({
  id: "openray-core",
  title: "Openray",
  commands: [
    {
      id: "quit",
      title: "Quit Openray",
      subtitle: "Openray",
      keywords: ["exit", "close"],
      kind: "Command",
      primaryActionTitle: "Run Command",
      run: () => invoke("quit_app"),
    },
  ],
});
