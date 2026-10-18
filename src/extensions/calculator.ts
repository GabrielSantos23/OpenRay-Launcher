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
  parseTimeQuery,
  places,
  plainNumber,
} from "../calculator";
import { defineExtension, type InlineResult } from "../extension-api";
import { copyAndClose, copyPasteActions, frontmostApp } from "../output";

const RATES_URL = "https://open.er-api.com/v6/latest/USD";
const RATES_MAX_AGE_MS = 6 * 3_600_000;
const RATES_KEY = "openray.rates";

interface Rates {
  rates: Record<string, number>;
  updated: number;
  fetched: number;
}

let cached: Rates | null = null;
let pending: Promise<Rates | null> | null = null;

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

async function rates(): Promise<Rates | null> {
  cached ??= readCache();
  if (cached && Date.now() - cached.fetched < RATES_MAX_AGE_MS) return cached;
  pending ??= fetchRates();
  return pending;
}

export function updatedAgo(updated: number, now = Date.now(), verb = "Updated"): string {
  const minutes = Math.max(0, Math.round((now - updated) / 60_000));
  if (minutes < 1) return `${verb} just now`;
  if (minutes < 60) return `${verb} ${minutes} ${minutes === 1 ? "minute" : "minutes"} ago`;
  const hours = Math.round(minutes / 60);
  if (hours < 24) return `${verb} about ${hours} ${hours === 1 ? "hour" : "hours"} ago`;
  const days = Math.round(hours / 24);
  return `${verb} ${days} ${days === 1 ? "day" : "days"} ago`;
}

function currencyName(code: string): string {
  const name = new Intl.DisplayNames([navigator.language, "en"], { type: "currency", fallback: "code" }).of(code) ?? code;
  return capitalize(name);
}

function formatMoney(amount: number, code: string): string {
  try {
    return new Intl.NumberFormat("en-US", { style: "currency", currency: code, maximumFractionDigits: Math.abs(amount) < 1 ? 6 : 2 }).format(amount);
  } catch {
    return `${formatNumber(amount)} ${code}`;
  }
}

async function answer(display: string, plain: string, input: InlineResult["input"], label: string, note?: string): Promise<InlineResult> {
  return {
    section: "Calculator",
    input,
    output: { text: display, label },
    note,
    primaryActionTitle: "Copy Answer",
    actions: [copyPasteActions(plain, await frontmostApp(), "Copy Answer")],
  };
}

async function inline(query: string): Promise<InlineResult | null> {
  const text = query.trim();
  if (!text) return null;

  const color = parseColor(text);
  if (color) {
    const formats = colorFormats(color.rgba);
    return {
      section: "Color",
      input: { text, label: color.format },
      output: { text: "", label: colorName(color.rgba), swatch: formats.find((f) => f.id === "hexAlpha")!.value },
      primaryActionTitle: "Copy Color as…",
      actions: [
        [
          {
            id: "copy-color-as",
            title: "Copy Color as…",
            icon: Clipboard,
            submenu: () =>
              formats.map((format) => ({
                id: format.id,
                title: format.label,
                accessory: format.value,
                run: () => copyAndClose(format.value, `Copied ${format.label} to Clipboard`),
              })),
            run: () => {},
          },
        ],
      ],
    };
  }

  if (/[a-z$€£¥₹₩₽₺₿]/i.test(text) && /\d/.test(text) && /\s(in|to|as|=)\s/i.test(text)) {
    const table = await rates();
    if (table) {
      const parsed = parseCurrency(text, new Set(Object.keys(table.rates)));
      const value = parsed && convertCurrency(parsed, table.rates);
      if (parsed && value !== null) {
        return answer(
          formatMoney(value, parsed.to),
          value.toFixed(2),
          { text, label: currencyName(parsed.from) },
          currencyName(parsed.to),
          updatedAgo(table.updated),
        );
      }
    }
  }

  const place = parseTimeQuery(text, places(navigator.languages));
  if (place) {
    const { text: when, offset } = formatTimeIn(new Date(), place.timeZone);
    return answer(when, when, { text, label: place.name }, offset);
  }

  if (!isExpression(text)) return null;
  const value = evaluate(text);
  if (value === null || plainNumber(value) === text) return null;
  return answer(formatNumber(value), plainNumber(value), { text, label: "Expression" }, numberToWords(value) ?? "Result");
}

export default defineExtension({
  id: "calculator",
  title: "Calculator",
  icon: { glyph: Calculator, tone: "orange" },
  inline,
});
