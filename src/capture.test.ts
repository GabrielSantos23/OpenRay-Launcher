import { describe, expect, it } from "vitest";
import { clampToScreen, measure, rectFrom, targetAt } from "./capture";

describe("capture geometry", () => {
  it("builds a rectangle from two corners in any direction", () => {
    expect(rectFrom([10, 20], [5, 2])).toEqual({ x: 5, y: 2, w: 6, h: 19 });
    expect(rectFrom([3, 3], [3, 3])).toEqual({ x: 3, y: 3, w: 1, h: 1 });
  });

  it("picks the topmost window under the cursor, then the monitor", () => {
    const windows = [
      { x: 100, y: 100, w: 200, h: 200 },
      { x: 0, y: 0, w: 1000, h: 800 },
    ];
    const monitors = [{ x: 0, y: 0, w: 1920, h: 1080 }];
    expect(targetAt([150, 150], windows, monitors)).toBe(windows[0]);
    expect(targetAt([50, 50], windows, monitors)).toBe(windows[1]);
    expect(targetAt([1500, 900], windows, monitors)).toBe(monitors[0]);
    expect(targetAt([5000, 900], windows, monitors)).toBeNull();
  });

  it("clamps a selection to the screen", () => {
    expect(clampToScreen({ x: -10, y: 5, w: 30, h: 2000 }, 1920, 1080)).toEqual({ x: 0, y: 5, w: 20, h: 1075 });
    expect(clampToScreen({ x: 3000, y: 5, w: 30, h: 20 }, 1920, 1080)).toBeNull();
  });

  it("formats measurements", () => {
    expect(measure({ x: 0, y: 0, w: 300, h: 400 })).toBe("300 × 400  ·  500 px");
  });
});
