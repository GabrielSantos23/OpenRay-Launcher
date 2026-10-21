import { describe, expect, it } from "vitest";
import { definitionText, guessUrl, parseEntries, parseSuggestions, remember, suggestionUrl } from "./dictionary";

describe("dictionary", () => {
  it("reads Datamuse words with their first definition and puts the exact word first", () => {
    const body = [{ word: "help", defs: ["v\tTo provide assistance. "] }, { word: "hel", defs: ["n\tA word."] }, { word: "helm" }];
    expect(parseSuggestions("Hel", body, false)).toEqual([
      { word: "hel", definition: "A word.", isGuess: false },
      { word: "help", definition: "To provide assistance.", isGuess: false },
      { word: "helm", definition: "", isGuess: false },
    ]);
    expect(parseSuggestions("x", { error: true }, false)).toEqual([]);
  });

  it("drops the typed word from guesses and shortens long definitions", () => {
    expect(parseSuggestions("helo", [{ word: "helo" }, { word: "hello" }], true)).toEqual([{ word: "hello", definition: "", isGuess: true }]);
    expect(parseSuggestions("a", [{ word: "a", defs: [`n\t${"x".repeat(130)}`] }], false)[0].definition).toHaveLength(121);
  });

  it("encodes the query in the request urls", () => {
    expect(suggestionUrl(" ice cream ", 8)).toBe("https://api.datamuse.com/words?sp=ice%20cream*&md=d&max=8");
    expect(guessUrl("helo", 8)).toBe("https://api.datamuse.com/sug?s=helo&max=8");
  });

  it("parses dictionaryapi.dev entries with phonetics and examples", () => {
    const body = [
      {
        word: "hello",
        phonetics: [{ audio: "" }, { text: "/həˈləʊ/" }],
        meanings: [{ partOfSpeech: "noun", definitions: [{ definition: "A greeting.", example: "she said hello" }, { definition: "" }], synonyms: ["greeting"], antonyms: [] }],
        sourceUrls: ["https://en.wiktionary.org/wiki/hello"],
      },
    ];
    const entries = parseEntries(body);
    expect(entries).toEqual([
      { word: "hello", phonetic: "/həˈləʊ/", meanings: [{ partOfSpeech: "noun", senses: [{ definition: "A greeting.", example: "she said hello" }], synonyms: ["greeting"], antonyms: [] }], sourceUrl: "https://en.wiktionary.org/wiki/hello" },
    ]);
    expect(definitionText(entries)).toBe('hello /həˈləʊ/\n\nnoun\n1. A greeting. "she said hello"');
    expect(parseEntries({ title: "No Definitions Found" })).toEqual([]);
  });

  it("keeps recent words unique and newest first", () => {
    const a = { word: "a", definition: "", isGuess: false };
    const b = { word: "b", definition: "", isGuess: true };
    expect(remember([a, b], { ...b })).toEqual([{ ...b, isGuess: false }, a]);
    expect(remember([a], a)).toEqual([a]);
  });
});
