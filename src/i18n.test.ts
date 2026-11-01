import { afterEach, describe, expect, it } from "vitest";
import { detectLanguage, setLanguage, t } from "./i18n";

afterEach(() => setLanguage("system"));

describe("i18n", () => {
  it("falls back to the key itself in English", () => {
    setLanguage("en");
    expect(t("Add to Favorites")).toBe("Add to Favorites");
    expect(t("Not translated yet")).toBe("Not translated yet");
  });

  it("translates and interpolates in Portuguese", () => {
    setLanguage("pt");
    expect(t("Add to Favorites")).toBe("Adicionar aos favoritos");
    expect(t("No results for “{query}”", { query: "abc" })).toBe("Nenhum resultado para “abc”");
    expect(t("Unknown key")).toBe("Unknown key");
  });

  it("detects Portuguese system locales", () => {
    expect(detectLanguage("system", "pt-BR")).toBe("pt");
    expect(detectLanguage("system", "en-US")).toBe("en");
    expect(detectLanguage("en", "pt-BR")).toBe("en");
    expect(detectLanguage("pt", "en-US")).toBe("pt");
  });
});
