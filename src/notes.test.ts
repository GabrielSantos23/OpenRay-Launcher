import { describe, expect, it } from "vitest";
import { browseSections, countLabel, currentNote, emptyNote, newNote, noteTitle } from "./notes";

describe("notes", () => {
  it("titles a note by its first non-empty line", () => {
    expect(noteTitle("\n\n  Groceries  \n\nmilk")).toBe("Groceries");
    expect(noteTitle("   \n")).toBe("Untitled");
  });

  it("counts characters and words with singular labels", () => {
    expect(countLabel("one two  three", "words")).toBe("3 words");
    expect(countLabel("a\n\n", "characters")).toBe("1 character");
    expect(countLabel("x".repeat(1200), "characters")).toBe("1,200 characters");
  });

  it("picks the most recently opened note as current and groups pinned first", () => {
    const a = { ...newNote(1, "# A", "A"), lastAccessedAt: 5 };
    const b = { ...newNote(2, "", ""), lastAccessedAt: 9 };
    const c = { ...newNote(3, "C", "C"), lastAccessedAt: 1, pinned: true };
    expect(currentNote([a, b, c])).toBe(b);
    expect(browseSections([a, b, c]).map((s) => [s.title, s.notes.map((n) => n.text)])).toEqual([
      ["Pinned", ["C"]],
      ["Notes", ["", "A"]],
    ]);
    expect(emptyNote([a, b, c])).toBe(b);
  });
});
