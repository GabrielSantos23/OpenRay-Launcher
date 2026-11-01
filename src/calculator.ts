const FUNCTIONS: Record<string, (x: number) => number> = {
  sqrt: Math.sqrt,
  cbrt: Math.cbrt,
  abs: Math.abs,
  round: Math.round,
  floor: Math.floor,
  ceil: Math.ceil,
  exp: Math.exp,
  ln: Math.log,
  log: Math.log10,
  log2: Math.log2,
  sin: Math.sin,
  cos: Math.cos,
  tan: Math.tan,
  asin: Math.asin,
  acos: Math.acos,
  atan: Math.atan,
};

const CONSTANTS: Record<string, number> = { pi: Math.PI, "π": Math.PI, e: Math.E, tau: 2 * Math.PI };

type Token = { type: "num"; value: number; percent?: boolean } | { type: "op"; value: string } | { type: "id"; value: string };

function tokenize(input: string): Token[] | null {
  const tokens: Token[] = [];
  const source = input.replace(/×/g, "*").replace(/÷/g, "/").replace(/\*\*/g, "^");
  let i = 0;
  while (i < source.length) {
    const rest = source.slice(i);
    const space = rest.match(/^\s+/);
    if (space) {
      i += space[0].length;
      continue;
    }
    const number = rest.match(/^(\d{1,3}(,\d{3})+|\d+)(\.\d+)?|^\.\d+/);
    if (number) {
      tokens.push({ type: "num", value: Number(number[0].replace(/,/g, "")) });
      i += number[0].length;
      continue;
    }
    const word = rest.match(/^[a-zπ][a-z0-9]*/i);
    if (word) {
      tokens.push({ type: "id", value: word[0].toLowerCase() });
      i += word[0].length;
      continue;
    }
    if ("+-*/^%()!".includes(source[i])) {
      tokens.push({ type: "op", value: source[i] });
      i += 1;
      continue;
    }
    return null;
  }
  return tokens;
}

function factorial(n: number): number {
  if (!Number.isInteger(n) || n < 0 || n > 170) return NaN;
  let result = 1;
  for (let k = 2; k <= n; k++) result *= k;
  return result;
}

export function evaluate(input: string): number | null {
  const tokens = tokenize(input) ?? [];
  if (tokens.length === 0) return null;
  let pos = 0;
  const peek = () => tokens[pos];
  const isOp = (value: string) => peek()?.type === "op" && peek()!.value === value;

  type Value = { value: number; percent: boolean };

  function primary(): Value {
    const token = tokens[pos++];
    if (!token) throw new Error("unexpected end");
    if (token.type === "num") return { value: token.value, percent: false };
    if (token.type === "op" && token.value === "(") {
      const inner = expression();
      if (!isOp(")")) throw new Error("missing )");
      pos++;
      return { value: inner, percent: false };
    }
    if (token.type === "id") {
      if (token.value in CONSTANTS) return { value: CONSTANTS[token.value], percent: false };
      const fn = FUNCTIONS[token.value];
      if (fn) return { value: fn(unary().value), percent: false };
    }
    throw new Error("unexpected token");
  }

  function postfix(): Value {
    let result = primary();
    for (;;) {
      if (isOp("!")) {
        pos++;
        result = { value: factorial(result.value), percent: false };
      } else if (isOp("%")) {
        pos++;
        result = { value: result.value / 100, percent: true };
      } else if (peek()?.type === "id" || isOp("(")) {
        result = { value: result.value * postfix().value, percent: false };
      } else return result;
    }
  }

  function power(): Value {
    const base = postfix();
    if (!isOp("^")) return base;
    pos++;
    return { value: Math.pow(base.value, unary().value), percent: false };
  }

  function unary(): Value {
    if (isOp("-")) {
      pos++;
      const value = unary();
      return { value: -value.value, percent: value.percent };
    }
    if (isOp("+")) {
      pos++;
      return unary();
    }
    return power();
  }

  function term(): number {
    let result = unary().value;
    while (isOp("*") || isOp("/")) {
      const op = tokens[pos++].value;
      const right = unary().value;
      result = op === "*" ? result * right : result / right;
    }
    return result;
  }

  function expression(): number {
    let result = term();
    while (isOp("+") || isOp("-")) {
      const op = tokens[pos++].value;
      const start = pos;
      const right = unary();
      const isPlainPercent = right.percent && (isOp("+") || isOp("-") || isOp(")") || pos >= tokens.length);
      const amount = isPlainPercent ? result * right.value : (pos = start, term());
      result = op === "+" ? result + amount : result - amount;
    }
    return result;
  }

  try {
    const result = expression();
    if (pos !== tokens.length || !Number.isFinite(result)) return null;
    return result;
  } catch {
    return null;
  }
}

export function isExpression(input: string): boolean {
  const tokens = tokenize(input);
  if (!tokens || tokens.length < 2) return !!tokens && tokens.length === 1 && tokens[0].type === "id" && tokens[0].value in CONSTANTS && input.trim().length > 1;
  return tokens.some((t) => t.type === "op" || (t.type === "id" && (t.value in FUNCTIONS || t.value in CONSTANTS)));
}

export function formatNumber(value: number): string {
  const rounded = Number(value.toPrecision(12));
  return new Intl.NumberFormat("en-US", { maximumFractionDigits: 10 }).format(rounded);
}

export function plainNumber(value: number): string {
  return String(Number(value.toPrecision(12)));
}

const ONES = ["Zero", "One", "Two", "Three", "Four", "Five", "Six", "Seven", "Eight", "Nine", "Ten", "Eleven", "Twelve", "Thirteen", "Fourteen", "Fifteen", "Sixteen", "Seventeen", "Eighteen", "Nineteen"];
const TENS = ["", "", "Twenty", "Thirty", "Forty", "Fifty", "Sixty", "Seventy", "Eighty", "Ninety"];
const SCALES = ["", "Thousand", "Million", "Billion", "Trillion"];

function belowThousand(n: number): string {
  const hundreds = Math.floor(n / 100);
  const rest = n % 100;
  const parts = hundreds ? [`${ONES[hundreds]} Hundred`] : [];
  if (rest >= 20) parts.push(TENS[Math.floor(rest / 10)] + (rest % 10 ? ` ${ONES[rest % 10]}` : ""));
  else if (rest > 0 || parts.length === 0) parts.push(ONES[rest]);
  return parts.join(" ");
}

export function numberToWords(value: number): string | null {
  if (!Number.isInteger(value) || Math.abs(value) >= 1e15) return null;
  if (value === 0) return "Zero";
  const parts: string[] = [];
  let n = Math.abs(value);
  for (let scale = 0; n > 0; scale++, n = Math.floor(n / 1000)) {
    const chunk = n % 1000;
    if (chunk) parts.unshift(belowThousand(chunk) + (SCALES[scale] ? ` ${SCALES[scale]}` : ""));
  }
  return (value < 0 ? "Minus " : "") + parts.join(" ");
}

const PT_ONES = ["zero", "um", "dois", "três", "quatro", "cinco", "seis", "sete", "oito", "nove", "dez", "onze", "doze", "treze", "quatorze", "quinze", "dezesseis", "dezessete", "dezoito", "dezenove"];
const PT_TENS = ["", "", "vinte", "trinta", "quarenta", "cinquenta", "sessenta", "setenta", "oitenta", "noventa"];
const PT_HUNDREDS = ["", "cento", "duzentos", "trezentos", "quatrocentos", "quinhentos", "seiscentos", "setecentos", "oitocentos", "novecentos"];

function ptBelowThousand(n: number): string {
  const hundreds = Math.floor(n / 100);
  const rest = n % 100;
  const parts = hundreds ? [hundreds === 1 && rest === 0 ? "cem" : PT_HUNDREDS[hundreds]] : [];
  if (rest >= 20) parts.push(PT_TENS[Math.floor(rest / 10)] + (rest % 10 ? ` e ${PT_ONES[rest % 10]}` : ""));
  else if (rest > 0 || parts.length === 0) parts.push(PT_ONES[rest]);
  return parts.join(" e ");
}

function ptScale(n: number, scale: number): string {
  const words = ptBelowThousand(n);
  if (scale === 0) return words;
  if (scale === 1) return n === 1 ? "mil" : `${words} mil`;
  const plural = n > 1;
  return `${n === 1 ? "um" : words} ${scale === 2 ? (plural ? "milhões" : "milhão") : plural ? "bilhões" : "bilhão"}`;
}

export function numberToWordsPt(value: number): string | null {
  if (!Number.isInteger(value) || Math.abs(value) >= 1e15) return null;
  if (value === 0) return "Zero";
  const chunks: { n: number; scale: number }[] = [];
  let n = Math.abs(value);
  for (let scale = 0; n > 0; scale++, n = Math.floor(n / 1000)) if (n % 1000) chunks.unshift({ n: n % 1000, scale });
  const joined = chunks
    .map((chunk, i) => {
      const word = ptScale(chunk.n, chunk.scale);
      const rest = chunks.slice(i + 1).filter((c) => c.n > 0);
      const remainder = rest.length ? rest[rest.length - 1].n : 0;
      const singleRemainder = rest.length === 1;
      const needsE = singleRemainder && (remainder < 100 || remainder % 100 === 0);
      return needsE ? `${word} e` : word;
    })
    .join(" ");
  return (value < 0 ? "menos " : "") + joined;
}

export const CURRENCY_SYMBOLS: Record<string, string> = {
  "R$": "BRL",
  "US$": "USD",
  "C$": "CAD",
  "A$": "AUD",
  "$": "USD",
  "€": "EUR",
  "£": "GBP",
  "¥": "JPY",
  "₹": "INR",
  "₩": "KRW",
  "₽": "RUB",
  "₺": "TRY",
  "₿": "BTC",
};

export interface CurrencyQuery {
  amount: number;
  from: string;
  to: string;
}

const CURRENCY_WORDS: Record<string, string> = {
  dolar: "USD",
  dolares: "USD",
  dollars: "USD",
  dollar: "USD",
  real: "BRL",
  reais: "BRL",
  euro: "EUR",
  euros: "EUR",
  libra: "GBP",
  libras: "GBP",
  pound: "GBP",
  pounds: "GBP",
  iene: "JPY",
  ienes: "JPY",
  yen: "JPY",
  yens: "JPY",
  yuan: "CNY",
  won: "KRW",
  rublo: "RUB",
  rublos: "RUB",
  bitcoin: "BTC",
  bitcoins: "BTC",
  ethereum: "ETH",
  ether: "ETH",
  solana: "SOL",
  ripple: "XRP",
  dogecoin: "DOGE",
  cardano: "ADA",
  litecoin: "LTC",
  tether: "USDT",
};

export const CURRENCY_CONNECTORS = ["in", "to", "as", "para", "em", "como", "pra", "pro"];

export function unaccent(text: string): string {
  return text.normalize("NFD").replace(/[\u0300-\u036f]/g, "");
}

function currencyCode(token: string, known: Set<string>): string | null {
  const word = CURRENCY_WORDS[unaccent(token.toLowerCase())];
  if (word) return word;
  const symbol = CURRENCY_SYMBOLS[token.toUpperCase()] ?? CURRENCY_SYMBOLS[token];
  if (symbol) return symbol;
  const code = token.toUpperCase();
  return known.has(code) ? code : null;
}

export function parseCurrency(query: string, known: Set<string>): CurrencyQuery | null {
  const symbols = Object.keys(CURRENCY_SYMBOLS)
    .map((s) => s.replace(/[$]/g, "\\$"))
    .join("|");
  const unit = `(?:${symbols}|\\p{L}+)`;
  const connector = CURRENCY_CONNECTORS.join("|");
  const match = query
    .trim()
    .match(new RegExp(`^(${unit})?\\s*(\\d+(?:[.,]\\d+)?)\\s*(${unit})?\\s+(?:${connector}|=)\\s+(${unit})$`, "iu"));
  if (!match) return null;
  const [, before, amountText, after, target] = match;
  if (!!before === !!after) return null;
  const from = currencyCode(before ?? after, known);
  const to = currencyCode(target, known);
  if (!from || !to || from === to) return null;
  return { amount: Number(amountText.replace(",", ".")), from, to };
}

export function convertCurrency(query: CurrencyQuery, rates: Record<string, number>): number | null {
  const from = rates[query.from];
  const to = rates[query.to];
  return from && to ? (query.amount / from) * to : null;
}

export function capitalize(text: string): string {
  return text.charAt(0).toLocaleUpperCase() + text.slice(1);
}

export interface Place {
  name: string;
  timeZone: string;
}

function regionZones(code: string): string[] {
  const locale = new Intl.Locale(`und-${code}`) as Intl.Locale & { getTimeZones?: () => string[]; timeZones?: string[] };
  return locale.getTimeZones?.() ?? locale.timeZones ?? [];
}

let placeIndex: Map<string, Place> | null = null;

export function places(languages: readonly string[] = ["en"]): Map<string, Place> {
  if (placeIndex) return placeIndex;
  const index = new Map<string, Place>();
  for (const zone of Intl.supportedValuesOf("timeZone")) {
    const city = zone.split("/").pop()!.replace(/_/g, " ");
    index.set(city.toLowerCase(), { name: city, timeZone: zone });
  }
  const displays = languages.map((language) => new Intl.DisplayNames([language], { type: "region", fallback: "none" }));
  const english = new Intl.DisplayNames(["en"], { type: "region", fallback: "none" });
  for (let a = 65; a <= 90; a++) {
    for (let b = 65; b <= 90; b++) {
      const code = String.fromCharCode(a, b);
      const name = english.of(code);
      if (!name) continue;
      const zones = regionZones(code);
      if (zones.length === 0) continue;
      for (const display of [english, ...displays]) {
        const local = display.of(code);
        if (local) index.set(local.toLowerCase(), { name: local, timeZone: zones[0] });
      }
    }
  }
  index.set("utc", { name: "UTC", timeZone: "UTC" });
  index.set("gmt", { name: "GMT", timeZone: "UTC" });
  placeIndex = index;
  return index;
}

export function parseTimeQuery(query: string, index: Map<string, Place>): Place | null {
  const text = query.trim().toLowerCase();
  const match =
    text.match(/^(?:time|now|date)\s+in\s+(.+)$/) ??
    text.match(/^(.+?)\s+time$/) ??
    text.match(/^(?:hora|horas|agora|data|que\s+horas)\s+(?:em|no|na|de)\s+(.+)$/);
  return match ? (index.get(unaccent(match[1].trim())) ?? index.get(match[1].trim()) ?? null) : null;
}

export function formatTimeIn(date: Date, timeZone: string, locale: readonly string[] = navigator.languages): { text: string; offset: string } {
  const primary = (locale[0] ?? "en-US").toLowerCase();
  const text = new Intl.DateTimeFormat(locale.length ? locale : ["en-US"], { dateStyle: primary.startsWith("pt") ? "medium" : "long", timeStyle: "short", timeZone }).format(date);
  const offset =
    new Intl.DateTimeFormat(locale.length ? locale : ["en-US"], { timeZone, timeZoneName: "shortOffset" }).formatToParts(date).find((p) => p.type === "timeZoneName")?.value ?? timeZone;
  return { text, offset };
}

export interface Rgba {
  r: number;
  g: number;
  b: number;
  a: number;
}

export interface ParsedColor {
  rgba: Rgba;
  format: string;
}

function hslToRgb(h: number, s: number, l: number): [number, number, number] {
  const k = (n: number) => (n + h / 30) % 12;
  const a = s * Math.min(l, 1 - l);
  const f = (n: number) => l - a * Math.max(-1, Math.min(k(n) - 3, Math.min(9 - k(n), 1)));
  return [f(0) * 255, f(8) * 255, f(4) * 255];
}

function channel(text: string, max: number): number {
  return text.endsWith("%") ? (Number(text.slice(0, -1)) / 100) * max : Number(text);
}

export function parseColor(input: string): ParsedColor | null {
  const text = input.trim().toLowerCase();
  const hex = text.match(/^#([0-9a-f]{3,4}|[0-9a-f]{6}|[0-9a-f]{8})$/);
  if (hex) {
    const digits = hex[1].length <= 4 ? [...hex[1]].map((c) => c + c).join("") : hex[1];
    const values = digits.match(/../g)!.map((pair) => parseInt(pair, 16));
    return { rgba: { r: values[0], g: values[1], b: values[2], a: values.length === 4 ? values[3] / 255 : 1 }, format: "HEX" };
  }
  const fn = text.match(/^(rgba?|hsla?|lab|lch|oklab|oklch)\(\s*([^)]*)\)$/);
  if (!fn) return null;
  const [body, alphaPart] = fn[2].split("/").map((s) => s.trim());
  const parts = body.includes(",") ? body.split(",").map((s) => s.trim()) : body.split(/\s+/);
  const alphaText = alphaPart ?? parts[3];
  const number = (value: string, scale = 1): number => (value.endsWith("%") ? (Number(value.slice(0, -1)) / 100) * scale : Number(value));
  if (fn[1] === "lab" || fn[1] === "lch") {
    if (parts.length < 3) return null;
    const L = number(parts[0], 100);
    const second = number(parts[1], fn[1] === "lab" ? 125 : 150);
    const third = number(parts[2], fn[1] === "lab" ? 125 : 360);
    const lab: [number, number, number] = fn[1] === "lab" ? [L, second, third] : [L, second * Math.cos((third * Math.PI) / 180), second * Math.sin((third * Math.PI) / 180)];
    const rgba = rgbFromLab(lab);
    return valid(rgba) ? { rgba, format: fn[1].toUpperCase() } : null;
  }
  if (fn[1] === "oklab" || fn[1] === "oklch") {
    if (parts.length < 3) return null;
    const L = number(parts[0], 1);
    const second = number(parts[1], 0.4);
    const third = number(parts[2], fn[1] === "oklab" ? 0.4 : 360);
    const oklab: [number, number, number] =
      fn[1] === "oklab" ? [L, second, third] : [L, second * Math.cos((third * Math.PI) / 180), second * Math.sin((third * Math.PI) / 180)];
    const rgba = rgbFromOklab(oklab);
    return valid(rgba) ? { rgba, format: fn[1].toUpperCase() } : null;
  }
  if (parts.length < 3 || parts.slice(0, 3).some((p) => !/^-?[\d.]+(%|deg)?$/.test(p))) return null;
  const a = alphaText === undefined ? 1 : channel(alphaText, 1);
  if (fn[1].startsWith("rgb")) {
    const [r, g, b] = parts.slice(0, 3).map((p) => channel(p, 255));
    return valid({ r, g, b, a }) ? { rgba: { r, g, b, a }, format: "RGB" } : null;
  }
  const h = Number(parts[0].replace("deg", ""));
  const [s, l] = parts.slice(1, 3).map((p) => Number(p.replace("%", "")) / 100);
  const [r, g, b] = hslToRgb(((h % 360) + 360) % 360, s, l);
  return valid({ r, g, b, a }) && s <= 1 && l <= 1 ? { rgba: { r, g, b, a }, format: "HSL" } : null;
}

function valid({ r, g, b, a }: Rgba): boolean {
  return [r, g, b].every((v) => Number.isFinite(v) && v >= 0 && v <= 255) && Number.isFinite(a) && a >= 0 && a <= 1;
}

const significant = new Intl.NumberFormat("en-US", { maximumSignificantDigits: 3, useGrouping: false });
const fmt = (n: number) => significant.format(Math.abs(n) < 1e-9 ? 0 : n);
const hex2 = (n: number) => Math.round(n).toString(16).padStart(2, "0").toUpperCase();

function rgbToHsl({ r, g, b }: Rgba): [number, number, number] {
  const [rn, gn, bn] = [r / 255, g / 255, b / 255];
  const max = Math.max(rn, gn, bn);
  const min = Math.min(rn, gn, bn);
  const l = (max + min) / 2;
  const d = max - min;
  if (d === 0) return [0, 0, l * 100];
  const s = d / (1 - Math.abs(2 * l - 1));
  const h = max === rn ? ((gn - bn) / d) % 6 : max === gn ? (bn - rn) / d + 2 : (rn - gn) / d + 4;
  return [(h * 60 + 360) % 360, s * 100, l * 100];
}

export function colorFormats(color: Rgba): { id: string; label: string; value: string }[] {
  const { r, g, b, a } = color;
  const [h, s, l] = rgbToHsl(color);
  const [lab, lch] = labFromRgb(color);
  const [oklab, oklch] = oklabFromRgb(color);
  const white = Math.min(r, g, b) / 2.55;
  const black = 100 - Math.max(r, g, b) / 2.55;
  const alphaSuffix = a < 1 ? ` / ${fmt(a * 100)}%` : "";
  return [
    { id: "hex", label: "Hex", value: `#${hex2(r)}${hex2(g)}${hex2(b)}` },
    { id: "hexAlpha", label: "Hex with Alpha", value: `#${hex2(r)}${hex2(g)}${hex2(b)}${hex2(a * 255)}` },
    { id: "rgba", label: "RGBA", value: `rgba(${fmt(r)}, ${fmt(g)}, ${fmt(b)}, ${fmt(a)})` },
    { id: "rgbaPercentage", label: "RGBA (Percentage)", value: `rgba(${fmt(r / 2.55)}%, ${fmt(g / 2.55)}%, ${fmt(b / 2.55)}%, ${fmt(a)})` },
    { id: "rgbCss4", label: "RGB (CSS4)", value: a < 1 ? `rgb(${fmt(r)}, ${fmt(g)}, ${fmt(b)}${alphaSuffix})` : `rgb(${fmt(r)}, ${fmt(g)}, ${fmt(b)})` },
    { id: "hsl", label: "HSL", value: `hsl(${fmt(h)}deg, ${fmt(s)}%, ${fmt(l)}%)` },
    { id: "hsla", label: "HSL with Alpha", value: `hsl(${fmt(h)}deg, ${fmt(s)}%, ${fmt(l)}%, ${fmt(a)})` },
    { id: "hslCss4", label: "HSL (CSS4)", value: `hsl(${fmt(h)}deg ${fmt(s)}% ${fmt(l)}%${alphaSuffix})` },
    { id: "hwb", label: "HWB", value: `hwb(${fmt(h)}deg ${fmt(white)}% ${fmt(black)}%${alphaSuffix})` },
    { id: "lab", label: "LAB", value: `lab(${fmt(lab[0])}% ${fmt(lab[1])} ${fmt(lab[2])})` },
    { id: "lch", label: "LCH", value: `lch(${fmt(lch[0])}% ${fmt(lch[1])} ${fmt(lch[2])})` },
    { id: "oklab", label: "OKLAB", value: `oklab(${fmt(oklab[0])} ${fmt(oklab[1])} ${fmt(oklab[2])})` },
    { id: "oklch", label: "OKLCH", value: `oklch(${fmt(oklch[0])} ${fmt(oklch[1])} ${fmt(oklch[2])})` },
  ];
}

const srgbToLinear = (channel: number): number => {
  const v = channel / 255;
  return v <= 0.04045 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4;
};

const linearToSrgb = (v: number): number => {
  const c = Math.min(1, Math.max(0, v));
  return Math.round(255 * (c <= 0.0031308 ? c * 12.92 : 1.055 * c ** (1 / 2.4) - 0.055));
};

function labFromRgb({ r, g, b }: Rgba): [[number, number, number], [number, number, number]] {
  const [lr, lg, lb] = [r, g, b].map(srgbToLinear);
  const x = 0.4123907993 * lr + 0.3575843394 * lg + 0.1804807884 * lb;
  const y = 0.2126390059 * lr + 0.7151686788 * lg + 0.0721923154 * lb;
  const z = 0.0193308187 * lr + 0.1191947798 * lg + 0.9505321522 * lb;
  const f = (t: number) => (t > 216 / 24389 ? Math.cbrt(t) : (24389 / 27 * t + 16) / 116);
  const [fx, fy, fz] = [f(x / 0.95047), f(y), f(z / 1.08883)];
  const L = 116 * fy - 16;
  const a = 500 * (fx - fy);
  const bStar = 200 * (fy - fz);
  const C = Math.hypot(a, bStar);
  const H = ((Math.atan2(bStar, a) * 180) / Math.PI + 360) % 360;
  return [[L, a, bStar], [L, C, H]];
}

function oklabFromRgb({ r, g, b }: Rgba): [[number, number, number], [number, number, number]] {
  const [lr, lg, lb] = [r, g, b].map(srgbToLinear);
  const l = Math.cbrt(0.4122214708 * lr + 0.5363325363 * lg + 0.0514459929 * lb);
  const m = Math.cbrt(0.2119034982 * lr + 0.6806995451 * lg + 0.1073969566 * lb);
  const s = Math.cbrt(0.0883024619 * lr + 0.2817188376 * lg + 0.6299787005 * lb);
  const L = 0.2104542553 * l + 0.793617785 * m - 0.0040720468 * s;
  const A = 1.9779984951 * l - 2.428592205 * m + 0.4505937099 * s;
  const B = 0.0259040371 * l + 0.7827717662 * m - 0.808675766 * s;
  const C = Math.hypot(A, B);
  const H = ((Math.atan2(B, A) * 180) / Math.PI + 360) % 360;
  return [[L, A, B], [L, C, H]];
}

function rgbFromLab(lab: [number, number, number]): Rgba {
  const [L, a, bStar] = lab;
  const fy = (L + 16) / 116;
  const fx = fy + a / 500;
  const fz = fy - bStar / 200;
  const finv = (t: number) => (t > 6 / 29 ? t ** 3 : 3 * (6 / 29) ** 2 * (t - 4 / 29));
  const x = 0.95047 * finv(fx);
  const y = finv(fy);
  const z = 1.08883 * finv(fz);
  const lr = 3.2404542 * x - 1.5371385 * y - 0.4985314 * z;
  const lg = -0.969266 * x + 1.8760108 * y + 0.041556 * z;
  const lb = 0.0556434 * x - 0.2040259 * y + 1.0572252 * z;
  return { r: linearToSrgb(lr), g: linearToSrgb(lg), b: linearToSrgb(lb), a: 1 };
}

function rgbFromOklab([L, A, B]: [number, number, number]): Rgba {
  const l = (L + 0.3963377774 * A + 0.2158037573 * B) ** 3;
  const m = (L - 0.1055613458 * A - 0.0638541728 * B) ** 3;
  const s = (L - 0.0894841775 * A - 1.291485548 * B) ** 3;
  const lr = 4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s;
  const lg = -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s;
  const lb = -0.0041960863 * l - 0.7034186147 * m + 1.707614701 * s;
  return { r: linearToSrgb(lr), g: linearToSrgb(lg), b: linearToSrgb(lb), a: 1 };
}

const NAMED_COLORS =
  "Alice Blue:f0f8ff,Antique White:faebd7,Aqua:00ffff,Aquamarine:7fffd4,Azure:f0ffff,Beige:f5f5dc,Bisque:ffe4c4,Black:000000,Blanched Almond:ffebcd,Blue:0000ff,Blue Violet:8a2be2,Brown:a52a2a,Burlywood:deb887,Cadet Blue:5f9ea0,Chartreuse:7fff00,Chocolate:d2691e,Coral:ff7f50,Cornflower Blue:6495ed,Cornsilk:fff8dc,Crimson:dc143c,Dark Blue:00008b,Dark Cyan:008b8b,Dark Goldenrod:b8860b,Dark Gray:a9a9a9,Dark Green:006400,Dark Khaki:bdb76b,Dark Magenta:8b008b,Dark Olive Green:556b2f,Dark Orange:ff8c00,Dark Orchid:9932cc,Dark Red:8b0000,Dark Salmon:e9967a,Dark Sea Green:8fbc8f,Dark Slate Blue:483d8b,Dark Slate Gray:2f4f4f,Dark Turquoise:00ced1,Dark Violet:9400d3,Deep Pink:ff1493,Deep Sky Blue:00bfff,Dim Gray:696969,Dodger Blue:1e90ff,Firebrick:b22222,Floral White:fffaf0,Forest Green:228b22,Gainsboro:dcdcdc,Ghost White:f8f8ff,Gold:ffd700,Goldenrod:daa520,Gray:808080,Green:008000,Green Yellow:adff2f,Honeydew:f0fff0,Hot Pink:ff69b4,Indian Red:cd5c5c,Indigo:4b0082,Ivory:fffff0,Khaki:f0e68c,Lavender:e6e6fa,Lavender Blush:fff0f5,Lawn Green:7cfc00,Lemon Chiffon:fffacd,Light Blue:add8e6,Light Coral:f08080,Light Cyan:e0ffff,Light Goldenrod Yellow:fafad2,Light Gray:d3d3d3,Light Green:90ee90,Light Pink:ffb6c1,Light Salmon:ffa07a,Light Sea Green:20b2aa,Light Sky Blue:87cefa,Light Slate Gray:778899,Light Steel Blue:b0c4de,Light Yellow:ffffe0,Lime:00ff00,Lime Green:32cd32,Linen:faf0e6,Maroon:800000,Medium Aquamarine:66cdaa,Medium Blue:0000cd,Medium Orchid:ba55d3,Medium Purple:9370db,Medium Sea Green:3cb371,Medium Slate Blue:7b68ee,Medium Spring Green:00fa9a,Medium Turquoise:48d1cc,Medium Violet Red:c71585,Midnight Blue:191970,Mint Cream:f5fffa,Misty Rose:ffe4e1,Moccasin:ffe4b5,Navajo White:ffdead,Navy:000080,Old Lace:fdf5e6,Olive:808000,Olive Drab:6b8e23,Orange:ffa500,Orange Red:ff4500,Orchid:da70d6,Pale Goldenrod:eee8aa,Pale Green:98fb98,Pale Turquoise:afeeee,Pale Violet Red:db7093,Papaya Whip:ffefd5,Peach Puff:ffdab9,Peru:cd853f,Pink:ffc0cb,Plum:dda0dd,Powder Blue:b0e0e6,Purple:800080,Rebecca Purple:663399,Red:ff0000,Rosy Brown:bc8f8f,Royal Blue:4169e1,Saddle Brown:8b4513,Salmon:fa8072,Sandy Brown:f4a460,Sea Green:2e8b57,Sea Shell:fff5ee,Sienna:a0522d,Silver:c0c0c0,Sky Blue:87ceeb,Slate Blue:6a5acd,Slate Gray:708090,Snow:fffafa,Spring Green:00ff7f,Steel Blue:4682b4,Tan:d2b48c,Teal:008080,Thistle:d8bfd8,Tomato:ff6347,Turquoise:40e0d0,Violet:ee82ee,Wheat:f5deb3,White:ffffff,White Smoke:f5f5f5,Yellow:ffff00,Yellow Green:9acd32";

const NAMED = NAMED_COLORS.split(",").map((entry) => {
  const [name, hex] = entry.split(":");
  return { name, rgb: hex.match(/../g)!.map((pair) => parseInt(pair, 16)) };
});

export function colorName({ r, g, b }: Rgba): string {
  let best = NAMED[0];
  let distance = Infinity;
  for (const named of NAMED) {
    const d = (named.rgb[0] - r) ** 2 + (named.rgb[1] - g) ** 2 + (named.rgb[2] - b) ** 2;
    if (d < distance) {
      distance = d;
      best = named;
    }
  }
  return best.name;
}
