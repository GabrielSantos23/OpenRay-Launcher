import { invoke } from "@tauri-apps/api/core";

export const SUGGEST_URL = "https://api.datamuse.com";
export const WIKTIONARY_URL = "https://pt.wiktionary.org";
const STORE = "dictionary";
export const RECENT_LIMIT = 100;
const SNIPPET_LIMIT = 120;

export interface Suggestion {
  word: string;
  definition: string;
  isGuess: boolean;
}

export interface Sense {
  definition: string;
  example?: string;
}

export interface Meaning {
  partOfSpeech: string;
  senses: Sense[];
  synonyms: string[];
  antonyms: string[];
}

export interface Entry {
  word: string;
  phonetic: string;
  meanings: Meaning[];
  sourceUrl?: string;
}

interface DatamuseWord {
  word?: string;
  defs?: string[];
}

function snippet(text: string): string {
  const trimmed = text.trim();
  return trimmed.length > SNIPPET_LIMIT ? `${trimmed.slice(0, SNIPPET_LIMIT)}…` : trimmed;
}

export function parseSuggestions(query: string, body: unknown, isGuess: boolean): Suggestion[] {
  if (!Array.isArray(body)) return [];
  const wanted = query.trim().toLowerCase();
  const words = (body as DatamuseWord[])
    .filter((w) => typeof w.word === "string" && (!isGuess || w.word.toLowerCase() !== wanted))
    .map((w) => ({ word: w.word!, definition: snippet(w.defs?.[0]?.split("\t").slice(1).join("\t") ?? ""), isGuess }));
  const exact = words.findIndex((w) => w.word.toLowerCase() === wanted);
  return exact > 0 ? [words[exact], ...words.slice(0, exact), ...words.slice(exact + 1)] : words;
}

export function suggestionUrl(query: string, limit: number): string {
  return `${SUGGEST_URL}/words?sp=${encodeURIComponent(`${query.trim()}*`)}&md=d&max=${limit}`;
}

export function guessUrl(query: string, limit: number): string {
  return `${SUGGEST_URL}/sug?s=${encodeURIComponent(query.trim())}&max=${limit}`;
}

interface RawEntry {
  word?: string;
  phonetic?: string;
  phonetics?: { text?: string }[];
  meanings?: { partOfSpeech?: string; definitions?: { definition?: string; example?: string }[]; synonyms?: string[]; antonyms?: string[] }[];
  sourceUrls?: string[];
}

export function parseEntries(body: unknown): Entry[] {
  if (!Array.isArray(body)) return [];
  return (body as RawEntry[]).map((e) => ({
    word: e.word ?? "",
    phonetic: e.phonetic || e.phonetics?.find((p) => p.text)?.text || "",
    meanings: (e.meanings ?? []).map((m) => ({
      partOfSpeech: m.partOfSpeech ?? "",
      senses: (m.definitions ?? []).filter((d) => d.definition).map((d) => ({ definition: d.definition!, example: d.example || undefined })),
      synonyms: m.synonyms ?? [],
      antonyms: m.antonyms ?? [],
    })),
    sourceUrl: e.sourceUrls?.[0],
  }));
}

export function definitionText(entries: Entry[]): string {
  return entries
    .map((e) =>
      [
        [e.word, e.phonetic].filter(Boolean).join(" "),
        ...e.meanings.map((m) => [m.partOfSpeech, ...m.senses.map((s, i) => `${i + 1}. ${s.definition}${s.example ? ` "${s.example}"` : ""}`)].join("\n")),
      ].join("\n\n"),
    )
    .join("\n\n");
}

export function remember(recent: Suggestion[], word: Suggestion): Suggestion[] {
  if (recent[0]?.word === word.word) return recent;
  return [{ ...word, isGuess: false }, ...recent.filter((w) => w.word !== word.word)].slice(0, RECENT_LIMIT);
}

export type DictionaryLanguage = "en" | "pt";

export const DICTIONARY_LANGUAGES: { value: DictionaryLanguage; label: string }[] = [
  { value: "en", label: "English" },
  { value: "pt", label: "Português" },
];

export interface DictionaryData {
  recent: Suggestion[];
  language: DictionaryLanguage;
}

export async function loadDictionary(): Promise<DictionaryData> {
  const stored = await invoke<Partial<DictionaryData> | null>("data_get", { name: STORE }).catch(() => null);
  return { recent: stored?.recent ?? [], language: stored?.language === "pt" ? "pt" : "en" };
}

export async function saveDictionary(patch: Partial<DictionaryData>) {
  await invoke("data_set", { name: STORE, value: { ...(await loadDictionary()), ...patch } });
}

export function wiktionaryUrl(params: Record<string, string>): string {
  return `${WIKTIONARY_URL}/w/api.php?${new URLSearchParams({ ...params, format: "json", formatversion: "2", origin: "*" })}`;
}

function titles(query: string, list: string[], isGuess: boolean): Suggestion[] {
  const wanted = query.trim().toLowerCase();
  const words = [...new Set(list)].filter((w) => !isGuess || w.toLowerCase() !== wanted).map((word) => ({ word, definition: "", isGuess }));
  const exact = words.findIndex((w) => w.word.toLowerCase() === wanted);
  return exact > 0 ? [words[exact], ...words.slice(0, exact), ...words.slice(exact + 1)] : words;
}

export function parseOpenSearch(query: string, body: unknown): Suggestion[] {
  return Array.isArray(body) && Array.isArray(body[1]) ? titles(query, body[1].filter((t: unknown) => typeof t === "string"), false) : [];
}

export function parseSearch(query: string, body: unknown): Suggestion[] {
  const results = (body as { query?: { search?: { title?: string }[] } })?.query?.search ?? [];
  return titles(query, results.map((r) => r.title).filter((t): t is string => typeof t === "string"), true);
}

function cleanWikitext(text: string): string {
  return text
    .replace(/\{\{escopo2?\|(?:[a-z-]+\|)?([^}|]+)[^}]*\}\}/g, "($1)")
    .replace(/\{\{[^{}]*\}\}/g, "")
    .replace(/\[\[[^\]|]*\|([^\]]*)\]\]/g, "$1")
    .replace(/\[\[([^\]]*)\]\]/g, "$1")
    .replace(/'{2,}/g, "")
    .replace(/<[^>]+>/g, "")
    .replace(/\s+/g, " ")
    .trim();
}

const SKIPPED_SECTIONS = /^(Pronúncia|Etimologia|Ver também|Tradução|Referências|Anagramas|Formas alternativas|Expressões)/;

export function parseWikitext(word: string, wikitext: string): Entry[] {
  const start = wikitext.indexOf("={{-pt-}}=");
  if (start < 0) return [];
  const rest = wikitext.slice(start + "={{-pt-}}=".length);
  const next = rest.search(/^=[^=]/m);
  const section = next < 0 ? rest : rest.slice(0, next);
  const meanings: Meaning[] = [];
  let current: Meaning | null = null;
  let synonyms = false;
  for (const raw of section.split("\n")) {
    const line = raw.trim();
    const heading = /^(=+)\s*(.*?)\s*=+$/.exec(line);
    if (heading) {
      const title = cleanWikitext(heading[2]);
      synonyms = heading[1].length === 3 && title.startsWith("Sinônimo");
      if (heading[1].length === 2) {
        current = SKIPPED_SECTIONS.test(title) ? null : { partOfSpeech: title.toLowerCase(), senses: [], synonyms: [], antonyms: [] };
        if (current) meanings.push(current);
      }
    } else if (current && /^#[:*]/.test(line)) {
      const example = cleanWikitext(line.replace(/^#[:*]+/, ""));
      const last = current.senses.at(-1);
      if (last && example && !last.example) last.example = example;
    } else if (current && line.startsWith("#")) {
      const definition = cleanWikitext(line.replace(/^#+/, ""));
      if (definition) current.senses.push({ definition });
    } else if (current && synonyms && line.startsWith("*")) {
      const links = [...line.matchAll(/\[\[(?:[^\]|]*\|)?([^\]]+)\]\]/g)].map((m) => m[1]);
      current.synonyms.push(...links.filter((l) => !current!.synonyms.includes(l)));
    }
  }
  const phonetic = /\{\{AFI\|([^}|]+)/.exec(section)?.[1]?.trim() ?? "";
  const kept = meanings.filter((m) => m.senses.length);
  return kept.length ? [{ word, phonetic, meanings: kept, sourceUrl: `${WIKTIONARY_URL}/wiki/${encodeURIComponent(word)}` }] : [];
}
