import { describe, expect, it } from "vitest";
import type { UiTree } from "@flowkey-cli/native-sdk";
import { formatClock, litSegments, mediaPosition, notchItems, notchSize, primaryAction, weekStrip } from "./notch";

describe("notch", () => {
  it("sizes the pill by state and widens it while media plays", () => {
    expect(notchSize("collapsed", false).width).toBeLessThan(notchSize("collapsed", true).width);
    expect(notchSize("expanded", false)).toEqual(notchSize("expanded", true));
    expect(notchSize("volume", true).height).toBeGreaterThan(notchSize("collapsed", true).height);
  });

  it("centres the week strip on today across a month boundary", () => {
    const days = weekStrip(new Date(2026, 9, 1), "en-US");
    expect(days.map((d) => d.day)).toEqual(["28", "29", "30", "01", "02", "03", "04"]);
    expect(days.filter((d) => d.today).map((d) => d.day)).toEqual(["01"]);
    expect(days[3].weekday).toBe("T");
  });

  it("lights volume segments proportionally and clamps", () => {
    expect(litSegments(0, 36)).toBe(0);
    expect(litSegments(81, 36)).toBe(29);
    expect(litSegments(100, 36)).toBe(36);
    expect(litSegments(140, 36)).toBe(36);
  });

  it("advances the media position only while playing and caps it at the duration", () => {
    const media = { playing: true, positionMs: 10_000, durationMs: 60_000, updatedAtMs: 1_000 };
    expect(mediaPosition(media, 6_000)).toBe(15_000);
    expect(mediaPosition({ ...media, playing: false }, 6_000)).toBe(10_000);
    expect(mediaPosition(media, 1_000_000)).toBe(60_000);
    expect(mediaPosition({ playing: true }, 6_000)).toBeNull();
    expect(formatClock(75_400)).toBe("1:15");
  });

  it("flattens extension trees into widget rows", () => {
    const list: UiTree = { type: "list", sections: [{ items: [{ id: "a", title: "A" }] }, { items: [{ id: "b", title: "B", actions: [{ id: "x", title: "X" }, { id: "y", title: "Y", primary: true }] }] }] };
    expect(notchItems(list).map((i) => i.id)).toEqual(["a", "b"]);
    expect(notchItems({ type: "grid", columns: 2, items: [{ id: "g", title: "G" }] }).map((i) => i.id)).toEqual(["g"]);
    expect(notchItems({ type: "detail", title: "D", fields: [] })).toEqual([]);
    expect(primaryAction(notchItems(list)[1])?.id).toBe("y");
    expect(primaryAction(notchItems(list)[0])).toBeUndefined();
  });
});
