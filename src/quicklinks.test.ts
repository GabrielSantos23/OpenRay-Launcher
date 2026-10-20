import { describe, expect, it } from "vitest";
import { displayLink, exportQuicklinks, faviconUrl, fillLink, linkArguments, mergeQuicklinks, parseQuicklinkImport } from "./quicklinks";

describe("quicklinks", () => {
  it("lists named arguments once, with defaults", () => {
    expect(linkArguments('https://google.com/search?q={argument name="Query"}')).toEqual([{ name: "Query", defaultValue: undefined }]);
    expect(linkArguments('x/{argument name="A" default="1"}/{argument name="A"}/{argument}')).toEqual([
      { name: "A", defaultValue: "1" },
      { name: "Query", defaultValue: undefined },
    ]);
    expect(linkArguments("https://example.com")).toEqual([]);
  });

  it("fills arguments and context, encoding values in URLs only", () => {
    const now = new Date(2026, 9, 18, 14, 5);
    expect(fillLink('https://google.com/search?q={argument name="Query"}', { Query: "a b&c" })).toBe("https://google.com/search?q=a%20b%26c");
    expect(fillLink('C:\\notes\\{argument default="today"}.txt', {})).toBe("C:\\notes\\today.txt");
    expect(fillLink("https://x.com/{clipboard}?d={date}", {}, { clipboard: "hi there", now })).toBe("https://x.com/hi%20there?d=Oct%2018%2C%202026");
    expect(fillLink("id={uuid}&{unknown}", {}, { uuid: () => "u1" })).toBe("id=u1&{unknown}");
  });

  it("previews links and finds favicons for web links only", () => {
    expect(displayLink('https://google.com/search?q={argument name="Query"}')).toBe("https://google.com/search?q=_");
    expect(displayLink("https://x.com/{argument}?d={date}", { Query: "a b" })).toBe("https://x.com/a b?d={date}");
    expect(faviconUrl("https://www.github.com/x")).toBe("https://www.google.com/s2/favicons?domain=www.github.com&sz=64");
    expect(faviconUrl("C:\\Users")).toBeNull();
  });

  it("imports the reference shapes, skips duplicate links and round-trips exports", () => {
    expect(parseQuicklinkImport('{"name":"A","url":"https://a.com","applicationID":"Firefox"}')).toEqual([{ name: "A", link: "https://a.com", openWith: "Firefox" }]);
    expect(parseQuicklinkImport('{"quicklinks":[{"name":"B","link":"https://b.com"}]}')).toEqual([{ name: "B", link: "https://b.com" }]);
    expect(() => parseQuicklinkImport('[{"link":"x"}]')).toThrow();
    const apps = [{ name: "Firefox", path: "C:\ff.exe" }];
    const existing = [{ id: "1", name: "Old", link: "https://a.com" }];
    const incoming = parseQuicklinkImport('[{"name":"A","link":"https://a.com"},{"name":"C","link":" https://c.com ","openWith":"firefox"}]');
    const { list, added, skipped } = mergeQuicklinks(existing, incoming, apps, () => "new");
    expect([added, skipped]).toEqual([1, 1]);
    expect(list[1]).toEqual({ id: "new", name: "C", link: "https://c.com", appPath: "C:\ff.exe", appName: "Firefox" });
    expect(parseQuicklinkImport(exportQuicklinks(list))).toEqual([{ name: "Old", link: "https://a.com" }, { name: "C", link: "https://c.com", openWith: "Firefox" }]);
  });
});
