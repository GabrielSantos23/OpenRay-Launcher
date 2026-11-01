import { describe, expect, it } from "vitest";
import { convertUnits, formatDuration, formatUnitValue, parseUnitQuery } from "./units";

describe("parseUnitQuery", () => {
  it("parses English unit queries", () => {
    expect(parseUnitQuery("10 km in miles")?.value).toBeCloseTo(6.2137119223733395, 6);
    expect(parseUnitQuery("23 C to F")?.value).toBeCloseTo(73.4, 6);
    expect(parseUnitQuery("23 °C to °F")?.value).toBeCloseTo(73.4, 6);
    expect(parseUnitQuery("2rem in px")?.value).toBe(32);
    expect(parseUnitQuery("1 in in px")?.value).toBe(96);
    expect(parseUnitQuery("1 inch to cm")?.value).toBeCloseTo(2.54, 6);
    expect(parseUnitQuery("100 lb in kg")?.value).toBeCloseTo(45.359237, 4);
    expect(parseUnitQuery("90 minutes in hours")?.value).toBe(1.5);
    expect(parseUnitQuery("1 GB in MB")?.value).toBe(1000);
    expect(parseUnitQuery("1 GiB in MB")?.value).toBeCloseTo(1073.741824, 4);
    expect(parseUnitQuery("1 acre in ha")?.value).toBeCloseTo(0.404685642, 6);
    expect(parseUnitQuery("1 hp in W")?.value).toBeCloseTo(745.699872, 3);
    expect(parseUnitQuery("1 psi in bar")?.value).toBeCloseTo(0.0689476, 4);
    expect(parseUnitQuery("0 K in C")?.value).toBeCloseTo(-273.15, 6);
  });

  it("scales CSS pixels with an explicit PPI", () => {
    expect(parseUnitQuery("1 in to px at 300 ppi")?.value).toBe(300);
    expect(parseUnitQuery("1 inch to px @300ppi")?.value).toBe(300);
    expect(parseUnitQuery("2rem in px at 300 ppi")?.value).toBeCloseTo(100, 6);
  });

  it("parses Portuguese unit queries", () => {
    expect(parseUnitQuery("10 km em milhas")?.value).toBeCloseTo(6.2137119223733395, 6);
    expect(parseUnitQuery("2 quilos em libras")?.value).toBeCloseTo(4.409245243697564, 4);
    expect(parseUnitQuery("23 c para f")?.value).toBeCloseTo(73.4, 6);
    expect(parseUnitQuery("90 minutos em horas")?.value).toBe(1.5);
    expect(parseUnitQuery("10 litros em galoes")?.value).toBeCloseTo(2.6417205124, 6);
    expect(parseUnitQuery("1 polegada em cm")?.value).toBeCloseTo(2.54, 6);
    expect(parseUnitQuery("8 horas uteis em dias uteis")?.value).toBe(1);
    expect(parseUnitQuery("2 hectares em m2")?.value).toBe(20000);
  });

  it("rejects cross-category and non-unit queries", () => {
    expect(parseUnitQuery("10 km in kg")).toBeNull();
    expect(parseUnitQuery("100 usd in brl")).toBeNull();
    expect(parseUnitQuery("100 dolares para brl")).toBeNull();
    expect(parseUnitQuery("hello world")).toBeNull();
  });
});

describe("convertUnits and formatting", () => {
  it("keeps negative temperatures", () => {
    expect(convertUnits(-40, { id: "°C", category: "temperature", factor: 1, names: [] }, { id: "°F", category: "temperature", factor: 1, names: [] })).toBe(-40);
  });

  it("formats values like the reference", () => {
    expect(formatUnitValue(6.2137119223733395)).toBe("6.213712");
    expect(formatUnitValue(73.4)).toBe("73.4");
    expect(formatUnitValue(20000)).toBe("20,000");
  });

  it("formats readable durations", () => {
    expect(formatDuration(9000)).toBe("2 hours 30 minutes");
    expect(formatDuration(65)).toBe("1 minute 5 seconds");
    expect(formatDuration(60 * 60 * 49)).toBe("2 days 1 hour");
  });
});
