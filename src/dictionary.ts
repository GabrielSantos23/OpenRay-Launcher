export const SUGGEST_URL = "https://api.datamuse.com";
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
