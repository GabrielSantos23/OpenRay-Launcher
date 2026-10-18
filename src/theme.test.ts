import { describe, expect, it } from "vitest";
import type { Settings } from "./settings";
import { activeTheme, THEMES, themeVariables } from "./theme";

const settings = (patch: Partial<Settings>): Settings => ({
  hotkey: "Alt+Space",
  showInTray: true,
  showOn: "mouse-screen",
  escapeKeyBehavior: "pop-back-or-close",
  escapeKeyClosesWindow: true,
  appearance: "system",
  themeDarkId: "bundled-sky",
  themeLightId: "bundled-noon",
  commands: {},
  ...patch,
});

describe("themes", () => {
  it("ships the eight bundled reference themes", () => {
    expect(THEMES.map((t) => t.name)).toEqual(["Raycast Dark", "Raycast Light", "Sky", "Sunset", "Midnight", "Dawn", "Overcast", "Noon"]);
  });

  it("follows the system appearance unless one is forced", () => {
    expect(activeTheme(settings({}), true).name).toBe("Sky");
    expect(activeTheme(settings({}), false).name).toBe("Noon");
    expect(activeTheme(settings({ appearance: "dark" }), false).name).toBe("Sky");
    expect(activeTheme(settings({ appearance: "light", themeLightId: "missing" }), true).name).toBe("Raycast Light");
  });

  it("puts 40% of the theme background over the Windows backdrop, as the reference does", () => {
    const vars = themeVariables(THEMES[0], "#E2241A");
    expect(vars["--color-bg"]).toBe("color-mix(in srgb, #000000 40%, #333333)");
    expect(vars["--color-accent"]).toBe("#E2241A");
    expect(themeVariables(THEMES[1], null)["--color-accent"]).toBe("#138AF2");
  });
});
