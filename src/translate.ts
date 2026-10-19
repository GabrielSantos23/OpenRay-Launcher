export const AUTO = "auto";
export const DEFAULT_TARGET = "en";
export const DEBOUNCE_MS = 330;

export interface Language {
  value: string;
  label: string;
}

export interface LanguagePair {
  source: string;
  target: string;
}

export const LANGUAGES: Language[] = [
  { value: "af", label: "Afrikaans" },
  { value: "ak", label: "Akan" },
  { value: "sq", label: "Albanian" },
  { value: "am", label: "Amharic" },
  { value: "ar", label: "Arabic" },
  { value: "hy", label: "Armenian" },
  { value: "as", label: "Assamese" },
  { value: "ay", label: "Aymara" },
  { value: "az", label: "Azerbaijani" },
  { value: "bm", label: "Bambara" },
  { value: "eu", label: "Basque" },
  { value: "be", label: "Belarusian" },
  { value: "bn", label: "Bengali" },
  { value: "bho", label: "Bhojpuri" },
  { value: "bs", label: "Bosnian" },
  { value: "bg", label: "Bulgarian" },
  { value: "ca", label: "Catalan" },
  { value: "ceb", label: "Cebuano" },
  { value: "ny", label: "Chichewa" },
  { value: "zh-CN", label: "Chinese (Simplified)" },
  { value: "zh-TW", label: "Chinese (Traditional)" },
  { value: "co", label: "Corsican" },
  { value: "hr", label: "Croatian" },
  { value: "cs", label: "Czech" },
  { value: "da", label: "Danish" },
  { value: "dv", label: "Divehi" },
  { value: "doi", label: "Dogri" },
  { value: "nl", label: "Dutch" },
  { value: "en", label: "English" },
  { value: "eo", label: "Esperanto" },
  { value: "et", label: "Estonian" },
  { value: "ee", label: "Ewe" },
  { value: "tl", label: "Filipino" },
  { value: "fi", label: "Finnish" },
  { value: "fr", label: "French" },
  { value: "fy", label: "Frisian" },
  { value: "gl", label: "Galician" },
  { value: "lg", label: "Ganda" },
  { value: "ka", label: "Georgian" },
  { value: "de", label: "German" },
  { value: "gom", label: "Goan Konkani" },
  { value: "el", label: "Greek" },
  { value: "gn", label: "Guarani" },
  { value: "gu", label: "Gujarati" },
  { value: "ht", label: "Haitian Creole" },
  { value: "ha", label: "Hausa" },
  { value: "haw", label: "Hawaiian" },
  { value: "iw", label: "Hebrew" },
  { value: "hi", label: "Hindi" },
  { value: "hmn", label: "Hmong" },
  { value: "hu", label: "Hungarian" },
  { value: "is", label: "Icelandic" },
  { value: "ig", label: "Igbo" },
  { value: "ilo", label: "Iloko" },
  { value: "id", label: "Indonesian" },
  { value: "ga", label: "Irish" },
  { value: "it", label: "Italian" },
  { value: "ja", label: "Japanese" },
  { value: "jw", label: "Javanese" },
  { value: "kn", label: "Kannada" },
  { value: "kk", label: "Kazakh" },
  { value: "km", label: "Khmer" },
  { value: "rw", label: "Kinyarwanda" },
  { value: "ko", label: "Korean" },
  { value: "kri", label: "Krio" },
  { value: "ku", label: "Kurdish (Kurmanji)" },
  { value: "ckb", label: "Kurdish (Sorani)" },
  { value: "ky", label: "Kyrgyz" },
  { value: "lo", label: "Lao" },
  { value: "la", label: "Latin" },
  { value: "lv", label: "Latvian" },
  { value: "ln", label: "Lingala" },
  { value: "lt", label: "Lithuanian" },
  { value: "lb", label: "Luxembourgish" },
  { value: "mk", label: "Macedonian" },
  { value: "mai", label: "Maithili" },
  { value: "mg", label: "Malagasy" },
  { value: "ms", label: "Malay" },
  { value: "ml", label: "Malayalam" },
  { value: "mt", label: "Maltese" },
  { value: "mni-Mtei", label: "Manipuri (Meitei Mayek)" },
  { value: "mi", label: "Maori" },
  { value: "mr", label: "Marathi" },
  { value: "lus", label: "Mizo" },
  { value: "mn", label: "Mongolian" },
  { value: "my", label: "Myanmar (Burmese)" },
  { value: "ne", label: "Nepali" },
  { value: "nso", label: "Northern Sotho" },
  { value: "no", label: "Norwegian" },
  { value: "or", label: "Odia (Oriya)" },
  { value: "om", label: "Oromo" },
  { value: "ps", label: "Pashto" },
  { value: "fa", label: "Persian" },
  { value: "pl", label: "Polish" },
  { value: "pt", label: "Portuguese" },
  { value: "pa", label: "Punjabi" },
  { value: "qu", label: "Quechua" },
  { value: "ro", label: "Romanian" },
  { value: "ru", label: "Russian" },
  { value: "sm", label: "Samoan" },
  { value: "sa", label: "Sanskrit" },
  { value: "gd", label: "Scots Gaelic" },
  { value: "sr", label: "Serbian" },
  { value: "st", label: "Sesotho" },
  { value: "sn", label: "Shona" },
  { value: "sd", label: "Sindhi" },
  { value: "si", label: "Sinhala" },
  { value: "sk", label: "Slovak" },
  { value: "sl", label: "Slovenian" },
  { value: "so", label: "Somali" },
  { value: "es", label: "Spanish" },
  { value: "su", label: "Sundanese" },
  { value: "sw", label: "Swahili" },
  { value: "sv", label: "Swedish" },
  { value: "tg", label: "Tajik" },
  { value: "ta", label: "Tamil" },
  { value: "tt", label: "Tatar" },
  { value: "te", label: "Telugu" },
  { value: "th", label: "Thai" },
  { value: "ti", label: "Tigrinya" },
  { value: "ts", label: "Tsonga" },
  { value: "tr", label: "Turkish" },
  { value: "tk", label: "Turkmen" },
  { value: "uk", label: "Ukrainian" },
  { value: "ur", label: "Urdu" },
  { value: "ug", label: "Uyghur" },
  { value: "uz", label: "Uzbek" },
  { value: "vi", label: "Vietnamese" },
  { value: "cy", label: "Welsh" },
  { value: "xh", label: "Xhosa" },
  { value: "yi", label: "Yiddish" },
  { value: "yo", label: "Yoruba" },
  { value: "zu", label: "Zulu" },
];

const ALIASES: Record<string, string> = { zh: "zh-CN", he: "iw", jv: "jw", fil: "tl", nb: "no" };

export function findLanguage(code: string | undefined): string | undefined {
  if (!code) return undefined;
  const exact = LANGUAGES.find((l) => l.value.toLowerCase() === code.toLowerCase());
  if (exact) return exact.value;
  const base = code.split("-")[0].toLowerCase();
  return ALIASES[base] ?? LANGUAGES.find((l) => l.value === base)?.value;
}

export function languageLabel(code: string): string {
  return LANGUAGES.find((l) => l.value === findLanguage(code))?.label ?? code;
}

export function restoreLanguages(stored: unknown): LanguagePair {
  const value = stored && typeof stored === "object" ? (stored as Partial<Record<keyof LanguagePair, unknown>>) : {};
  const source = typeof value.source === "string" && value.source !== AUTO ? findLanguage(value.source) : undefined;
  const target = typeof value.target === "string" ? findLanguage(value.target) : undefined;
  return { source: source ?? AUTO, target: target ?? DEFAULT_TARGET };
}

export function sameLanguage({ source, target }: LanguagePair): boolean {
  return source !== AUTO && source === target;
}

export function swapLanguages({ source, target }: LanguagePair, detected: string | undefined): LanguagePair | null {
  const from = source === AUTO ? findLanguage(detected) : source;
  return from ? { source: target, target: from } : null;
}

export function countLabel(text: string): string {
  const words = text.trim() ? text.trim().split(/\s+/).length : 0;
  const characters = [...text].length;
  return `${words} ${words === 1 ? "Word" : "Words"} · ${characters} ${characters === 1 ? "Character" : "Characters"}`;
}
