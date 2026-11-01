export type UnitCategory = "length" | "area" | "volume" | "mass" | "temperature" | "speed" | "time" | "energy" | "power" | "pressure" | "force" | "data";

export interface Unit {
  id: string;
  category: UnitCategory;
  factor: number;
  names: string[];
}

const px = (ppi: number) => 0.0254 / ppi;

const BASE_UNITS: Unit[] = [
  { id: "mm", category: "length", factor: 0.001, names: ["mm", "millimeter", "millimeters", "milimetro", "milimetros"] },
  { id: "cm", category: "length", factor: 0.01, names: ["cm", "centimeter", "centimeters", "centimetro", "centimetros"] },
  { id: "m", category: "length", factor: 1, names: ["m", "meter", "meters", "metro", "metros"] },
  { id: "km", category: "length", factor: 1000, names: ["km", "kilometer", "kilometers", "quilometro", "quilometros", "kilometro", "kilometros"] },
  { id: "µm", category: "length", factor: 1e-6, names: ["um", "µm", "micrometer", "micrometers", "micrometro", "micrometros"] },
  { id: "in", category: "length", factor: 0.0254, names: ["in", "inch", "inches", "polegada", "polegadas"] },
  { id: "ft", category: "length", factor: 0.3048, names: ["ft", "foot", "feet", "pe", "pes"] },
  { id: "yd", category: "length", factor: 0.9144, names: ["yd", "yard", "yards", "jarda", "jardas"] },
  { id: "mi", category: "length", factor: 1609.344, names: ["mi", "mile", "miles", "milha", "milhas"] },
  { id: "nmi", category: "length", factor: 1852, names: ["nmi", "nautical mile", "nautical miles", "milha nautica", "milhas nauticas"] },
  { id: "px", category: "length", factor: px(96), names: ["px", "pixel", "pixels"] },
  { id: "em", category: "length", factor: 16 * px(96), names: ["em", "ems"] },
  { id: "rem", category: "length", factor: 16 * px(96), names: ["rem", "rems"] },
  { id: "pt", category: "length", factor: 0.0254 / 72, names: ["pt", "point", "points", "ponto", "pontos"] },
  { id: "pc", category: "length", factor: 12 * (0.0254 / 72), names: ["pc", "pica", "picas"] },
  { id: "q", category: "length", factor: 0.00025, names: ["q"] },

  { id: "mm²", category: "area", factor: 1e-6, names: ["mm2", "mm²", "square millimeter", "square millimeters", "milimetro quadrado", "milimetros quadrados"] },
  { id: "cm²", category: "area", factor: 1e-4, names: ["cm2", "cm²", "square centimeter", "square centimeters", "centimetro quadrado", "centimetros quadrados"] },
  { id: "m²", category: "area", factor: 1, names: ["m2", "m²", "square meter", "square meters", "metro quadrado", "metros quadrados"] },
  { id: "km²", category: "area", factor: 1e6, names: ["km2", "km²", "square kilometer", "square kilometers", "quilometro quadrado", "quilometros quadrados"] },
  { id: "in²", category: "area", factor: 0.0254 ** 2, names: ["in2", "in²", "square inch", "square inches", "polegada quadrada", "polegadas quadradas"] },
  { id: "ft²", category: "area", factor: 0.3048 ** 2, names: ["ft2", "ft²", "square foot", "square feet", "pe quadrado", "pes quadrados"] },
  { id: "yd²", category: "area", factor: 0.9144 ** 2, names: ["yd2", "yd²", "square yard", "square yards", "jarda quadrada", "jardas quadradas"] },
  { id: "mi²", category: "area", factor: 1609.344 ** 2, names: ["mi2", "mi²", "square mile", "square miles", "milha quadrada", "milhas quadradas"] },
  { id: "ha", category: "area", factor: 10000, names: ["ha", "hectare", "hectares"] },
  { id: "ac", category: "area", factor: 4046.8564224, names: ["ac", "acre", "acres"] },

  { id: "ml", category: "volume", factor: 0.001, names: ["ml", "milliliter", "milliliters", "mililitro", "mililitros"] },
  { id: "l", category: "volume", factor: 1, names: ["l", "liter", "liters", "litro", "litros"] },
  { id: "m³", category: "volume", factor: 1000, names: ["m3", "m³", "cubic meter", "cubic meters", "metro cubico", "metros cubicos"] },
  { id: "tsp", category: "volume", factor: 0.00492892159375, names: ["tsp", "teaspoon", "teaspoons", "colher de cha", "colheres de cha"] },
  { id: "tbsp", category: "volume", factor: 0.01478676478125, names: ["tbsp", "tablespoon", "tablespoons", "colher de sopa", "colheres de sopa"] },
  { id: "fl oz", category: "volume", factor: 0.0295735295625, names: ["floz", "fl oz", "fluid ounce", "fluid ounces", "onca fluida", "oncas fluidas"] },
  { id: "cup", category: "volume", factor: 0.2365882365, names: ["cup", "cups", "xicara", "xicaras"] },
  { id: "pint", category: "volume", factor: 0.473176473, names: ["pint", "pints"] },
  { id: "quart", category: "volume", factor: 0.946352946, names: ["quart", "quarts"] },
  { id: "gal", category: "volume", factor: 3.785411784, names: ["gal", "gallon", "gallons", "galao", "galoes"] },

  { id: "mg", category: "mass", factor: 1e-6, names: ["mg", "milligram", "milligrams", "miligrama", "miligramas"] },
  { id: "g", category: "mass", factor: 0.001, names: ["g", "gram", "grams", "grama", "gramas"] },
  { id: "kg", category: "mass", factor: 1, names: ["kg", "kilo", "kilos", "kilogram", "kilograms", "quilograma", "quilogramas", "quilo", "quilos"] },
  { id: "t", category: "mass", factor: 1000, names: ["t", "ton", "tons", "tonne", "tonnes", "tonelada", "toneladas"] },
  { id: "oz", category: "mass", factor: 0.028349523125, names: ["oz", "ounce", "ounces", "onca", "oncas"] },
  { id: "lb", category: "mass", factor: 0.45359237, names: ["lb", "lbs", "pound", "pounds", "libra", "libras"] },
  { id: "st", category: "mass", factor: 6.35029318, names: ["st", "stone", "stones"] },

  { id: "°C", category: "temperature", factor: 1, names: ["c", "°c", "ºc", "celsius"] },
  { id: "°F", category: "temperature", factor: 1, names: ["f", "°f", "ºf", "fahrenheit"] },
  { id: "K", category: "temperature", factor: 1, names: ["k", "kelvin"] },

  { id: "m/s", category: "speed", factor: 1, names: ["m/s", "mps", "metro por segundo", "metros por segundo"] },
  { id: "km/h", category: "speed", factor: 1 / 3.6, names: ["km/h", "kmh", "kph", "quilometro por hora", "quilometros por hora"] },
  { id: "mph", category: "speed", factor: 0.44704, names: ["mph", "mile per hour", "miles per hour", "milha por hora", "milhas por hora"] },
  { id: "kn", category: "speed", factor: 0.5144444444444445, names: ["kn", "knot", "knots"] },
  { id: "ft/s", category: "speed", factor: 0.3048, names: ["ft/s", "fps", "feet per second", "pes por segundo"] },

  { id: "ms", category: "time", factor: 0.001, names: ["ms", "millisecond", "milliseconds", "milissegundo", "milissegundos"] },
  { id: "s", category: "time", factor: 1, names: ["s", "sec", "secs", "second", "seconds", "segundo", "segundos"] },
  { id: "min", category: "time", factor: 60, names: ["min", "mins", "minute", "minutes", "minuto", "minutos"] },
  { id: "h", category: "time", factor: 3600, names: ["h", "hr", "hrs", "hour", "hours", "hora", "horas"] },
  { id: "d", category: "time", factor: 86400, names: ["d", "day", "days", "dia", "dias"] },
  { id: "wk", category: "time", factor: 604800, names: ["wk", "week", "weeks", "semana", "semanas"] },
  { id: "mo", category: "time", factor: 2629746, names: ["mo", "month", "months", "mes", "meses"] },
  { id: "yr", category: "time", factor: 31556952, names: ["y", "yr", "year", "years", "ano", "anos"] },
  { id: "work hour", category: "time", factor: 3600, names: ["work hour", "work hours", "hora util", "horas uteis"] },
  { id: "work day", category: "time", factor: 28800, names: ["wd", "work day", "work days", "dia util", "dias uteis"] },
  { id: "work week", category: "time", factor: 144000, names: ["ww", "work week", "work weeks", "semana util", "semanas uteis"] },

  { id: "J", category: "energy", factor: 1, names: ["j", "joule", "joules"] },
  { id: "kJ", category: "energy", factor: 1000, names: ["kj", "kilojoule", "kilojoules", "quilojoule", "quilojoules"] },
  { id: "cal", category: "energy", factor: 4.184, names: ["cal", "calorie", "calories", "caloria", "calorias"] },
  { id: "kcal", category: "energy", factor: 4184, names: ["kcal", "kilocalorie", "kilocalories", "quilocaloria", "quilocalorias"] },
  { id: "Wh", category: "energy", factor: 3600, names: ["wh", "watt hour", "watt hours", "watt-hour"] },
  { id: "kWh", category: "energy", factor: 3.6e6, names: ["kwh", "kilowatt hour", "kilowatt hours", "quilowatt hora", "quilowatt horas"] },
  { id: "BTU", category: "energy", factor: 1055.05585262, names: ["btu"] },

  { id: "mW", category: "power", factor: 0.001, names: ["mw", "milliwatt", "milliwatts", "miliwatt", "miliwatts"] },
  { id: "W", category: "power", factor: 1, names: ["w", "watt", "watts"] },
  { id: "kW", category: "power", factor: 1000, names: ["kw", "kilowatt", "kilowatts", "quilowatt", "quilowatts"] },
  { id: "MW", category: "power", factor: 1e6, names: ["megawatt", "megawatts"] },
  { id: "GW", category: "power", factor: 1e9, names: ["gw", "gigawatt", "gigawatts"] },
  { id: "hp", category: "power", factor: 745.6998715822702, names: ["hp", "horsepower", "cv", "cavalo vapor", "cavalos"] },

  { id: "Pa", category: "pressure", factor: 1, names: ["pa", "pascal", "pascals"] },
  { id: "hPa", category: "pressure", factor: 100, names: ["hpa", "hectopascal", "hectopascals"] },
  { id: "kPa", category: "pressure", factor: 1000, names: ["kpa", "kilopascal", "kilopascals"] },
  { id: "MPa", category: "pressure", factor: 1e6, names: ["mpa", "megapascal", "megapascals"] },
  { id: "bar", category: "pressure", factor: 1e5, names: ["bar", "bares"] },
  { id: "mbar", category: "pressure", factor: 100, names: ["mbar", "millibar", "millibars", "milibar", "milibares"] },
  { id: "atm", category: "pressure", factor: 101325, names: ["atm", "atmosphere", "atmospheres", "atmosfera", "atmosferas"] },
  { id: "psi", category: "pressure", factor: 6894.757293168361, names: ["psi"] },
  { id: "mmHg", category: "pressure", factor: 133.322387415, names: ["mmhg", "mm hg", "millimeter of mercury", "milimetro de mercurio"] },
  { id: "Torr", category: "pressure", factor: 101325 / 760, names: ["torr"] },

  { id: "mN", category: "force", factor: 0.001, names: ["mn", "millinewton", "millinewtons", "milinewton", "milinewtons"] },
  { id: "N", category: "force", factor: 1, names: ["n", "newton", "newtons"] },
  { id: "kN", category: "force", factor: 1000, names: ["kilonewton", "kilonewtons", "quilonewton", "quilonewtons"] },
  { id: "kgf", category: "force", factor: 9.80665, names: ["kgf", "kilogram force", "quilograma forca"] },
  { id: "lbf", category: "force", factor: 4.4482216152605, names: ["lbf", "pound force", "pound-force", "libra forca", "libra-forca"] },

  { id: "bit", category: "data", factor: 0.125, names: ["bit", "bits"] },
  { id: "B", category: "data", factor: 1, names: ["b", "byte", "bytes"] },
  { id: "KB", category: "data", factor: 1e3, names: ["kb", "kilobyte", "kilobytes"] },
  { id: "MB", category: "data", factor: 1e6, names: ["mb", "megabyte", "megabytes"] },
  { id: "GB", category: "data", factor: 1e9, names: ["gb", "gigabyte", "gigabytes"] },
  { id: "TB", category: "data", factor: 1e12, names: ["tb", "terabyte", "terabytes"] },
  { id: "KiB", category: "data", factor: 1024, names: ["kib", "kibibyte", "kibibytes"] },
  { id: "MiB", category: "data", factor: 1024 ** 2, names: ["mib", "mebibyte", "mebibytes"] },
  { id: "GiB", category: "data", factor: 1024 ** 3, names: ["gib", "gibibyte", "gibibytes"] },
  { id: "TiB", category: "data", factor: 1024 ** 4, names: ["tib", "tebibyte", "tebibytes"] },
];

const CONNECTORS = ["in", "to", "as", "para", "em", "como", "pra", "pro"];

function unaccent(text: string): string {
  return text.normalize("NFD").replace(/[\u0300-\u036f]/g, "");
}

function aliasIndex(ppi: number): Map<string, Unit> {
  const index = new Map<string, Unit>();
  const scaled = BASE_UNITS.map((unit) => (unit.id === "px" ? { ...unit, factor: px(ppi) } : unit));
  for (const unit of scaled) for (const name of unit.names) index.set(name, unit);
  return index;
}

function alternation(index: Map<string, Unit>): string {
  return [...index.keys()].sort((a, b) => b.length - a.length).map((name) => name.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")).join("|");
}

export interface UnitQuery {
  amount: number;
  from: Unit;
  to: Unit;
  value: number;
}

export function convertUnits(amount: number, from: Unit, to: Unit): number | null {
  if (from.category !== to.category) return null;
  if (from.category === "temperature") {
    const celsius = from.id === "°C" ? amount : from.id === "°F" ? ((amount - 32) * 5) / 9 : amount - 273.15;
    return to.id === "°C" ? celsius : to.id === "°F" ? (celsius * 9) / 5 + 32 : celsius + 273.15;
  }
  return (amount * from.factor) / to.factor;
}

export function parseUnitQuery(query: string, ppi = 96): UnitQuery | null {
  let text = query.trim();
  let scale = ppi;
  const ppiMatch = text.match(/\s*(?:@|at|a|com)\s*(\d+(?:[.,]\d+)?)\s*ppi$/i);
  if (ppiMatch) {
    scale = Number(ppiMatch[1].replace(",", "."));
    text = text.slice(0, ppiMatch.index).trim();
  }
  const index = aliasIndex(scale);
  const unit = `(?:${alternation(index)})`;
  const connector = CONNECTORS.join("|");
  const match = text.match(new RegExp(`^(-?\\d+(?:[.,]\\d+)?)\\s*(${unit})\\s+(?:${connector}|=)\\s+(${unit})$`, "i"));
  if (!match) return null;
  const from = index.get(unaccent(match[2].toLowerCase()));
  const to = index.get(unaccent(match[3].toLowerCase()));
  if (!from || !to) return null;
  const value = convertUnits(Number(match[1].replace(",", ".")), from, to);
  if (value === null || !Number.isFinite(value)) return null;
  return { amount: Number(match[1].replace(",", ".")), from, to, value };
}

const CATEGORY_LABELS: Record<UnitCategory, string> = {
  length: "Length",
  area: "Area",
  volume: "Volume",
  mass: "Mass",
  temperature: "Temperature",
  speed: "Speed",
  time: "Time",
  energy: "Energy",
  power: "Power",
  pressure: "Pressure",
  force: "Force",
  data: "Data",
};

export function unitCategoryLabel(category: UnitCategory): string {
  return CATEGORY_LABELS[category];
}

const DURATION_PARTS: [number, string][] = [
  [86400, "day"],
  [3600, "hour"],
  [60, "minute"],
  [1, "second"],
];

export function formatDuration(seconds: number): string {
  const parts: string[] = [];
  let rest = Math.abs(seconds);
  for (const [size, name] of DURATION_PARTS) {
    const count = Math.floor(rest / size);
    if (count > 0 || (parts.length === 0 && size === 1)) {
      parts.push(`${count} ${count === 1 ? name : `${name}s`}`);
      rest -= count * size;
    }
    if (parts.length === 2) break;
  }
  return (seconds < 0 ? "-" : "") + parts.join(" ");
}

export function formatUnitValue(value: number): string {
  const rounded = Number(value.toPrecision(12));
  return new Intl.NumberFormat("en-US", { maximumFractionDigits: 6 }).format(rounded);
}
