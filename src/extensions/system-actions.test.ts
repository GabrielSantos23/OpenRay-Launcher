import { describe, expect, it } from "vitest";
import { parseVolume } from "./system-actions";

describe("Set Volume argument", () => {
  it("accepts numbers with an optional percent sign and caps at 100", () => {
    expect(["40", " 40% ", "33.6", "150", "0"].map(parseVolume)).toEqual([40, 40, 34, 100, 0]);
    expect(["", "-5", "loud", "mute", "4 0"].map(parseVolume)).toEqual([null, null, null, null, null]);
  });
});
