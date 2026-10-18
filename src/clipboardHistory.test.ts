import { describe, expect, it } from "vitest";
import { entryTitle, entryType, groupEntries, matchesFilter, wordCount, type ClipboardEntry } from "./clipboardHistory";

const entry = (over: Partial<ClipboardEntry>): ClipboardEntry => ({
  id: "1",
  kind: "text",
  text: "",
  sourceName: "",
  sourcePath: "",
  copyCount: 1,
  createdAt: 0,
  updatedAt: 0,
  pinned: null,
  ...over,
});

describe("clipboard history", () => {
  it("classifies text entries", () => {
    expect(entryType(entry({ text: "https://raycast.com/x" }))).toBe("Link");
    expect(entryType(entry({ text: "me@example.com" }))).toBe("Email");
    expect(entryType(entry({ text: "#FF6363" }))).toBe("Color");
    expect(entryType(entry({ text: "Built-in features" }))).toBe("Text");
    expect(entryType(entry({ kind: "image", width: 2, height: 2 }))).toBe("Image");
    expect(matchesFilter(entry({ text: "me@example.com" }), "link")).toBe(true);
    expect(matchesFilter(entry({ text: "#fff" }), "text")).toBe(true);
    expect(matchesFilter(entry({ kind: "files", paths: ["C:\\a.txt"] }), "text")).toBe(false);
  });

  it("titles entries by content", () => {
    expect(entryTitle(entry({ text: "\n  first line  \nsecond" }))).toBe("first line");
    expect(entryTitle(entry({ kind: "image", width: 340, height: 189 }))).toBe("Image (340×189)");
    expect(entryTitle(entry({ kind: "files", paths: ["C:\\Users\\gabs\\notes.txt"] }))).toBe("notes.txt");
    expect(entryTitle(entry({ kind: "files", paths: ["a", "b"] }))).toBe("2 Files");
    expect(wordCount(" and other  things ")).toBe(3);
  });

  it("groups pinned entries first, then by day", () => {
    const now = new Date(2026, 9, 18, 12).getTime();
    const yesterday = new Date(2026, 9, 17, 23).getTime();
    const older = new Date(2026, 9, 14, 9).getTime();
    const groups = groupEntries(
      [entry({ id: "p", pinned: 1, updatedAt: older }), entry({ id: "a", updatedAt: now }), entry({ id: "b", updatedAt: yesterday }), entry({ id: "c", updatedAt: older })],
      now,
    );
    expect(groups.map((g) => [g.title, g.entries.map((e) => e.id)])).toEqual([
      ["Pinned", ["p"]],
      ["Today", ["a"]],
      ["Yesterday", ["b"]],
      ["October 14", ["c"]],
    ]);
  });
});
