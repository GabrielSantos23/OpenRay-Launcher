import { describe, expect, it } from "vitest";
import { screenshotSections, type Screenshot } from "./screenshots";

const shot = (name: string, date: Date): Screenshot => ({ path: `C:\\s\\${name}`, name, modified: date.getTime(), size: 1, video: false });

describe("screenshotSections", () => {
  it("groups newest first by relative day, then by month", () => {
    const now = new Date(2026, 9, 20, 18);
    const sections = screenshotSections(
      [shot("old.png", new Date(2026, 7, 3)), shot("a.png", new Date(2026, 9, 20, 9)), shot("b.png", new Date(2026, 9, 19, 23)), shot("c.png", new Date(2026, 9, 15)), shot("d.png", new Date(2026, 9, 2))],
      now,
    );
    expect(sections.map((s) => [s.title, s.items.map((i) => i.name)])).toEqual([
      ["Today", ["a.png"]],
      ["Yesterday", ["b.png"]],
      ["Previous 7 Days", ["c.png"]],
      ["October 2026", ["d.png"]],
      ["August 2026", ["old.png"]],
    ]);
  });
});
