import { describe, expect, it, vi } from "vitest";
import { actionGroups, matchesShortcut, shortcutLabels } from "./actions";
import type { RegisteredCommand } from "./extension-api";

function cmd(key: string, actions?: RegisteredCommand["actions"]): RegisteredCommand {
  return { id: key, key, extensionId: "test", title: key, kind: "Application", primaryActionTitle: "Open Application", actions, run: () => {} };
}

function press(key: string, code: string, mods: { ctrl?: boolean; alt?: boolean; shift?: boolean } = {}) {
  return { key, code, ctrlKey: !!mods.ctrl, altKey: !!mods.alt, shiftKey: !!mods.shift };
}

describe("shortcuts", () => {
  it("labels modifiers in Ctrl, Alt, Shift order with key glyphs", () => {
    expect(shortcutLabels({ key: "Enter", ctrl: true, shift: true })).toEqual(["Ctrl", "Shift", "↵"]);
    expect(shortcutLabels({ key: "c", ctrl: true, alt: true })).toEqual(["Ctrl", "Alt", "C"]);
    expect(shortcutLabels({ key: "ArrowUp", alt: true, ctrl: true })).toEqual(["Ctrl", "Alt", "↑"]);
  });

  it("matches exact modifiers and letters by physical key", () => {
    const copy = { key: "c", ctrl: true, alt: true };
    expect(matchesShortcut(press("©", "KeyC", { ctrl: true, alt: true }), copy)).toBe(true);
    expect(matchesShortcut(press("c", "", { ctrl: true, alt: true }), copy)).toBe(true);
    expect(matchesShortcut(press("c", "KeyC", { ctrl: true }), copy)).toBe(false);
    expect(matchesShortcut(press("Enter", "Enter"), { key: "Enter" })).toBe(true);
    expect(matchesShortcut(press("Enter", "Enter", { ctrl: true, shift: true }), { key: "Enter" })).toBe(false);
  });
});

describe("actionGroups", () => {
  const base = { frecency: {}, setFavorites: vi.fn(), resetRanking: vi.fn() };
  const titles = (groups: ReturnType<typeof actionGroups>) => groups.map((g) => g.map((a) => a.title));

  it("puts the primary and extension actions first, then favorites, then the rest", () => {
    const explorer = { id: "x", title: "Show in File Explorer", run: () => {} };
    const copy = { id: "c", title: "Copy Name", run: () => {} };
    const groups = actionGroups(cmd("a", [[explorer], [copy]]), { ...base, favorites: [] });
    expect(titles(groups)).toEqual([["Open Application", "Show in File Explorer"], ["Add to Favorites"], ["Copy Name"]]);
  });

  it("offers remove and only the moves that are possible", () => {
    expect(titles(actionGroups(cmd("a"), { ...base, favorites: ["a", "b"] }))[1]).toEqual(["Remove from Favorites", "Move Favorite Down"]);
    expect(titles(actionGroups(cmd("b"), { ...base, favorites: ["a", "b"] }))[1]).toEqual(["Remove from Favorites", "Move Favorite Up"]);
  });

  it("moves and removes favorites in order", () => {
    const setFavorites = vi.fn();
    const groups = actionGroups(cmd("b"), { ...base, setFavorites, favorites: ["a", "b", "c"] });
    groups[1].find((a) => a.id === "move-favorite-up")!.run();
    expect(setFavorites).toHaveBeenLastCalledWith(["b", "a", "c"]);
    groups[1].find((a) => a.id === "move-favorite-down")!.run();
    expect(setFavorites).toHaveBeenLastCalledWith(["a", "c", "b"]);
    groups[1].find((a) => a.id === "remove-favorite")!.run();
    expect(setFavorites).toHaveBeenLastCalledWith(["a", "c"]);
  });

  it("offers Reset Ranking only for ranked commands", () => {
    expect(titles(actionGroups(cmd("a"), { ...base, favorites: [] })).flat()).not.toContain("Reset Ranking");
    expect(titles(actionGroups(cmd("a"), { ...base, favorites: [], frecency: { a: 2 } })).at(-1)).toEqual(["Reset Ranking"]);
  });
});
