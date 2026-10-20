import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { Clipboard, Folder, Gamepad2, Trash2 } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { fail } from "../ActionHost";
import { defineExtension, type Action, type Command, type IconSpec, type Item, type ViewProps } from "../extension-api";
import { GlyphTile } from "../icons";
import { ListView } from "../ListView";
import { copyAndClose } from "../output";

const ICON: IconSpec = { glyph: Gamepad2, tone: "green" };

interface Game {
  id: string;
  name: string;
  launcher: string;
  installDir: string;
  exe: string | null;
  icon: string | null;
  launch: string;
  uninstall: string;
}

interface Launcher {
  name: string;
  path: string;
}

const listGames = () => invoke<{ games: Game[]; launchers: Launcher[] }>("list_games");

const iconUrl = (game: Game) => {
  const size = Math.round(22 * window.devicePixelRatio);
  if (game.icon) return `${convertFileSrc(game.icon, "appicon")}?size=${size}&thumb=1`;
  return game.exe ? `${convertFileSrc(game.exe, "appicon")}?size=${size}` : null;
};

const openGame = (game: Game) => invoke<void>("open_url", { url: game.launch, appPath: null });

export function gameActions(game: Game): Action[][] {
  return [
    [{ id: "show-in-explorer", title: "Show in File Explorer", icon: Folder, run: () => invoke("show_in_explorer", { path: game.installDir }) }],
    [
      { id: "copy-path", title: "Copy Path", icon: Clipboard, shortcut: { key: "c", ctrl: true, alt: true }, run: () => copyAndClose(game.installDir, "Copied Path to Clipboard") },
      { id: "copy-name", title: "Copy Name", icon: Clipboard, shortcut: { key: "c", alt: true, shift: true }, run: () => copyAndClose(game.name.trim(), "Copied Name to Clipboard") },
    ],
    [
      {
        id: "uninstall",
        title: "Uninstall Game",
        icon: Trash2,
        destructive: true,
        confirm: { title: `Uninstall ${game.name.trim()}?`, message: `${game.launcher} will open to uninstall it.`, confirmTitle: "Uninstall" },
        run: () => invoke("open_url", { url: game.uninstall, appPath: null }),
      },
    ],
  ];
}

function GameIcon({ game }: { game: Game }) {
  const [failed, setFailed] = useState(false);
  const src = iconUrl(game);
  if (!src || failed) return <Gamepad2 aria-hidden="true" className="size-[22px] shrink-0 text-fg-60" />;
  return <img src={src} alt="" draggable={false} onError={() => setFailed(true)} className="size-[22px] shrink-0 rounded-[4px]" />;
}

function GameLauncher({ pop }: ViewProps) {
  const [data, setData] = useState<{ games: Game[]; launchers: Launcher[] } | null>(null);
  const [text, setText] = useState("");

  useEffect(() => {
    listGames()
      .then(setData)
      .catch((e) => fail("Unable to Load Games", e));
  }, []);

  const sections = useMemo(() => {
    const kind = (label: string) => <span className="text-13 text-fg-60">{label}</span>;
    const games = (data?.games ?? []).map(
      (game): Item => ({
        id: game.id,
        title: game.name.trim(),
        icon: <GameIcon game={game} />,
        accessory: kind("Game"),
        keywords: [game.launcher],
        actions: [[{ id: "open", title: "Open Game", icon: Gamepad2, run: () => openGame(game) }, ...gameActions(game)[0]], ...gameActions(game).slice(1)],
      }),
    );
    const launchers = (data?.launchers ?? []).map(
      (launcher): Item => ({
        id: launcher.path,
        title: launcher.name,
        icon: <img src={`${convertFileSrc(launcher.path, "appicon")}?size=${Math.round(22 * window.devicePixelRatio)}`} alt="" draggable={false} className="size-[22px] shrink-0" />,
        accessory: kind("Launcher"),
        actions: [[{ id: "open", title: "Open Application", icon: Gamepad2, run: () => invoke<void>("launch_app", { path: launcher.path }) }]],
      }),
    );
    if (text.trim()) return [{ title: "Results", items: [...games, ...launchers] }];
    return [
      { title: "Suggestions", items: games.slice(0, 5) },
      { title: "Games", items: games.slice(5) },
      { title: "Game Launchers", items: launchers },
    ].filter((s) => s.items.length > 0);
  }, [data, text]);

  return (
    <ListView
      placeholder="Search games…"
      sections={sections}
      onBack={pop}
      searchText={text}
      onSearchTextChange={setText}
      footerTitle="Game Launcher"
      footerIcon={<GlyphTile spec={ICON} small />}
      isLoading={data === null}
      emptyText={text.trim() ? "No Results" : "No Games Found"}
      emptyDescription={text.trim() ? "Try a different search term." : undefined}
    />
  );
}

export default defineExtension({
  id: "games",
  title: "Games",
  icon: ICON,
  commands: [
    {
      id: "search-games",
      title: "Game Launcher",
      subtitle: "Games",
      keywords: ["games", "steam", "play"],
      kind: "Command",
      primaryActionTitle: "Open Command",
      view: GameLauncher,
    },
  ],
  items: async () =>
    (await listGames()).games.map(
      (game): Command => ({
        id: game.id,
        title: game.name.trim(),
        icon: iconUrl(game) ?? ICON,
        keywords: ["game", "games", game.launcher],
        kind: "Game",
        primaryActionTitle: "Open Game",
        actions: gameActions(game),
        run: () => openGame(game),
      }),
    ),
});
