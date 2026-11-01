import { invoke } from "@tauri-apps/api/core";
import { Clipboard, Menu, Play } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { fail } from "../ActionHost";
import { defineExtension, type IconSpec, type Item, type ViewProps } from "../extension-api";
import { GlyphTile } from "../icons";
import { t } from "../i18n";
import { ListView } from "../ListView";
import { copyAndClose } from "../output";

const ICON: IconSpec = { glyph: Menu, tone: "slate" };

interface MenuItem {
  id: number;
  title: string;
  path: string[];
  shortcut: string;
  checked: boolean;
}

function SearchMenuItems({ pop }: ViewProps) {
  const [data, setData] = useState<{ app: string; items: MenuItem[] } | null>(null);

  useEffect(() => {
    invoke<[string, MenuItem[]]>("menu_items_list")
      .then(([app, items]) => setData({ app, items }))
      .catch((e) => {
        setData({ app: "", items: [] });
        fail(t("Could not read menu items"), e);
      });
  }, []);

  const sections = useMemo(() => {
    const groups = new Map<string, Item[]>();
    for (const item of data?.items ?? []) {
      const section = item.path[0] ?? t("Menu");
      const location = item.path.slice(1).join(" › ");
      const entry: Item = {
        id: `${item.id}:${item.path.join("/")}:${item.title}`,
        title: item.checked ? `${item.title} ✓` : item.title,
        subtitle: location || undefined,
        keywords: item.path,
        icon: <span className="flex size-[22px] shrink-0 items-center justify-center text-fg-60"><Menu aria-hidden="true" className="size-4" /></span>,
        accessory: item.shortcut ? <kbd className="font-sans text-12 text-fg-60">{item.shortcut}</kbd> : undefined,
        actions: [
          [
            { id: "run", title: t("Run Menu Item"), icon: Play, run: () => invoke<void>("menu_items_invoke", { id: item.id }) },
            { id: "copy", title: t("Copy Menu Path"), icon: Clipboard, shortcut: { key: "c", ctrl: true, shift: true }, run: () => copyAndClose([...item.path, item.title].join(" › "), t("Copied Menu Path")) },
          ],
        ],
      };
      groups.set(section, [...(groups.get(section) ?? []), entry]);
    }
    return [...groups].map(([title, items]) => ({ title, items }));
  }, [data]);

  return (
    <ListView
      placeholder={data?.app ? t("Search menu items of {app}…", { app: data.app }) : t("Search menu items…")}
      sections={sections}
      onBack={pop}
      footerTitle={t("Search Menu Items")}
      footerIcon={<GlyphTile spec={ICON} small />}
      isLoading={data === null}
      emptyText={t("No Menu Items")}
      emptyDescription={t("{app} has no classic menu bar that Openray can read.", { app: data?.app || t("This app") })}
    />
  );
}

export default defineExtension({
  id: "menu-items",
  title: "Menu Items",
  icon: ICON,
  commands: [
    {
      id: "search",
      title: "Search Menu Items",
      subtitle: "Menu Items",
      keywords: ["menu bar", "menu", "commands", "active app", "itens de menu", "barra de menus"],
      kind: "Command",
      primaryActionTitle: "Open Command",
      view: SearchMenuItems,
    },
  ],
});
