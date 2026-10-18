import { beforeEach, describe, expect, it, vi } from "vitest";

const invoke = vi.fn(() => Promise.resolve());
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("react", () => ({ useSyncExternalStore: (_subscribe: unknown, getSnapshot: () => unknown) => getSnapshot() }));

const { hideToast, setLauncherVisible, showToast, useToast } = await import("./toast");

describe("showToast", () => {
  beforeEach(() => {
    invoke.mockClear();
    hideToast();
  });

  it("shows the toast in the footer while the launcher is visible", () => {
    setLauncherVisible(true);
    showToast({ variant: "success", title: "Copied path to clipboard" });
    expect(useToast()).toEqual({ variant: "success", title: "Copied path to clipboard" });
    expect(invoke).not.toHaveBeenCalled();
  });

  it("sends the toast to the HUD window while the launcher is hidden", () => {
    setLauncherVisible(false);
    const toast = { variant: "failure" as const, title: "Could not open Notepad" };
    showToast(toast);
    expect(useToast()).toBeNull();
    expect(invoke).toHaveBeenCalledWith("show_hud", { toast });
  });

  it("clears the footer toast when the launcher hides", () => {
    setLauncherVisible(true);
    showToast({ variant: "success", title: "Copied" });
    setLauncherVisible(false);
    expect(useToast()).toBeNull();
  });
});
