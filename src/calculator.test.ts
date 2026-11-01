import { describe, expect, it } from "vitest";
import {
  colorFormats,
  colorName,
  convertCurrency,
  evaluate,
  formatNumber,
  formatTimeIn,
  isExpression,
  numberToWords,
  numberToWordsPt,
  parseColor,
  parseCurrency,
  parseTimeQuery,
  places,
} from "./calculator";

describe("evaluate", () => {
  it("follows operator precedence, parentheses and powers", () => {
    expect(evaluate("2 + 3 * 4")).toBe(14);
    expect(evaluate("(2 + 3) * 4")).toBe(20);
    expect(evaluate("2^3^2")).toBe(512);
    expect(evaluate("-2^2")).toBe(-4);
    expect(evaluate("10 / 4")).toBe(2.5);
    expect(evaluate("6 × 7 ÷ 2")).toBe(21);
  });

  it("supports functions, constants, factorial and implicit multiplication", () => {
    expect(evaluate("sqrt(16) + 5!")).toBe(124);
    expect(evaluate("2pi")).toBeCloseTo(2 * Math.PI);
    expect(evaluate("3(4 + 1)")).toBe(15);
    expect(evaluate("1,000 * 2")).toBe(2000);
  });

  it("treats percentages like a calculator", () => {
    expect(evaluate("50%")).toBe(0.5);
    expect(evaluate("200 + 10%")).toBeCloseTo(220);
    expect(evaluate("200 - 25%")).toBe(150);
    expect(evaluate("20% * 50")).toBe(10);
  });

  it("rejects text that is not math", () => {
    expect(evaluate("hello world")).toBeNull();
    expect(evaluate("2 +")).toBeNull();
    expect(evaluate("1 / 0")).toBeNull();
    expect(isExpression("time in china")).toBe(false);
    expect(isExpression("42")).toBe(false);
    expect(isExpression("2+2")).toBe(true);
    expect(isExpression("pi")).toBe(true);
  });

  it("formats results and spells integers", () => {
    expect(formatNumber(1234567.5)).toBe("1,234,567.5");
    expect(formatNumber(0.1 + 0.2)).toBe("0.3");
    expect(numberToWords(1214)).toBe("One Thousand Two Hundred Fourteen");
    expect(numberToWords(-40)).toBe("Minus Forty");
    expect(numberToWords(2.5)).toBeNull();
  });
});

describe("currency", () => {
  const known = new Set(["USD", "BRL", "EUR"]);

  it("parses amounts with symbols or codes on either side", () => {
    expect(parseCurrency("100$ in brl", known)).toEqual({ amount: 100, from: "USD", to: "BRL" });
    expect(parseCurrency("€50 to usd", known)).toEqual({ amount: 50, from: "EUR", to: "USD" });
    expect(parseCurrency("12,5 eur in R$", known)).toEqual({ amount: 12.5, from: "EUR", to: "BRL" });
    expect(parseCurrency("100 in brl", known)).toBeNull();
    expect(parseCurrency("100 xyz in brl", known)).toBeNull();
  });

  it("converts through the base currency", () => {
    expect(convertCurrency({ amount: 10, from: "EUR", to: "BRL" }, { USD: 1, EUR: 0.5, BRL: 5 })).toBe(100);
  });

  it("parses Portuguese queries with currency words", () => {
    expect(parseCurrency("100 dolares para brl", known)).toEqual({ amount: 100, from: "USD", to: "BRL" });
    expect(parseCurrency("100,50 reais em dolares", known)).toEqual({ amount: 100.5, from: "BRL", to: "USD" });
    expect(parseCurrency("10 dólares para real", known)).toEqual({ amount: 10, from: "USD", to: "BRL" });
    expect(parseCurrency("1 euro pra real", known)).toEqual({ amount: 1, from: "EUR", to: "BRL" });
    expect(parseCurrency("100 xyz para brl", known)).toBeNull();
  });
});

describe("time zones", () => {
  it("finds countries and cities", () => {
    const index = places(["en", "pt"]);
    expect(parseTimeQuery("time in china", index)?.timeZone).toBe("Asia/Shanghai");
    expect(parseTimeQuery("Tokyo time", index)?.timeZone).toBe("Asia/Tokyo");
    expect(parseTimeQuery("time in nowhere", index)).toBeNull();
  });

  it("finds places from Portuguese queries", () => {
    const index = places(["en", "pt"]);
    expect(parseTimeQuery("hora em brasil", index)?.name).toBe("Brasil");
    expect(parseTimeQuery("que horas em china", index)?.timeZone).toBe("Asia/Shanghai");
    expect(parseTimeQuery("hora em lugar nenhum", index)).toBeNull();
  });

  it("formats the date like the reference", () => {
    const { text, offset } = formatTimeIn(new Date("2026-10-18T22:32:00Z"), "Asia/Shanghai");
    expect(text).toBe("October 19, 2026 at 6:32 AM");
    expect(offset).toBe("GMT+8");
  });
});

describe("colors", () => {
  it("parses hex, rgb and hsl", () => {
    expect(parseColor("#fff")).toEqual({ rgba: { r: 255, g: 255, b: 255, a: 1 }, format: "HEX" });
    expect(parseColor("rgb(255 0 0 / 50%)")).toEqual({ rgba: { r: 255, g: 0, b: 0, a: 0.5 }, format: "RGB" });
    expect(parseColor("hsl(120, 100%, 50%)")?.rgba).toEqual({ r: 0, g: 255, b: 0, a: 1 });
    expect(parseColor("#ggg")).toBeNull();
    expect(parseColor("rgb(300, 0, 0)")).toBeNull();
  });

  it("names the nearest CSS color and lists copy formats", () => {
    expect(colorName({ r: 255, g: 255, b: 255, a: 1 })).toBe("White");
    expect(colorName({ r: 100, g: 149, b: 236, a: 1 })).toBe("Cornflower Blue");
    const formats = colorFormats({ r: 255, g: 0, b: 0, a: 1 });
    expect(formats.find((f) => f.id === "hex")?.value).toBe("#FF0000");
    expect(formats.find((f) => f.id === "hsl")?.value).toBe("hsl(0deg, 100%, 50%)");
    expect(formats.find((f) => f.id === "rgbCss4")?.value).toBe("rgb(255, 0, 0)");
  });
});

describe("numberToWordsPt", () => {
  it("spells integers in Portuguese", () => {
    expect(numberToWordsPt(0)).toBe("Zero");
    expect(numberToWordsPt(5)).toBe("cinco");
    expect(numberToWordsPt(21)).toBe("vinte e um");
    expect(numberToWordsPt(100)).toBe("cem");
    expect(numberToWordsPt(150)).toBe("cento e cinquenta");
    expect(numberToWordsPt(1000)).toBe("mil");
    expect(numberToWordsPt(1100)).toBe("mil e cem");
    expect(numberToWordsPt(1123)).toBe("mil cento e vinte e três");
    expect(numberToWordsPt(2100)).toBe("dois mil e cem");
    expect(numberToWordsPt(1_000_000)).toBe("um milhão");
    expect(numberToWordsPt(21_000_000)).toBe("vinte e um milhões");
    expect(numberToWordsPt(-40)).toBe("menos quarenta");
    expect(numberToWordsPt(2.5)).toBeNull();
  });
});
