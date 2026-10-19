import { describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const { BUILTIN_CATEGORIES, DEFAULT_PREFERENCES, elapsedFraction, formatTimer, normalizeHost, parseCategoryImport, pauseRemainingMs, remainingMs, slugify, startInput, timerStep } =
  await import("./focus");

const session = { id: 1, title: "Write", mode: "block" as const, status: "running" as const, startedAt: 0, durationMs: 600_000, endAt: 600_000, remainingMs: 600_000, pauseEndsAt: null, form: null };

describe("focus", () => {
  it("formats the timer like the reference panel", () => {
    expect(formatTimer(25 * 60_000)).toBe("25:00");
    expect(formatTimer(59_001)).toBe("01:00");
    expect(formatTimer(90 * 60_000)).toBe("01:30");
    expect(formatTimer(-5)).toBe("00:00");
    expect(timerStep(300_000)).toBe(1000);
    expect(timerStep(1_800_000)).toBe(5000);
    expect(timerStep(7_200_000)).toBe(60_000);
  });

  it("tracks remaining, paused and elapsed time", () => {
    expect(remainingMs(session, 150_000)).toBe(450_000);
    expect(elapsedFraction(session, 150_000)).toBe(0.25);
    const paused = { ...session, status: "paused" as const, endAt: null, remainingMs: 200_000, pauseEndsAt: 400_000 };
    expect(remainingMs(paused, 999_999)).toBe(200_000);
    expect(pauseRemainingMs(paused, 100_000)).toBe(300_000);
    expect(remainingMs({ ...session, durationMs: null }, 1)).toBe(0);
    expect(elapsedFraction({ ...session, durationMs: null }, 1)).toBe(0);
  });

  it("normalizes website hosts", () => {
    expect(normalizeHost("https://www.YouTube.com/watch?v=1")).toBe("youtube.com");
    expect(normalizeHost("x.com")).toBe("x.com");
    expect(normalizeHost("amazon.*")).toBe("amazon.*");
    expect(normalizeHost("not a host")).toBeNull();
    expect(normalizeHost("localhost")).toBeNull();
  });

  it("merges categories, apps and websites into the session input", () => {
    const input = startInput({ goal: " ", durationMs: 1500_000, mode: "block", items: ["category:social", "app:C:\\Apps\\Code.exe", "site:github.com"] }, BUILTIN_CATEGORIES, DEFAULT_PREFERENCES);
    expect(input.title).toBe("Focus session");
    expect(input.apps).toEqual(["C:\\Apps\\Code.exe"]);
    expect(input.websites).toContain("reddit.com");
    expect(input.websites).toContain("github.com");
    expect(input.snoozeMs).toBe(180_000);
    expect(startInput({ goal: "", durationMs: null, mode: "allow", items: [] }, [], { ...DEFAULT_PREFERENCES, snoozeMs: 0 }).snoozeMs).toBeNull();
  });

  it("imports categories in the reference JSON shapes", () => {
    expect(parseCategoryImport('[{"title":"Dev","websiteHosts":["https://github.com"],"applicationIds":["code.exe"]}]')).toEqual([
      { title: "Dev", icon: "target", apps: ["code.exe"], websites: ["github.com"] },
    ]);
    expect(parseCategoryImport('{"focusCategories":[{"title":"Fun","icon":"game","websites":["x.com"],"apps":[]}]}')[0].icon).toBe("game");
    expect(parseCategoryImport('{"title":"One"}')).toHaveLength(1);
    expect(() => parseCategoryImport('[{"websites":[]}]')).toThrow();
  });

  it("makes unique category ids", () => {
    expect(slugify("Deep Work!", [])).toBe("deep-work");
    expect(slugify("Social", ["social", "social-2"])).toBe("social-3");
  });
});
