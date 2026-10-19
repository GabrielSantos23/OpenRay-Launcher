import { describe, expect, it } from "vitest";
import { AUTO, countLabel, findLanguage, LANGUAGES, languageLabel, restoreLanguages, sameLanguage, swapLanguages } from "./translate";

describe("language list", () => {
  it("has unique codes and sorted labels", () => {
    expect(new Set(LANGUAGES.map((l) => l.value)).size).toBe(LANGUAGES.length);
    const labels = LANGUAGES.map((l) => l.label);
    expect(labels).toEqual([...labels].sort((a, b) => a.localeCompare(b)));
  });

  it("maps detected codes onto list values", () => {
    expect(findLanguage("fr")).toBe("fr");
    expect(findLanguage("zh-cn")).toBe("zh-CN");
    expect(findLanguage("zh")).toBe("zh-CN");
    expect(findLanguage("he")).toBe("iw");
    expect(findLanguage("pt-BR")).toBe("pt");
    expect(findLanguage("xx")).toBeUndefined();
    expect(findLanguage(undefined)).toBeUndefined();
  });

  it("labels known codes and falls back to the code", () => {
    expect(languageLabel("en")).toBe("English");
    expect(languageLabel("zh-TW")).toBe("Chinese (Traditional)");
    expect(languageLabel("xx")).toBe("xx");
  });
});

describe("restoreLanguages", () => {
  it("defaults to detect and English", () => {
    expect(restoreLanguages(null)).toEqual({ source: AUTO, target: "en" });
    expect(restoreLanguages("junk")).toEqual({ source: AUTO, target: "en" });
  });

  it("keeps valid saved languages and drops unknown ones", () => {
    expect(restoreLanguages({ source: "de", target: "fr" })).toEqual({ source: "de", target: "fr" });
    expect(restoreLanguages({ source: "auto", target: "ja" })).toEqual({ source: AUTO, target: "ja" });
    expect(restoreLanguages({ source: "xx", target: 3 })).toEqual({ source: AUTO, target: "en" });
  });
});

describe("swapLanguages", () => {
  it("swaps explicit languages", () => {
    expect(swapLanguages({ source: "en", target: "fr" }, undefined)).toEqual({ source: "fr", target: "en" });
  });

  it("uses the detected language when detecting", () => {
    expect(swapLanguages({ source: AUTO, target: "fr" }, "en")).toEqual({ source: "fr", target: "en" });
    expect(swapLanguages({ source: AUTO, target: "fr" }, undefined)).toBeNull();
  });

  it("treats identical explicit languages as no translation", () => {
    expect(sameLanguage({ source: "fr", target: "fr" })).toBe(true);
    expect(sameLanguage({ source: AUTO, target: "fr" })).toBe(false);
  });
});

describe("countLabel", () => {
  it("counts words and characters", () => {
    expect(countLabel("Hello world")).toBe("2 Words · 11 Characters");
    expect(countLabel("a")).toBe("1 Word · 1 Character");
    expect(countLabel("  ")).toBe("0 Words · 2 Characters");
  });
});
