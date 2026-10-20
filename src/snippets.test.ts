import { describe, expect, it } from "vitest";
import { exportSnippets, keywordInUse, mergeImport, parseSnippetImport, resolveSnippet, type Snippet } from "./snippets";

const snippet = (over: Partial<Snippet>): Snippet => ({ id: "x", name: "X", text: "", createdAt: 0, modifiedAt: 0, ...over });

const context = {
  clipboard: async (offset: number) => ["now", "before"][offset] ?? "",
  snippets: [snippet({ id: "sig", name: "Signature", text: "Ana{cursor} — {clipboard}" })],
  now: new Date(2026, 9, 19, 14, 5),
  locale: "en-US",
  uuid: () => "id-1",
};

describe("snippets", () => {
  it("resolves dynamic placeholders", async () => {
    expect((await resolveSnippet("{date} | {time} | {day}", context)).text).toBe("Oct 19, 2026 | 2:05 PM | Monday");
    expect((await resolveSnippet("{clipboard} {clipboard offset=\"1\"} {uuid}", context)).text).toBe("now before id-1");
    expect((await resolveSnippet("{unknown} {argument name=\"a\"}", context)).text).toBe("{unknown} {argument name=\"a\"}");
  });

  it("returns how far the cursor moves back", async () => {
    expect(await resolveSnippet("Hi {cursor}!\r\nBye", context)).toEqual({ text: "Hi !\nBye", cursorBack: 5 });
    expect((await resolveSnippet("plain", context)).cursorBack).toBe(0);
  });

  it("inserts other snippets one level deep", async () => {
    expect(await resolveSnippet("Best,\n{snippet name=\"signature\"}", context)).toEqual({ text: "Best,\nAna — now", cursorBack: 0 });
  });

  it("detects keywords already in use", () => {
    const list = [snippet({ id: "a", keyword: ";sig" })];
    expect(keywordInUse(" ;sig ", list)).toBe(true);
    expect(keywordInUse(";sig", list, "a")).toBe(false);
    expect(keywordInUse("", list)).toBe(false);
  });

  it("imports, skips duplicates and exports", () => {
    const parsed = parseSnippetImport('[{"name":"A","text":"a","keyword":";a"},{"name":"B","text":"b","keyword":";a"},{"title":"C","text":"c"}]');
    const merged = mergeImport([snippet({ id: "c0", name: "C", text: "c" })], parsed, 5, () => "new");
    expect([merged.added, merged.skipped]).toEqual([1, 2]);
    expect(JSON.parse(exportSnippets(merged.snippets))).toEqual([{ name: "C", text: "c" }, { name: "A", text: "a", keyword: ";a" }]);
    expect(() => parseSnippetImport('[{"name":"x"}]')).toThrow("Invalid snippet import format");
  });
});
