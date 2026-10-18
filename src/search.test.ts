import { describe, expect, it } from "vitest";
import type { RegisteredCommand } from "./extension-api";
import { matchScore, search } from "./search";

function cmd(title: string, keywords: string[] = []): RegisteredCommand {
  return {
    id: title,
    key: `test/${title}`,
    extensionId: "test",
    title,
    keywords,
    kind: "Application",
    primaryActionTitle: "Open Application",
    run: () => {},
  };
}

const vscode = cmd("Visual Studio Code");
const vs = cmd("Visual Studio 2022");
const notepad = cmd("Notepad");
const notion = cmd("Notion");
const terminal = cmd("Windows Terminal", ["wt", "console"]);
const all = [vscode, vs, notepad, notion, terminal];

describe("matchScore", () => {
  it("ranks prefix over word-start over acronym over substring over subsequence", () => {
    expect(matchScore("vis", "Visual Studio Code")).toBe(100);
    expect(matchScore("stu", "Visual Studio Code")).toBe(80);
    expect(matchScore("vsc", "Visual Studio Code")).toBe(70);
    expect(matchScore("sual", "Visual Studio Code")).toBe(50);
    expect(matchScore("vlcd", "Visual Studio Code")).toBe(20);
    expect(matchScore("xyz", "Visual Studio Code")).toBe(0);
  });

  it("is case-insensitive and ignores surrounding space", () => {
    expect(matchScore("  NOTE ", "Notepad")).toBe(100);
  });

  it("matches keywords", () => {
    expect(matchScore("console", "Windows Terminal", ["wt", "console"])).toBe(100);
  });

  it("matches multi-word queries in any order by word prefix", () => {
    expect(matchScore("code vis", "Visual Studio Code")).toBe(60);
    expect(matchScore("code xyz", "Visual Studio Code")).toBe(0);
  });

  it("splits camel case into words", () => {
    expect(matchScore("store", "GitHubStore")).toBe(80);
  });

  it("returns a neutral score for an empty query", () => {
    expect(matchScore("", "Anything")).toBe(1);
  });
});

describe("search", () => {
  it("filters and orders by match quality", () => {
    const [section] = search(all, "no", {});
    expect(section.title).toBe("Results");
    expect(section.items.map((c) => c.title)).toEqual(["Notepad", "Notion", "Windows Terminal"]);
  });

  it("breaks score ties with frecency", () => {
    const [section] = search(all, "no", { [notion.key]: 5 });
    expect(section.items.map((c) => c.title)).toEqual(["Notion", "Notepad", "Windows Terminal"]);
  });

  it("keeps a better text match above a more frequent weaker match", () => {
    const [section] = search(all, "vis", { [terminal.key]: 100 });
    expect(section.items[0].title.startsWith("Visual")).toBe(true);
  });

  it("returns no sections when nothing matches", () => {
    expect(search(all, "zzzz", {})).toEqual([]);
  });

  it("shows frecent commands as suggestions for an empty query", () => {
    const sections = search(all, "", { [notepad.key]: 2, [vs.key]: 9 });
    expect(sections).toHaveLength(1);
    expect(sections[0].title).toBe("Suggestions");
    expect(sections[0].items.map((c) => c.title)).toEqual(["Visual Studio 2022", "Notepad"]);
  });

  it("caps suggestions at eight", () => {
    const many = Array.from({ length: 12 }, (_, i) => cmd(`App ${i}`));
    const frecency = Object.fromEntries(many.map((c, i) => [c.key, i + 1]));
    expect(search(many, "", frecency)[0].items).toHaveLength(8);
  });

  it("shows favorites in their saved order above suggestions that exclude them", () => {
    const sections = search(all, "", { [notepad.key]: 2, [vs.key]: 9 }, [terminal.key, vs.key, "gone/app"]);
    expect(sections.map((s) => [s.title, s.items.map((c) => c.title)])).toEqual([
      ["Favorites", ["Windows Terminal", "Visual Studio 2022"]],
      ["Suggestions", ["Notepad"]],
    ]);
  });

  it("lists everything alphabetically for an empty query without history", () => {
    const sections = search(all, "", {});
    expect(sections[0].title).toBe("Results");
    expect(sections[0].items.map((c) => c.title)).toEqual([
      "Notepad",
      "Notion",
      "Visual Studio 2022",
      "Visual Studio Code",
      "Windows Terminal",
    ]);
  });
});
