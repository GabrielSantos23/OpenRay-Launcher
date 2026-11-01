import { Calculator, Clipboard } from "lucide-react";
import {
  capitalize,
  colorFormats,
  colorName,
  convertCurrency,
  evaluate,
  formatNumber,
  formatTimeIn,
  isExpression,
  numberToWords,
  parseColor,
  parseCurrency,
  numberToWordsPt,
  parseTimeQuery,
  places,
  plainNumber,
  unaccent,
} from "../calculator";
import type { Place } from "../calculator";
import type { CalculationKind } from "../calculatorHistory";
import { defineExtension, type Action, type InlineResult } from "../extension-api";
import { copyAndClose, copyPasteActions, frontmostApp } from "../output";
import { CalculatorHistory, saveCalculation } from "./calculator-history";
import { language, t } from "../i18n";
import { formatDuration, formatUnitValue, parseUnitQuery, unitCategoryLabel } from "../units";

const RATES_URL = "https://open.er-api.com/v6/latest/USD";
const RATES_MAX_AGE_MS = 6 * 3_600_000;
const RATES_KEY = "openray.rates";
const CRYPTO_URL = "https://api.coingecko.com/api/v3/simple/price?ids=bitcoin,ethereum,solana,ripple,cardano,dogecoin,litecoin,tether&vs_currencies=usd";
const CRYPTO_MAX_AGE_MS = 10 * 60_000;
const CRYPTO_KEY = "openray.crypto-rates";
const CRYPTO_CODES: Record<string, string> = { bitcoin: "BTC", ethereum: "ETH", solana: "SOL", ripple: "XRP", cardano: "ADA", dogecoin: "DOGE", litecoin: "LTC", tether: "USDT" };

interface Rates {
  rates: Record<string, number>;
  updated: number;
  fetched: number;
}

let cached: Rates | null = null;
let pending: Promise<Rates | null> | null = null;
let crypto: Rates | null = null;
let cryptoPending: Promise<Rates | null> | null = null;

function readCache(): Rates | null {
  try {
    return JSON.parse(localStorage.getItem(RATES_KEY) ?? "null");
  } catch {
    return null;
  }
}

async function fetchRates(): Promise<Rates | null> {
  try {
    const response = await fetch(RATES_URL);
    const body = await response.json();
    if (body.result !== "success") throw new Error(body["error-type"] ?? "rates unavailable");
    cached = { rates: body.rates, updated: body.time_last_update_unix * 1000, fetched: Date.now() };
    try {
      localStorage.setItem(RATES_KEY, JSON.stringify(cached));
    } catch {
      return cached;
    }
    return cached;
  } catch (error) {
    console.error("exchange rates unavailable", error);
    return cached;
  } finally {
    pending = null;
  }
}

export function cryptoRatesFromPrices(prices: Record<string, { usd?: number }>): Record<string, number> {
  return Object.fromEntries(
    Object.entries(CRYPTO_CODES)
      .map(([id, code]) => [code, prices[id]?.usd ? 1 / prices[id].usd! : null])
      .filter((entry): entry is [string, number] => entry[1] !== null),
  );
}

function readCryptoCache(): Rates | null {
  try {
    return JSON.parse(localStorage.getItem(CRYPTO_KEY) ?? "null");
  } catch {
    return null;
  }
}

async function fetchCryptoRates(): Promise<Rates | null> {
  try {
    const response = await fetch(CRYPTO_URL);
    const prices = await response.json();
    const rates = cryptoRatesFromPrices(prices);
    if (Object.keys(rates).length === 0) throw new Error("crypto rates unavailable");
    crypto = { rates, updated: Date.now(), fetched: Date.now() };
    try {
      localStorage.setItem(CRYPTO_KEY, JSON.stringify(crypto));
    } catch {
      return crypto;
    }
    return crypto;
  } catch (error) {
    console.error("crypto rates unavailable", error);
    return crypto;
  }
}

async function rates(): Promise<Rates | null> {
  cached ??= readCache();
  if (!cached || Date.now() - cached.fetched >= RATES_MAX_AGE_MS) {
    pending ??= fetchRates().finally(() => (pending = null));
    const fresh = await pending;
    if (fresh) cached = fresh;
  }
  crypto ??= readCryptoCache();
  if (!crypto || Date.now() - crypto.fetched >= CRYPTO_MAX_AGE_MS) {
    cryptoPending ??= fetchCryptoRates().finally(() => (cryptoPending = null));
    const fresh = await cryptoPending;
    if (fresh) crypto = fresh;
  }
  if (!cached) return null;
  return { ...cached, rates: { ...cached.rates, ...(crypto?.rates ?? {}) } };
}

export function updatedAgo(updated: number, now = Date.now(), verb = "Updated"): string {
  const minutes = Math.max(0, Math.round((now - updated) / 60_000));
  if (minutes < 1) return t("{verb} just now", { verb: t(verb) });
  if (minutes < 60) return t(minutes === 1 ? "{verb} {n} minute ago" : "{verb} {n} minutes ago", { verb: t(verb), n: minutes });
  const hours = Math.round(minutes / 60);
  if (hours < 24) return t(hours === 1 ? "{verb} about {n} hour ago" : "{verb} about {n} hours ago", { verb: t(verb), n: hours });
  const days = Math.round(hours / 24);
  return t(days === 1 ? "{verb} {n} day ago" : "{verb} {n} days ago", { verb: t(verb), n: days });
}

function currencyName(code: string): string {
  const name = new Intl.DisplayNames([navigator.language, "en"], { type: "currency", fallback: "code" }).of(code) ?? code;
  return capitalize(name);
}

function formatMoney(amount: number, code: string): string {
  try {
    return new Intl.NumberFormat(language() === "pt" ? "pt-BR" : "en-US", { style: "currency", currency: code, maximumFractionDigits: Math.abs(amount) < 1 ? 6 : 2 }).format(amount);
  } catch {
    return `${formatNumber(amount)} ${code}`;
  }
}


export const PLACE_ALIASES: Record<string, string> = { ldn: "london", nyc: "new york", sf: "los angeles", la: "los angeles", sp: "sao paulo", hk: "hong kong" };

export function lookupPlace(name: string, index: Map<string, Place>): Place | null {
  const key = unaccent(name.trim().toLowerCase());
  return index.get(PLACE_ALIASES[key] ?? key) ?? null;
}

export interface Clock {
  hour: number;
  minute: number;
}

export function parseClock(text: string): Clock | null {
  const match = text.toLowerCase().match(/^(\d{1,2})(?::(\d{2}))?\s*(am|pm|h)$/);
  if (!match) return null;
  let hour = Number(match[1]);
  const minute = Number(match[2] ?? 0);
  if (match[3] === "pm" && hour < 12) hour += 12;
  if (match[3] === "am" && hour === 12) hour = 0;
  if (hour > 23 || minute > 59) return null;
  return { hour, minute };
}

export interface ClockQuery {
  time: Clock;
  from: string;
  to: string;
}

export function parseClockQuery(text: string): ClockQuery | null {
  const parts = text.trim().split(/\s+(?:in|em|para)\s+/i);
  if (parts.length !== 2) return null;
  const left = parts[0].trim().match(/^(\S+)\s+(.+)$/);
  if (!left) return null;
  const time = parseClock(left[1]);
  if (!time) return null;
  return { time, from: left[2], to: parts[1] };
}

function offsetMinutes(date: Date, timeZone: string): number {
  const name = new Intl.DateTimeFormat("en-US", { timeZone, timeZoneName: "longOffset" }).formatToParts(date).find((p) => p.type === "timeZoneName")?.value;
  const match = name?.match(/GMT([+-])(\d{2}):(\d{2})/);
  if (!match) return 0;
  const sign = match[1] === "-" ? -1 : 1;
  return sign * (Number(match[2]) * 60 + Number(match[3]));
}

function zonedYmd(date: Date, timeZone: string): string {
  const parts = new Intl.DateTimeFormat("en-CA", { timeZone, year: "numeric", month: "2-digit", day: "2-digit" }).formatToParts(date);
  const get = (type: string) => parts.find((p) => p.type === type)?.value ?? "01";
  return `${get("year")}-${get("month")}-${get("day")}`;
}

export function instantFor(ymd: string, time: Clock, timeZone: string): number {
  const guess = Date.parse(`${ymd}T${String(time.hour).padStart(2, "0")}:${String(time.minute).padStart(2, "0")}:00Z`);
  return guess - offsetMinutes(new Date(guess), timeZone) * 60_000;
}

const RELATIVE_UNITS: Record<string, number> = {
  second: 1000, seconds: 1000, segundo: 1000, segundos: 1000,
  minute: 60_000, minutes: 60_000, minuto: 60_000, minutos: 60_000,
  hour: 3_600_000, hours: 3_600_000, hora: 3_600_000, horas: 3_600_000,
  day: 86_400_000, days: 86_400_000, dia: 86_400_000, dias: 86_400_000,
  week: 604_800_000, weeks: 604_800_000, semana: 604_800_000, semanas: 604_800_000,
  month: 2_629_746_000, months: 2_629_746_000, mes: 2_629_746_000, meses: 2_629_746_000,
  year: 31_556_952_000, years: 31_556_952_000, ano: 31_556_952_000, anos: 31_556_952_000,
};

export function parseRelativeDate(text: string): { delta: number; future: boolean } | null {
  const normalized = unaccent(text.trim().toLowerCase());
  let match = normalized.match(/^(?:in|em)\s+(\d+(?:[.,]\d+)?)\s+([a-z]+)$/);
  if (match) {
    const ms = RELATIVE_UNITS[match[2]];
    return ms ? { delta: Number(match[1].replace(",", ".")) * ms, future: true } : null;
  }
  match = normalized.match(/^(\d+(?:[.,]\d+)?)\s+([a-z]+)\s+ago$/) ?? normalized.match(/^ha\s+(\d+(?:[.,]\d+)?)\s+([a-z]+)$/);
  if (match) {
    const ms = RELATIVE_UNITS[match[2]];
    return ms ? { delta: Number(match[1].replace(",", ".")) * ms, future: false } : null;
  }
  return null;
}

export function parseTargetDate(text: string): Date | null {
  const trimmed = text.trim();
  const iso = trimmed.match(/^(\d{4})-(\d{1,2})-(\d{1,2})$/);
  if (iso) return new Date(Number(iso[1]), Number(iso[2]) - 1, Number(iso[3]));
  const dmy = trimmed.match(/^(\d{1,2})[/.-](\d{1,2})(?:[/.-](\d{4}))?$/);
  if (dmy) return new Date(Number(dmy[3] ?? new Date().getFullYear()), Number(dmy[2]) - 1, Number(dmy[1]));
  const parsed = Date.parse(trimmed);
  return Number.isNaN(parsed) ? null : new Date(parsed);
}

export function daysUntil(target: Date, now = new Date()): number {
  const start = (date: Date) => {
    const copy = new Date(date);
    copy.setHours(0, 0, 0, 0);
    return copy;
  };
  return Math.round((start(target).getTime() - start(now).getTime()) / 86_400_000);
}

export function parseTimeDiff(text: string): { from: string; to: string } | null {
  const normalized = unaccent(text.trim().toLowerCase());
  const match = normalized.match(/^(?:time (?:difference|diff)|diferenca de horario)\s+(?:between|entre)?\s*(.+?)\s+(?:and|e)\s+(.+)$/);
  return match ? { from: match[1], to: match[2] } : null;
}

export function timeDiffMinutes(from: Place, to: Place, now = new Date()): number {
  return offsetMinutes(now, to.timeZone) - offsetMinutes(now, from.timeZone);
}

export function formatDiff(minutes: number): string {
  const sign = minutes < 0 ? "-" : "+";
  const hours = Math.floor(Math.abs(minutes) / 60);
  const rest = Math.abs(minutes) % 60;
  return `${sign}${hours} h${rest ? ` ${rest} min` : ""}`;
}

export interface CalculatorAnswer extends InlineResult {
  kind: CalculationKind;
  plain: string;
}

const remembered = (action: Action, save: () => Promise<void>): Action => ({
  ...action,
  run: async () => {
    await action.run();
    await save();
  },
});

async function answer(kind: CalculationKind, display: string, plain: string, input: InlineResult["input"], label: string, note?: string): Promise<CalculatorAnswer> {
  const save = () => saveCalculation(input.text, display, kind);
  return {
    kind,
    plain,
    section: "Calculator",
    input,
    output: { text: display, label },
    note,
    primaryActionTitle: t("Copy Answer"),
    actions: [
      [
        ...copyPasteActions(plain, await frontmostApp(), t("Copy Answer")).map((action) => remembered(action, save)),
        remembered({ id: "copy-question-and-answer", title: t("Copy Question and Answer"), icon: Clipboard, shortcut: { key: "c", alt: true, shift: true }, run: () => copyAndClose(`${input.text} = ${display}`) }, save),
      ],
    ],
  };
}

export async function calculate(query: string): Promise<CalculatorAnswer | null> {
  const result = await inline(query);
  return result && "kind" in result ? (result as CalculatorAnswer) : null;
}

async function inline(query: string): Promise<InlineResult | CalculatorAnswer | null> {
  const text = query.trim();
  if (!text) return null;

  const color = parseColor(text);
  if (color) {
    const formats = colorFormats(color.rgba);
    return {
      section: t("Color"),
      input: { text, label: color.format },
      output: { text: "", label: colorName(color.rgba), swatch: formats.find((f) => f.id === "hexAlpha")!.value },
      primaryActionTitle: t("Copy Color as…"),
      actions: [
        [
          {
            id: "copy-color-as",
            title: t("Copy Color as…"),
            icon: Clipboard,
            submenu: () =>
              formats.map((format) => ({
                id: format.id,
                title: format.label,
                accessory: format.value,
                run: () => copyAndClose(format.value, t("Copied {value} to Clipboard", { value: format.label })),
              })),
            run: () => {},
          },
        ],
      ],
    };
  }

  const unit = parseUnitQuery(text);
  if (unit) {
    const category = t(unitCategoryLabel(unit.from.category));
    const note = unit.from.category === "time" ? formatDuration(unit.value * unit.to.factor) : undefined;
    return answer(
      "unitConversion",
      `${formatUnitValue(unit.value)} ${unit.to.id}`,
      String(Number(unit.value.toPrecision(12))),
      { text, label: category },
      category,
      note,
    );
  }

  if (/[a-z$€£¥₹₩₽₺₿]/i.test(text) && /\d/.test(text) && /\s(?:in|to|as|para|em|como|pra|pro|=)\s/i.test(text)) {
    const table = await rates();
    if (table) {
      const parsed = parseCurrency(text, new Set(Object.keys(table.rates)));
      const value = parsed && convertCurrency(parsed, table.rates);
      if (parsed && value !== null) {
        return answer(
          "unitConversion",
          formatMoney(value, parsed.to),
          value.toFixed(2),
          { text, label: currencyName(parsed.from) },
          currencyName(parsed.to),
          updatedAgo(table.updated),
        );
      }
    }
  }

  const index = places(navigator.languages);
  const clock = parseClockQuery(text);
  const clockFrom = clock && lookupPlace(clock.from, index);
  const clockTo = clock && lookupPlace(clock.to, index);
  if (clock && clockFrom && clockTo) {
    const instant = instantFor(zonedYmd(new Date(), clockFrom.timeZone), clock.time, clockFrom.timeZone);
    const date = new Date(instant);
    const when = new Intl.DateTimeFormat(navigator.languages, { hour: "numeric", minute: "2-digit", timeZone: clockTo.timeZone }).format(date);
    const day = new Intl.DateTimeFormat(navigator.languages, { dateStyle: "medium", timeZone: clockTo.timeZone }).format(date);
    return answer("dateTime", when, when, { text, label: clockFrom.name }, clockTo.name, day);
  }

  const relative = parseRelativeDate(text);
  if (relative) {
    const date = new Date(Date.now() + (relative.future ? relative.delta : -relative.delta));
    const when = new Intl.DateTimeFormat(navigator.languages, { dateStyle: "full", timeStyle: "short" }).format(date);
    return answer("dateTime", when, when, { text, label: t("Date") }, t("Date"));
  }

  const untilMatch = text.trim().match(/^(?:days? (?:until|to)|dias ate)\s+(.+)$/i) ?? text.trim().match(/^dias até\s+(.+)$/i);
  const target = untilMatch && parseTargetDate(untilMatch[1]);
  if (target) {
    const days = daysUntil(target);
    return answer(
      "dateTime",
      t(days === 1 || days === -1 ? "{n} day" : "{n} days", { n: Math.abs(days) }),
      String(days),
      { text, label: t("Date") },
      t("Date"),
      days < 0 ? t("Past") : undefined,
    );
  }

  const diff = parseTimeDiff(text);
  const diffFrom = diff && lookupPlace(diff.from, index);
  const diffTo = diff && lookupPlace(diff.to, index);
  if (diff && diffFrom && diffTo) {
    const minutes = timeDiffMinutes(diffFrom, diffTo);
    return answer("dateTime", formatDiff(minutes), formatDiff(minutes), { text, label: t("Time difference") }, `${diffFrom.name} → ${diffTo.name}`);
  }

  const place = parseTimeQuery(text, index);
  if (place) {
    const { text: when, offset } = formatTimeIn(new Date(), place.timeZone);
    return answer("dateTime", when, when, { text, label: place.name }, offset);
  }

  if (!isExpression(text)) return null;
  const value = evaluate(text);
  if (value === null || plainNumber(value) === text) return null;
  return answer("math", formatNumber(value), plainNumber(value), { text, label: t("Expression") }, (language() === "pt" ? numberToWordsPt(value) : numberToWords(value)) ?? t("Result"));
}

export default defineExtension({
  id: "calculator",
  title: "Calculator",
  icon: { glyph: Calculator, tone: "orange" },
  inline,
  commands: [
    {
      id: "history",
      title: "Calculator History",
      subtitle: "Calculator",
      keywords: ["calculator", "math", "history", "calculations", "conversions", "calculadora", "matemática", "conversões", "histórico"],
      kind: "Command",
      primaryActionTitle: "Open Command",
      view: CalculatorHistory,
    },
  ],
});
