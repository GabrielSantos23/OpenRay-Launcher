import { describe, expect, it } from "vitest";
import { colorFormats, parseColor, places } from "../calculator";
import { calculate } from "./calculator";
import {
  cryptoRatesFromPrices,
  daysUntil,
  formatDiff,
  instantFor,
  lookupPlace,
  parseClock,
  parseClockQuery,
  parseRelativeDate,
  parseTargetDate,
  parseTimeDiff,
  timeDiffMinutes,
} from "./calculator";

describe("crypto rates", () => {
  it("converts CoinGecko USD prices into rates per USD", () => {
    const rates = cryptoRatesFromPrices({ bitcoin: { usd: 50000 }, ethereum: { usd: 2000 }, dogecoin: { usd: 0 } });
    expect(rates["BTC"]).toBeCloseTo(0.00002, 8);
    expect(rates["ETH"]).toBeCloseTo(0.0005, 8);
    expect(rates["DOGE"]).toBeUndefined();
  });
});

describe("clock queries", () => {
  it("parses 12h, 24h and h-suffix times", () => {
    expect(parseClock("5pm")).toEqual({ hour: 17, minute: 0 });
    expect(parseClock("9:30am")).toEqual({ hour: 9, minute: 30 });
    expect(parseClock("17h")).toEqual({ hour: 17, minute: 0 });
    expect(parseClock("10")).toBeNull();
    expect(parseClock("25h")).toBeNull();
  });

  it("splits clock queries into two places", () => {
    expect(parseClockQuery("5pm ldn in sf")).toEqual({ time: { hour: 17, minute: 0 }, from: "ldn", to: "sf" });
    expect(parseClockQuery("17h sp em ny")).toEqual({ time: { hour: 17, minute: 0 }, from: "sp", to: "ny" });
    expect(parseClockQuery("10 km in miles")).toBeNull();
  });

  it("resolves aliases through the place index", () => {
    const index = places(["en"]);
    expect(lookupPlace("ldn", index)?.name).toBe("London");
    expect(lookupPlace("nyc", index)?.timeZone).toBe("America/New_York");
    expect(lookupPlace("sp", index)?.timeZone).toBe("America/Sao_Paulo");
  });

  it("converts a wall time between zones", () => {
    const instant = instantFor("2026-06-01", { hour: 12, minute: 0 }, "Europe/London");
    const text = new Intl.DateTimeFormat("en-US", { hour: "numeric", minute: "2-digit", timeZone: "America/Los_Angeles", timeZoneName: "shortOffset" }).format(new Date(instant));
    expect(text).toContain("4");
    expect(text).toContain("AM");
  });
});

describe("relative dates and countdowns", () => {
  it("parses relative date queries in both languages", () => {
    expect(parseRelativeDate("in 3 weeks")).toEqual({ delta: 3 * 604_800_000, future: true });
    expect(parseRelativeDate("em 2 dias")).toEqual({ delta: 2 * 86_400_000, future: true });
    expect(parseRelativeDate("2 hours ago")).toEqual({ delta: 2 * 3_600_000, future: false });
    expect(parseRelativeDate("há 5 minutos")).toEqual({ delta: 5 * 60_000, future: false });
    expect(parseRelativeDate("in 3 bananas")).toBeNull();
  });

  it("parses target dates", () => {
    expect(parseTargetDate("2026-12-25")).toEqual(new Date(2026, 11, 25));
    expect(parseTargetDate("25/12/2026")).toEqual(new Date(2026, 11, 25));
    expect(parseTargetDate("banana")).toBeNull();
  });

  it("counts days between midnights", () => {
    expect(daysUntil(new Date(2026, 0, 2), new Date(2026, 0, 1))).toBe(1);
    expect(daysUntil(new Date(2026, 0, 1), new Date(2026, 0, 2))).toBe(-1);
    expect(daysUntil(new Date(2026, 0, 1, 23, 59), new Date(2026, 0, 1, 0, 1))).toBe(0);
  });
});

describe("time differences", () => {
  it("parses both phrasings", () => {
    expect(parseTimeDiff("time difference between new york and tokyo")).toEqual({ from: "new york", to: "tokyo" });
    expect(parseTimeDiff("diferença de horário entre brasil e japão")).toEqual({ from: "brasil", to: "japao" });
    expect(parseTimeDiff("hello world")).toBeNull();
  });

  it("sums zone offsets and formats the result", () => {
    const index = places(["en"]);
    const from = lookupPlace("new york", index)!;
    const to = lookupPlace("tokyo", index)!;
    expect([780, 840]).toContain(timeDiffMinutes(from, to));
    expect(formatDiff(13 * 60 + 30)).toBe("+13 h 30 min");
    expect(formatDiff(-3 * 60)).toBe("-3 h");
  });
});

describe("advanced color formats", () => {
  it("emits LAB, LCH, OKLAB and OKLCH", () => {
    const formats = colorFormats({ r: 255, g: 99, b: 99, a: 1 });
    expect(formats.find((f) => f.id === "lab")?.value).toMatch(/^lab\(/);
    expect(formats.find((f) => f.id === "lch")?.value).toMatch(/^lch\(/);
    expect(formats.find((f) => f.id === "oklab")?.value).toMatch(/^oklab\(/);
    expect(formats.find((f) => f.id === "oklch")?.value).toMatch(/^oklch\(/);
  });

  it("round-trips lab and oklch input back to the color", () => {
    const formats = colorFormats({ r: 255, g: 99, b: 99, a: 1 });
    const lab = formats.find((f) => f.id === "lab")!.value;
    const oklch = formats.find((f) => f.id === "oklch")!.value;
    expect(parseColor(lab)?.rgba).toEqual({ r: 255, g: 99, b: 99, a: 1 });
    expect(parseColor(oklch)?.rgba).toEqual({ r: 255, g: 99, b: 99, a: 1 });
    expect(parseColor(lab)?.format).toBe("LAB");
    expect(parseColor("lab(200% 0 0)")?.rgba).toEqual({ r: 255, g: 255, b: 255, a: 1 });
  });
});

describe("calculate", () => {
  it("answers unit queries end to end", async () => {
    const result = await calculate("2rem in px");
    expect(result?.output.text).toBe("32 px");
    expect(result?.output.label).toBe("Length");
    const pt = await calculate("90 minutos em horas");
    expect(pt?.output.text).toBe("1.5 h");
    expect(pt?.note).toBe("1 hour 30 minutes");
  });

  it("answers date queries end to end", async () => {
    const result = await calculate("in 3 weeks");
    expect(result?.output.label).toBe("Date");
    expect(result?.kind).toBe("dateTime");
  });
});
