import { ArrowDown, ArrowUp, RotateCcw, SquareMenu, Star, StarOff } from "lucide-react";
import type { Action, RegisteredCommand, Shortcut } from "./extension-api";
import type { Frecency } from "./search";

const KEY_LABELS: Record<string, string> = { Enter: "↵", ArrowUp: "↑", ArrowDown: "↓", ArrowLeft: "←", ArrowRight: "→", Backspace: "⌫", Delete: "Del", Escape: "Esc", PageUp: "PgUp", PageDown: "PgDn", Home: "Home", End: "End", Tab: "Tab", " ": "Space" };

export function shortcutLabels(shortcut: Shortcut): string[] {
  const modifiers = [shortcut.ctrl && "Ctrl", shortcut.alt && "Alt", shortcut.shift && "Shift"].filter((m): m is string => !!m);
  return [...modifiers, KEY_LABELS[shortcut.key] ?? shortcut.key.toUpperCase()];
}

interface KeyEventLike {
  key: string;
  code: string;
  ctrlKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
}

export function matchesShortcut(event: KeyEventLike, shortcut: Shortcut): boolean {
  const key =
    shortcut.key.length === 1
      ? event.code === `Key${shortcut.key.toUpperCase()}` || event.key.toLowerCase() === shortcut.key
      : event.key === shortcut.key;
  return key && event.ctrlKey === !!shortcut.ctrl && event.altKey === !!shortcut.alt && event.shiftKey === !!shortcut.shift;
}

export interface ActionContext {
  favorites: string[];
  frecency: Frecency;
  setFavorites: (keys: string[]) => void;
  resetRanking: (key: string) => void;
}

export function actionGroups(command: RegisteredCommand, context: ActionContext): Action[][] {
  const { favorites, setFavorites } = context;
  const [own = [], ...rest] = command.actions ?? [];
  const primary: Action = { id: "primary", title: command.primaryActionTitle, icon: SquareMenu, shortcut: { key: "Enter" }, run: () => command.run?.() };
  const index = favorites.indexOf(command.key);
  const move = (offset: number) => {
    const next = [...favorites];
    [next[index], next[index + offset]] = [next[index + offset], next[index]];
    setFavorites(next);
  };
  const favoriteActions: Action[] =
    index < 0
      ? [
          {
            id: "add-favorite",
            title: "Add to Favorites",
            icon: Star,
            shortcut: { key: "f", ctrl: true, shift: true },
            keepOpen: true,
            run: () => setFavorites([...favorites, command.key]),
          },
        ]
      : [
          {
            id: "remove-favorite",
            title: "Remove from Favorites",
            icon: StarOff,
            shortcut: { key: "f", ctrl: true, shift: true },
            keepOpen: true,
            run: () => setFavorites(favorites.filter((key) => key !== command.key)),
          },
          ...(index > 0
            ? [{ id: "move-favorite-up", title: "Move Favorite Up", icon: ArrowUp, shortcut: { key: "ArrowUp", ctrl: true, alt: true }, keepOpen: true, run: () => move(-1) }]
            : []),
          ...(index < favorites.length - 1
            ? [
                {
                  id: "move-favorite-down",
                  title: "Move Favorite Down",
                  icon: ArrowDown,
                  shortcut: { key: "ArrowDown", ctrl: true, alt: true },
                  keepOpen: true,
                  run: () => move(1),
                },
              ]
            : []),
        ];
  const ranking: Action[] =
    (context.frecency[command.key] ?? 0) > 0
      ? [{ id: "reset-ranking", title: "Reset Ranking", icon: RotateCcw, keepOpen: true, run: () => context.resetRanking(command.key) }]
      : [];
  return [[primary, ...own], favoriteActions, ...rest, ranking].filter((group) => group.length > 0);
}
