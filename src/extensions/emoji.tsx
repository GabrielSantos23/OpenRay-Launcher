import { invoke } from "@tauri-apps/api/core";
import { Clipboard, ClipboardPaste, Clock, Flag, Gamepad2, Hash, Lightbulb, Plane, PawPrint, Smile, UtensilsCrossed, LayoutGrid } from "lucide-react";
import { useCallback, useEffect, useMemo, useState, type ReactNode } from "react";
import { Dropdown } from "../Dropdown";
import { GlyphTile } from "../icons";
import { defineExtension, type IconSpec, type Item, type ViewProps } from "../extension-api";
import { ListView, type ListSection } from "../ListView";
import { copyAndClose, frontmostApp, pasteTitle, type AppInfo } from "../output";
import { matchScore } from "../search";

interface Emoji {
  emoji: string;
  name: string;
  category: string;
}

interface Usage {
  [emoji: string]: { count: number; last: number };
}

const FREQUENT_LIMIT = 16;
const ICON: IconSpec = { glyph: Smile, tone: "yellow" };
const icon = (Glyph: typeof Smile) => <Glyph aria-hidden="true" className="size-4 shrink-0" />;

const CATEGORIES: { value: string; label: string; groups: string[]; icon: ReactNode }[] = [
  { value: "smileys_people", label: "Smileys & People", groups: ["smileys_emotion", "people_body"], icon: icon(Smile) },
  { value: "animals_nature", label: "Animals & Nature", groups: ["animals_nature"], icon: icon(PawPrint) },
  { value: "food_drink", label: "Food & Drink", groups: ["food_drink"], icon: icon(UtensilsCrossed) },
  { value: "activity", label: "Activity", groups: ["activities"], icon: icon(Gamepad2) },
  { value: "travel_places", label: "Travel & Places", groups: ["travel_places"], icon: icon(Plane) },
  { value: "objects", label: "Objects", groups: ["objects"], icon: icon(Lightbulb) },
  { value: "symbols", label: "Symbols", groups: ["symbols"], icon: icon(Hash) },
  { value: "flags", label: "Flags", groups: ["flags"], icon: icon(Flag) },
];

const FILTERS = [
  { value: "all", label: "All Categories", icon: icon(LayoutGrid) },
  { value: "recent", label: "Frequently Used", icon: icon(Clock) },
  ...CATEGORIES.map(({ value, label, icon }) => ({ value, label, icon })),
];

let catalog: Promise<Emoji[]> | null = null;

function loadCatalog(): Promise<Emoji[]> {
  catalog ??= import("unicode-emoji-json/data-by-group.json").then(({ default: groups }) =>
    (groups as { slug: string; emojis: { emoji: string; name: string }[] }[]).flatMap((group) => {
      const category = CATEGORIES.find((c) => c.groups.includes(group.slug));
      return category ? group.emojis.map((e) => ({ emoji: e.emoji, name: e.name, category: category.value })) : [];
    }),
  );
  return catalog;
}

const titleCase = (name: string) => name.replace(/(^|[\s-])(\p{L})/gu, (_, sep: string, letter: string) => sep + letter.toUpperCase());

export function frequent(usage: Usage, limit = FREQUENT_LIMIT): string[] {
  return Object.entries(usage)
    .sort(([, a], [, b]) => b.count - a.count || b.last - a.last)
    .slice(0, limit)
    .map(([emoji]) => emoji);
}

function SearchEmoji({ pop }: ViewProps) {
  const [emojis, setEmojis] = useState<Emoji[] | null>(null);
  const [usage, setUsage] = useState<Usage>({});
  const [text, setText] = useState("");
  const [category, setCategory] = useState("all");
  const [current, setCurrent] = useState<Item | undefined>();
  const [app, setApp] = useState<AppInfo | null>(null);

  useEffect(() => {
    loadCatalog().then(setEmojis);
    invoke<Usage | null>("data_get", { name: "emoji" })
      .then((value) => setUsage(value && typeof value === "object" ? value : {}))
      .catch(() => setUsage({}));
    frontmostApp().then(setApp);
  }, []);

  const record = useCallback(
    async (emoji: string) => {
      const next = { ...usage, [emoji]: { count: (usage[emoji]?.count ?? 0) + 1, last: Date.now() } };
      await invoke("data_set", { name: "emoji", value: next }).catch((e) => console.error("could not save emoji usage", e));
    },
    [usage],
  );

  const toItem = useCallback(
    (emoji: Emoji, section: string): Item => ({
      id: `${section}:${emoji.emoji}`,
      title: titleCase(emoji.name),
      keywords: [emoji.name],
      icon: <span className="font-emoji">{emoji.emoji}</span>,
      actions: [
        [
          {
            id: "paste",
            title: pasteTitle(app),
            icon: ClipboardPaste,
            run: async () => {
              await record(emoji.emoji);
              await invoke("paste_text", { text: emoji.emoji });
            },
          },
          {
            id: "copy",
            title: "Copy to Clipboard",
            icon: Clipboard,
            shortcut: { key: "Enter", ctrl: true },
            run: async () => {
              await record(emoji.emoji);
              await copyAndClose(emoji.emoji, "Copied Emoji to Clipboard");
            },
          },
        ],
      ],
    }),
    [app, record],
  );

  const sections = useMemo<ListSection[]>(() => {
    if (!emojis) return [];
    if (text.trim()) {
      const pool = category === "all" || category === "recent" ? emojis : emojis.filter((e) => e.category === category);
      const results = pool
        .map((e) => ({ e, score: matchScore(text, e.name) }))
        .filter((m) => m.score > 0)
        .sort((a, b) => b.score - a.score)
        .map((m) => toItem(m.e, "results"));
      return results.length ? [{ title: "Results", items: results }] : [];
    }
    const byEmoji = new Map(emojis.map((e) => [e.emoji, e]));
    const recent = frequent(usage).flatMap((emoji) => (byEmoji.has(emoji) ? [toItem(byEmoji.get(emoji)!, "recent")] : []));
    const recentSection = { title: "Frequently Used", items: recent, showCount: true };
    if (category === "recent") return [recentSection];
    const categories = CATEGORIES.filter((c) => category === "all" || c.value === category).map((c) => ({
      title: c.label,
      items: emojis.filter((e) => e.category === c.value).map((e) => toItem(e, c.value)),
      showCount: true,
    }));
    return [...(category === "all" && recent.length ? [recentSection] : []), ...categories];
  }, [emojis, usage, text, category, toItem]);

  return (
    <ListView
      placeholder="Search Emoji & Symbols…"
      sections={sections}
      onBack={pop}
      searchText={text}
      onSearchTextChange={setText}
      filtering={false}
      layout="grid"
      columns={8}
      footerTitle={current ? `Search Emoji & Symbols – ${current.title}` : "Search Emoji & Symbols"}
      footerIcon={<GlyphTile spec={ICON} small />}
      isLoading={emojis === null}
      emptyText="No emoji found"
      onSelectionChange={setCurrent}
      accessory={
        <div className="w-[197px] shrink-0">
          <span id="emoji-category" className="sr-only">
            Filter by Category
          </span>
          <Dropdown size="field" labelledBy="emoji-category" value={category} options={FILTERS} onChange={setCategory} />
        </div>
      }
    />
  );
}

export default defineExtension({
  id: "emoji-picker",
  title: "Emoji & Symbols",
  icon: ICON,
  commands: [
    {
      id: "searchEmoji",
      title: "Search Emoji & Symbols",
      subtitle: "Emoji & Symbols",
      keywords: ["emoji", "symbols", "smiley"],
      kind: "Command",
      primaryActionTitle: "Open Command",
      view: SearchEmoji,
    },
  ],
});
