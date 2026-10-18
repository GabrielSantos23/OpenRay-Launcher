import { invoke } from "@tauri-apps/api/core";
import { Search } from "lucide-react";
import { defineExtension, type Command } from "../extension-api";

const ENGINES = [
  { id: "google", title: "Google Search", home: "https://www.google.com", search: "https://www.google.com/search?q=", domain: "google.com" },
  { id: "duckDuckGo", title: "DuckDuckGo Search", home: "https://duckduckgo.com", search: "https://duckduckgo.com/?q=", domain: "duckduckgo.com" },
  { id: "bing", title: "Bing", home: "https://www.bing.com", search: "https://www.bing.com/search?q=", domain: "bing.com" },
];

export default defineExtension({
  id: "web-searches",
  title: "Web Searches",
  icon: { glyph: Search, tone: "green" },
  commands: ENGINES.map(
    (engine): Command => ({
      id: engine.id,
      title: engine.title,
      subtitle: "Web Searches",
      icon: `https://www.google.com/s2/favicons?domain=${engine.domain}&sz=64`,
      keywords: ["search", "web"],
      kind: "Command",
      primaryActionTitle: "Open Command",
      run: () => invoke("open_url", { url: engine.home }),
      fallback: (text) => invoke("open_url", { url: engine.search + encodeURIComponent(text) }),
    }),
  ),
});
