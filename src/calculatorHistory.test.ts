import { describe, expect, it } from "vitest";
import { groupHistory, movePinned, prune, RETENTION_MS, search, setPinned, upsert, type Calculation } from "./calculatorHistory";

const DAY = 86_400_000;
const NOW = new Date(2026, 9, 21, 15, 0).getTime();

describe("calculator history", () => {
  it("upserts by expression and keeps the newest result", () => {
    let { list, id } = upsert([], " 2+2 ", "4", "math", NOW - 1000);
    ({ list } = upsert(list, "10 usd in eur", "€9.20", "unitConversion", NOW - 500));
    const again = upsert(list, "2+2", "4", "math", NOW);
    expect(again.id).toBe(id);
    expect(again.list).toHaveLength(2);
    expect(again.list.find((c) => c.id === id)).toMatchObject({ expression: "2+2", createdAt: NOW - 1000, updatedAt: NOW });
  });

  it("drops unpinned entries older than three months", () => {
    const old: Calculation = { id: "a", expression: "1+1", result: "2", kind: "math", createdAt: 0, updatedAt: NOW - RETENTION_MS - 1 };
    const kept = prune([old, { ...old, id: "b", pinned: 1 }], NOW);
    expect(kept.map((c) => c.id)).toEqual(["b"]);
  });

  it("pins in order and moves pinned entries", () => {
    let { list } = upsert([], "1", "1", "math", NOW);
    const ids = ["a", "b", "c"].map((e, i) => {
      const r = upsert(list, e, e, "math", NOW + i);
      list = r.list;
      return r.id;
    });
    for (const id of ids) list = setPinned(list, id, true);
    list = movePinned(list, ids[2], -1);
    expect(groupHistory(list, NOW)[0].items.map((c) => c.expression)).toEqual(["a", "c", "b"]);
    list = setPinned(list, ids[0], false);
    expect(groupHistory(list, NOW).map((s) => s.title)).toEqual(["Pinned", "Today"]);
  });

  it("groups by day, week, month, year and older", () => {
    const at = (id: string, updatedAt: number): Calculation => ({ id, expression: id, result: id, kind: "math", createdAt: updatedAt, updatedAt });
    const list = [at("today", NOW - 1000), at("yesterday", NOW - DAY), at("week", new Date(2026, 9, 19, 9).getTime()), at("month", new Date(2026, 9, 2).getTime()), at("year", new Date(2026, 1, 1).getTime()), at("older", new Date(2025, 11, 1).getTime())];
    expect(groupHistory(list, NOW).map((s) => [s.title, s.items[0].id])).toEqual([
      ["Today", "today"],
      ["Yesterday", "yesterday"],
      ["This Week", "week"],
      ["This Month", "month"],
      ["This Year", "year"],
      ["Older", "older"],
    ]);
  });

  it("searches expressions and results", () => {
    const { list } = upsert([], "15% of 80", "12", "math", NOW);
    expect(search(list, "12")).toHaveLength(1);
    expect(search(list, "of 8")).toHaveLength(1);
    expect(search(list, "usd")).toHaveLength(0);
  });
});
