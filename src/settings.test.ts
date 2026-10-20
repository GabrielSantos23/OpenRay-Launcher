import { describe, expect, it } from "vitest";
import { hotkeyFromEvent, hotkeyLabels, hotkeyOwner, isEnabled, sameHotkey, withCommandPrefs, withHotkey, type Settings } from "./settings";

const event = (code: string, mods: Partial<{ ctrlKey: boolean; altKey: boolean; shiftKey: boolean; metaKey: boolean }> = {}) => ({
  code,
  ctrlKey: false,
  altKey: false,
  shiftKey: false,
  metaKey: false,
  ...mods,
});

const base: Settings = {
  hotkey: "Alt+Space",
  showInTray: true,
  showOn: "mouse-screen",
  escapeKeyBehavior: "pop-back-or-close",
  escapeKeyClosesWindow: true,
  appearance: "system",
  themeDarkId: "bundled-raycast-dark",
  themeLightId: "bundled-raycast-light",
  commands: {},
};

describe("hotkeyFromEvent", () => {
  it("names keys the way the backend parser reads them", () => {
    expect(hotkeyFromEvent(event("Space", { altKey: true }))).toBe("Alt+Space");
    expect(hotkeyFromEvent(event("KeyO", { ctrlKey: true, altKey: true, shiftKey: true }))).toBe("Ctrl+Alt+Shift+O");
    expect(hotkeyFromEvent(event("Digit1", { metaKey: true }))).toBe("Win+1");
    expect(hotkeyFromEvent(event("F12", { ctrlKey: true }))).toBe("Ctrl+F12");
    expect(hotkeyFromEvent(event("Backquote", { altKey: true }))).toBe("Alt+`");
  });

  it("waits for a key with at least one modifier", () => {
    expect(hotkeyFromEvent(event("KeyA"))).toBeNull();
    expect(hotkeyFromEvent(event("ControlLeft", { ctrlKey: true }))).toBeNull();
    expect(hotkeyFromEvent(event("CapsLock", { ctrlKey: true }))).toBeNull();
    expect(hotkeyFromEvent(event("F1"))).toBeNull();
  });

  it("records Print Screen with or without modifiers", () => {
    expect(hotkeyFromEvent(event("PrintScreen"))).toBe("PrintScreen");
    expect(hotkeyFromEvent(event("PrintScreen", { altKey: true }))).toBe("Alt+PrintScreen");
    expect(hotkeyFromEvent(event("F13"))).toBe("F13");
    expect(hotkeyLabels("Alt+PrintScreen")).toEqual(["Alt", "PrtSc"]);
  });

  it("records arrows, navigation keys and numpad digits", () => {
    expect(hotkeyFromEvent(event("ArrowRight", { ctrlKey: true, shiftKey: true }))).toBe("Ctrl+Shift+ArrowRight");
    expect(hotkeyFromEvent(event("Numpad7", { altKey: true }))).toBe("Alt+Numpad7");
    expect(hotkeyLabels("Ctrl+Shift+ArrowRight")).toEqual(["Ctrl", "Shift", "→"]);
  });
});

describe("command prefs", () => {
  it("disables a command alone or through its extension", () => {
    const one = withCommandPrefs(base, "applications/a", { disabled: true });
    expect(isEnabled(one, "applications/a", "applications")).toBe(false);
    expect(isEnabled(one, "applications/b", "applications")).toBe(true);
    const all = withCommandPrefs(base, "applications", { disabled: true });
    expect(isEnabled(all, "applications/b", "applications")).toBe(false);
    expect(withCommandPrefs(one, "applications/a", { alias: "x" }).commands["applications/a"]).toEqual({ alias: "x", hotkey: "", disabled: true });
  });
});

describe("hotkey ownership", () => {
  const taken = withCommandPrefs(base, "applications/a", { hotkey: "Ctrl+Alt+A" });

  it("compares hotkeys regardless of case, spacing and modifier order", () => {
    expect(sameHotkey("ctrl + alt + a", "Alt+Ctrl+A")).toBe(true);
    expect(sameHotkey("Ctrl+A", "Ctrl+Shift+A")).toBe(false);
    expect(sameHotkey("", "")).toBe(false);
  });

  it("finds who already uses a hotkey, ignoring the asker", () => {
    expect(hotkeyOwner(taken, "Alt+Ctrl+A", "applications/b")).toBe("applications/a");
    expect(hotkeyOwner(taken, "Ctrl+Alt+A", "applications/a")).toBeNull();
    expect(hotkeyOwner(taken, "Alt+Space", "applications/b")).toBe("launcher");
    expect(hotkeyOwner(taken, "Alt+Space", "launcher")).toBeNull();
  });

  it("moves a hotkey on overwrite and keeps others otherwise", () => {
    const moved = withHotkey(taken, "applications/b", "Ctrl+Alt+A", true);
    expect(moved.commands["applications/a"].hotkey).toBe("");
    expect(moved.commands["applications/b"].hotkey).toBe("Ctrl+Alt+A");
    expect(withHotkey(taken, "launcher", "Ctrl+Space", false).hotkey).toBe("Ctrl+Space");
  });
});
