import { describe, expect, it } from "vitest";
import { definitionText, guessUrl, parseEntries, parseOpenSearch, parseSearch, parseSuggestions, parseWikitext, remember, suggestionUrl, wiktionaryUrl } from "./dictionary";

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

  it("reads Portuguese suggestions from Wiktionary opensearch and search", () => {
    expect(parseOpenSearch("Sauda", ["sauda", ["saudação", "saudade", "sauda", "saudade"], [], []]).map((w) => w.word)).toEqual(["sauda", "saudação", "saudade"]);
    expect(parseSearch("acao", { query: { search: [{ title: "acao" }, { title: "ação" }] } })).toEqual([{ word: "ação", definition: "", isGuess: true }]);
    expect(parseOpenSearch("x", { error: 1 })).toEqual([]);
    expect(wiktionaryUrl({ action: "parse", page: "ação" })).toBe("https://pt.wiktionary.org/w/api.php?action=parse&page=a%C3%A7%C3%A3o&format=json&formatversion=2&origin=*");
  });

  it("parses the Portuguese section of a Wiktionary entry", () => {
    const wikitext = [
      "={{-pt-}}=",
      "==Substantivo==",
      "{{flex.pt|fs=ação|fp=ações}}",
      "# efeito de [[atuar]]",
      "#: ''Foi uma '''ação''' rápida.''",
      "# {{escopo|pt|Economia}} [[papel]] que [[representar|representa]] [[parte]] do capital",
      "===Sinônimos===",
      "* De '''1''' (efeito de atuar): [[ato]], [[feito]]",
      "===Tradução===",
      "* {{t|en|action}}",
      "==Pronúncia==",
      "* AFI: {{AFI|/a.ˈsɐ̃w̃/}}",
      "# não é definição",
      "={{-es-}}=",
      "==Substantivo==",
      "# otra lengua",
    ].join("\n");
    expect(parseWikitext("ação", wikitext)).toEqual([
      {
        word: "ação",
        phonetic: "/a.ˈsɐ̃w̃/",
        meanings: [
          {
            partOfSpeech: "substantivo",
            senses: [{ definition: "efeito de atuar", example: "Foi uma ação rápida." }, { definition: "(Economia) papel que representa parte do capital" }],
            synonyms: ["ato", "feito"],
            antonyms: [],
          },
        ],
        sourceUrl: "https://pt.wiktionary.org/wiki/a%C3%A7%C3%A3o",
      },
    ]);
    expect(parseWikitext("house", "={{-en-}}=\n==Noun==\n# a building")).toEqual([]);
  });
});
