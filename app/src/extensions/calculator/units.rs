use super::math::{group, to_precision, unaccent, CONNECTORS};
use regex::Regex;
use std::collections::HashMap;
use std::sync::LazyLock;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Category {
    Length,
    Area,
    Volume,
    Mass,
    Temperature,
    Speed,
    Time,
    Energy,
    Power,
    Pressure,
    Force,
    Data,
}

impl Category {
    pub fn label(self) -> &'static str {
        match self {
            Category::Length => "Length",
            Category::Area => "Area",
            Category::Volume => "Volume",
            Category::Mass => "Mass",
            Category::Temperature => "Temperature",
            Category::Speed => "Speed",
            Category::Time => "Time",
            Category::Energy => "Energy",
            Category::Power => "Power",
            Category::Pressure => "Pressure",
            Category::Force => "Force",
            Category::Data => "Data",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Unit {
    pub id: &'static str,
    pub category: Category,
    pub factor: f64,
    pub names: &'static [&'static str],
}

const INCH: f64 = 0.0254;
const CSS_PX: f64 = INCH / 96.0;

macro_rules! unit {
    ($id:expr, $cat:ident, $factor:expr, [$($name:expr),*]) => {
        Unit { id: $id, category: Category::$cat, factor: $factor, names: &[$($name),*] }
    };
}

const UNITS: &[Unit] = &[
    unit!("mm", Length, 0.001, ["mm", "millimeter", "millimeters", "milimetro", "milimetros"]),
    unit!("cm", Length, 0.01, ["cm", "centimeter", "centimeters", "centimetro", "centimetros"]),
    unit!("m", Length, 1.0, ["m", "meter", "meters", "metro", "metros"]),
    unit!("km", Length, 1000.0, ["km", "kilometer", "kilometers", "quilometro", "quilometros", "kilometro", "kilometros"]),
    unit!("µm", Length, 1e-6, ["um", "µm", "micrometer", "micrometers", "micrometro", "micrometros"]),
    unit!("in", Length, INCH, ["in", "inch", "inches", "polegada", "polegadas"]),
    unit!("ft", Length, 0.3048, ["ft", "foot", "feet", "pe", "pes"]),
    unit!("yd", Length, 0.9144, ["yd", "yard", "yards", "jarda", "jardas"]),
    unit!("mi", Length, 1609.344, ["mi", "mile", "miles", "milha", "milhas"]),
    unit!("nmi", Length, 1852.0, ["nmi", "nautical mile", "nautical miles", "milha nautica", "milhas nauticas"]),
    unit!("px", Length, CSS_PX, ["px", "pixel", "pixels"]),
    unit!("em", Length, 16.0 * CSS_PX, ["em", "ems"]),
    unit!("rem", Length, 16.0 * CSS_PX, ["rem", "rems"]),
    unit!("pt", Length, INCH / 72.0, ["pt", "point", "points", "ponto", "pontos"]),
    unit!("pc", Length, 12.0 * (INCH / 72.0), ["pc", "pica", "picas"]),
    unit!("q", Length, 0.00025, ["q"]),
    unit!("mm²", Area, 1e-6, ["mm2", "mm²", "square millimeter", "square millimeters", "milimetro quadrado", "milimetros quadrados"]),
    unit!("cm²", Area, 1e-4, ["cm2", "cm²", "square centimeter", "square centimeters", "centimetro quadrado", "centimetros quadrados"]),
    unit!("m²", Area, 1.0, ["m2", "m²", "square meter", "square meters", "metro quadrado", "metros quadrados"]),
    unit!("km²", Area, 1e6, ["km2", "km²", "square kilometer", "square kilometers", "quilometro quadrado", "quilometros quadrados"]),
    unit!("in²", Area, INCH * INCH, ["in2", "in²", "square inch", "square inches", "polegada quadrada", "polegadas quadradas"]),
    unit!("ft²", Area, 0.3048 * 0.3048, ["ft2", "ft²", "square foot", "square feet", "pe quadrado", "pes quadrados"]),
    unit!("yd²", Area, 0.9144 * 0.9144, ["yd2", "yd²", "square yard", "square yards", "jarda quadrada", "jardas quadradas"]),
    unit!("mi²", Area, 1609.344 * 1609.344, ["mi2", "mi²", "square mile", "square miles", "milha quadrada", "milhas quadradas"]),
    unit!("ha", Area, 10000.0, ["ha", "hectare", "hectares"]),
    unit!("ac", Area, 4046.8564224, ["ac", "acre", "acres"]),
    unit!("ml", Volume, 0.001, ["ml", "milliliter", "milliliters", "mililitro", "mililitros"]),
    unit!("l", Volume, 1.0, ["l", "liter", "liters", "litro", "litros"]),
    unit!("m³", Volume, 1000.0, ["m3", "m³", "cubic meter", "cubic meters", "metro cubico", "metros cubicos"]),
    unit!("tsp", Volume, 0.00492892159375, ["tsp", "teaspoon", "teaspoons", "colher de cha", "colheres de cha"]),
    unit!("tbsp", Volume, 0.01478676478125, ["tbsp", "tablespoon", "tablespoons", "colher de sopa", "colheres de sopa"]),
    unit!("fl oz", Volume, 0.0295735295625, ["floz", "fl oz", "fluid ounce", "fluid ounces", "onca fluida", "oncas fluidas"]),
    unit!("cup", Volume, 0.2365882365, ["cup", "cups", "xicara", "xicaras"]),
    unit!("pint", Volume, 0.473176473, ["pint", "pints"]),
    unit!("quart", Volume, 0.946352946, ["quart", "quarts"]),
    unit!("gal", Volume, 3.785411784, ["gal", "gallon", "gallons", "galao", "galoes"]),
    unit!("mg", Mass, 1e-6, ["mg", "milligram", "milligrams", "miligrama", "miligramas"]),
    unit!("g", Mass, 0.001, ["g", "gram", "grams", "grama", "gramas"]),
    unit!("kg", Mass, 1.0, ["kg", "kilo", "kilos", "kilogram", "kilograms", "quilograma", "quilogramas", "quilo", "quilos"]),
    unit!("t", Mass, 1000.0, ["t", "ton", "tons", "tonne", "tonnes", "tonelada", "toneladas"]),
    unit!("oz", Mass, 0.028349523125, ["oz", "ounce", "ounces", "onca", "oncas"]),
    unit!("lb", Mass, 0.45359237, ["lb", "lbs", "pound", "pounds", "libra", "libras"]),
    unit!("st", Mass, 6.35029318, ["st", "stone", "stones"]),
    unit!("°C", Temperature, 1.0, ["c", "°c", "ºc", "celsius"]),
    unit!("°F", Temperature, 1.0, ["f", "°f", "ºf", "fahrenheit"]),
    unit!("K", Temperature, 1.0, ["k", "kelvin"]),
    unit!("m/s", Speed, 1.0, ["m/s", "mps", "metro por segundo", "metros por segundo"]),
    unit!("km/h", Speed, 1.0 / 3.6, ["km/h", "kmh", "kph", "quilometro por hora", "quilometros por hora"]),
    unit!("km/s", Speed, 1000.0, ["km/s", "kps", "kilometer per second", "kilometers per second", "quilometro por segundo", "quilometros por segundo"]),
    unit!("mi/s", Speed, 1609.344, ["mi/s", "mile per second", "miles per second"]),
    unit!("mph", Speed, 0.44704, ["mph", "mile per hour", "miles per hour", "milha por hora", "milhas por hora"]),
    unit!("kn", Speed, 0.5144444444444445, ["kn", "knot", "knots"]),
    unit!("ft/s", Speed, 0.3048, ["ft/s", "fps", "feet per second", "pes por segundo"]),
    unit!("ms", Time, 0.001, ["ms", "millisecond", "milliseconds", "milissegundo", "milissegundos"]),
    unit!("s", Time, 1.0, ["s", "sec", "secs", "second", "seconds", "segundo", "segundos"]),
    unit!("min", Time, 60.0, ["min", "mins", "minute", "minutes", "minuto", "minutos"]),
    unit!("h", Time, 3600.0, ["h", "hr", "hrs", "hour", "hours", "hora", "horas"]),
    unit!("d", Time, 86400.0, ["d", "day", "days", "dia", "dias"]),
    unit!("wk", Time, 604800.0, ["wk", "week", "weeks", "semana", "semanas"]),
    unit!("mo", Time, 2629746.0, ["mo", "month", "months", "mes", "meses"]),
    unit!("yr", Time, 31556952.0, ["y", "yr", "year", "years", "ano", "anos"]),
    unit!("work hour", Time, 3600.0, ["work hour", "work hours", "hora util", "horas uteis"]),
    unit!("work day", Time, 28800.0, ["wd", "work day", "work days", "dia util", "dias uteis"]),
    unit!("work week", Time, 144000.0, ["ww", "work week", "work weeks", "semana util", "semanas uteis"]),
    unit!("J", Energy, 1.0, ["j", "joule", "joules"]),
    unit!("kJ", Energy, 1000.0, ["kj", "kilojoule", "kilojoules", "quilojoule", "quilojoules"]),
    unit!("cal", Energy, 4.184, ["cal", "calorie", "calories", "caloria", "calorias"]),
    unit!("kcal", Energy, 4184.0, ["kcal", "kilocalorie", "kilocalories", "quilocaloria", "quilocalorias"]),
    unit!("Wh", Energy, 3600.0, ["wh", "watt hour", "watt hours", "watt-hour"]),
    unit!("kWh", Energy, 3.6e6, ["kwh", "kilowatt hour", "kilowatt hours", "quilowatt hora", "quilowatt horas"]),
    unit!("BTU", Energy, 1055.05585262, ["btu"]),
    unit!("mW", Power, 0.001, ["mw", "milliwatt", "milliwatts", "miliwatt", "miliwatts"]),
    unit!("W", Power, 1.0, ["w", "watt", "watts"]),
    unit!("kW", Power, 1000.0, ["kw", "kilowatt", "kilowatts", "quilowatt", "quilowatts"]),
    unit!("MW", Power, 1e6, ["megawatt", "megawatts"]),
    unit!("GW", Power, 1e9, ["gw", "gigawatt", "gigawatts"]),
    unit!("hp", Power, 745.6998715822702, ["hp", "horsepower", "cv", "cavalo vapor", "cavalos"]),
    unit!("Pa", Pressure, 1.0, ["pa", "pascal", "pascals"]),
    unit!("hPa", Pressure, 100.0, ["hpa", "hectopascal", "hectopascals"]),
    unit!("kPa", Pressure, 1000.0, ["kpa", "kilopascal", "kilopascals"]),
    unit!("MPa", Pressure, 1e6, ["mpa", "megapascal", "megapascals"]),
    unit!("bar", Pressure, 1e5, ["bar", "bares"]),
    unit!("mbar", Pressure, 100.0, ["mbar", "millibar", "millibars", "milibar", "milibares"]),
    unit!("atm", Pressure, 101325.0, ["atm", "atmosphere", "atmospheres", "atmosfera", "atmosferas"]),
    unit!("psi", Pressure, 6894.757293168361, ["psi"]),
    unit!("mmHg", Pressure, 133.322387415, ["mmhg", "mm hg", "millimeter of mercury", "milimetro de mercurio"]),
    unit!("Torr", Pressure, 101325.0 / 760.0, ["torr"]),
    unit!("mN", Force, 0.001, ["mn", "millinewton", "millinewtons", "milinewton", "milinewtons"]),
    unit!("N", Force, 1.0, ["n", "newton", "newtons"]),
    unit!("kN", Force, 1000.0, ["kilonewton", "kilonewtons", "quilonewton", "quilonewtons"]),
    unit!("kgf", Force, 9.80665, ["kgf", "kilogram force", "quilograma forca"]),
    unit!("lbf", Force, 4.4482216152605, ["lbf", "pound force", "pound-force", "libra forca", "libra-forca"]),
    unit!("bit", Data, 0.125, ["bit", "bits"]),
    unit!("B", Data, 1.0, ["b", "byte", "bytes"]),
    unit!("KB", Data, 1e3, ["kb", "kilobyte", "kilobytes"]),
    unit!("MB", Data, 1e6, ["mb", "megabyte", "megabytes"]),
    unit!("GB", Data, 1e9, ["gb", "gigabyte", "gigabytes"]),
    unit!("TB", Data, 1e12, ["tb", "terabyte", "terabytes"]),
    unit!("KiB", Data, 1024.0, ["kib", "kibibyte", "kibibytes"]),
    unit!("MiB", Data, 1048576.0, ["mib", "mebibyte", "mebibytes"]),
    unit!("GiB", Data, 1073741824.0, ["gib", "gibibyte", "gibibytes"]),
    unit!("TiB", Data, 1099511627776.0, ["tib", "tebibyte", "tebibytes"]),
];

static INDEX: LazyLock<HashMap<&'static str, &'static Unit>> = LazyLock::new(|| UNITS.iter().flat_map(|u| u.names.iter().map(move |n| (*n, u))).collect());

static QUERY: LazyLock<Regex> = LazyLock::new(|| {
    let mut names: Vec<&str> = INDEX.keys().copied().collect();
    names.sort_by(|a, b| b.chars().count().cmp(&a.chars().count()).then(a.cmp(b)));
    let unit = format!("(?:{})", names.iter().map(|n| regex::escape(n)).collect::<Vec<_>>().join("|"));
    Regex::new(&format!(r"(?i)^(-?\d+(?:[.,]\d+)?)\s*({unit})\s+(?:{CONNECTORS}|=)\s+({unit})$")).unwrap()
});

static PPI: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)\s*(?:@|at|a|com)\s*(\d+(?:[.,]\d+)?)\s*ppi$").unwrap());

#[derive(Debug)]
pub struct UnitQuery {
    pub from: Unit,
    pub to: Unit,
    pub value: f64,
}

pub fn convert(amount: f64, from: &Unit, to: &Unit) -> Option<f64> {
    if from.category != to.category {
        return None;
    }
    if from.category == Category::Temperature {
        let celsius = match from.id {
            "°C" => amount,
            "°F" => (amount - 32.0) * 5.0 / 9.0,
            _ => amount - 273.15,
        };
        return Some(match to.id {
            "°C" => celsius,
            "°F" => celsius * 9.0 / 5.0 + 32.0,
            _ => celsius + 273.15,
        });
    }
    Some(amount * from.factor / to.factor)
}

fn lookup(name: &str, ppi: f64) -> Option<Unit> {
    let unit = **INDEX.get(unaccent(&name.to_lowercase()).as_str())?;
    Some(if unit.id == "px" { Unit { factor: INCH / ppi, ..unit } } else { unit })
}

pub fn parse(query: &str) -> Option<UnitQuery> {
    let mut text = query.trim();
    let mut ppi = 96.0;
    if let Some(caps) = PPI.captures(text) {
        ppi = caps[1].replace(',', ".").parse().ok()?;
        text = text[..caps.get(0)?.start()].trim();
    }
    let caps = QUERY.captures(text)?;
    let from = lookup(&caps[2], ppi)?;
    let to = lookup(&caps[3], ppi)?;
    let value = convert(caps[1].replace(',', ".").parse().ok()?, &from, &to)?;
    value.is_finite().then_some(UnitQuery { from, to, value })
}

pub fn format_value(value: f64) -> String {
    group(to_precision(value, 12), 6)
}

pub fn format_duration(seconds: f64) -> String {
    let mut parts = Vec::new();
    let mut rest = seconds.abs();
    for (size, name) in [(86400.0, "day"), (3600.0, "hour"), (60.0, "minute"), (1.0, "second")] {
        let count = (rest / size).floor();
        if count > 0.0 || (parts.is_empty() && size == 1.0) {
            parts.push(format!("{count} {name}{}", if count == 1.0 { "" } else { "s" }));
            rest -= count * size;
        }
        if parts.len() == 2 {
            break;
        }
    }
    format!("{}{}", if seconds < 0.0 { "-" } else { "" }, parts.join(" "))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn value(q: &str) -> f64 {
        parse(q).unwrap_or_else(|| panic!("{q}")).value
    }

    fn near(q: &str, expected: f64, digits: i32) {
        assert!((value(q) - expected).abs() < 0.5 * 10f64.powi(-digits), "{q}: {}", value(q));
    }

    #[test]
    fn parses_english_unit_queries() {
        near("10 km in miles", 6.2137119223733395, 6);
        near("23 C to F", 73.4, 6);
        near("23 °C to °F", 73.4, 6);
        assert_eq!(value("2rem in px"), 32.0);
        assert_eq!(value("1 in in px"), 96.0);
        near("1 inch to cm", 2.54, 6);
        near("100 lb in kg", 45.359237, 4);
        assert_eq!(value("90 minutes in hours"), 1.5);
        assert_eq!(value("1 GB in MB"), 1000.0);
        near("1 GiB in MB", 1073.741824, 4);
        near("1 acre in ha", 0.404685642, 6);
        near("1 hp in W", 745.699872, 3);
        near("1 psi in bar", 0.0689476, 4);
        near("0 K in C", -273.15, 6);
    }

    #[test]
    fn scales_css_pixels_with_an_explicit_ppi() {
        assert_eq!(value("1 in to px at 300 ppi"), 300.0);
        assert_eq!(value("1 inch to px @300ppi"), 300.0);
        near("2rem in px at 300 ppi", 100.0, 6);
    }

    #[test]
    fn parses_portuguese_unit_queries() {
        near("10 km em milhas", 6.2137119223733395, 6);
        near("2 quilos em libras", 4.409245243697564, 4);
        near("23 c para f", 73.4, 6);
        assert_eq!(value("90 minutos em horas"), 1.5);
        near("10 litros em galoes", 2.6417205124, 6);
        near("1 polegada em cm", 2.54, 6);
        assert_eq!(value("8 horas uteis em dias uteis"), 1.0);
        assert_eq!(value("2 hectares em m2"), 20000.0);
    }

    #[test]
    fn rejects_cross_category_and_non_unit_queries() {
        for q in ["10 km in kg", "100 usd in brl", "100 dolares para brl", "hello world"] {
            assert!(parse(q).is_none(), "{q}");
        }
    }

    #[test]
    fn converts_and_formats() {
        let c = lookup("c", 96.0).unwrap();
        let f = lookup("f", 96.0).unwrap();
        assert_eq!(convert(-40.0, &c, &f), Some(-40.0));
        assert_eq!(format_value(6.2137119223733395), "6.213712");
        assert_eq!(format_value(73.4), "73.4");
        assert_eq!(format_value(20000.0), "20,000");
        assert_eq!(format_duration(9000.0), "2 hours 30 minutes");
        assert_eq!(format_duration(65.0), "1 minute 5 seconds");
        assert_eq!(format_duration(60.0 * 60.0 * 49.0), "2 days 1 hour");
    }
}
