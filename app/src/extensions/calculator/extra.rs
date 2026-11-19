use super::math::{self, format_number, group, plain_number, to_precision};
use super::units;
use regex::Regex;
use std::sync::LazyLock;

pub struct Pure {
    pub display: String,
    pub plain: String,
    pub label: String,
    pub dim: Vec<(usize, usize)>,
    pub unit_like: bool,
}

fn pure(display: impl Into<String>, plain: impl Into<String>, label: impl Into<String>) -> Pure {
    Pure { display: display.into(), plain: plain.into(), label: label.into(), dim: Vec::new(), unit_like: false }
}

fn re(pattern: &str) -> Regex {
    Regex::new(pattern).expect("calculator pattern")
}

fn plural(count: f64, word: &str) -> String {
    format!("{} {word}{}", format_number(count), if count == 1.0 { "" } else { "s" })
}

pub const DAY_MS: f64 = 86_400_000.0;

pub fn civil(days: i64) -> (i64, i64, i64) {
    super::history::civil_from_days(days)
}

pub fn days_of(year: i64, month: i64, day: i64) -> i64 {
    super::history::days_from_civil(year, month, day)
}

fn month_length(year: i64, month: i64) -> i64 {
    let (ny, nm) = if month == 12 { (year + 1, 1) } else { (year, month + 1) };
    days_of(ny, nm, 1) - days_of(year, month, 1)
}

pub fn iso_week_start(year: i64, week: i64) -> i64 {
    let jan4 = days_of(year, 1, 4);
    let weekday = (jan4 + 3).rem_euclid(7);
    jan4 - weekday + (week - 1) * 7
}

pub fn iso_week(day: i64) -> (i64, i64) {
    let weekday = (day + 3).rem_euclid(7);
    let thursday = day - weekday + 3;
    let year = civil(thursday).0;
    (year, (thursday - days_of(year, 1, 1)) / 7 + 1)
}

fn weeks_in(year: i64) -> i64 {
    iso_week(days_of(year, 12, 28)).1
}

const MONTHS: [(&str, &str, &str); 12] = [
    ("jan", "january", "janeiro"),
    ("feb", "february", "fevereiro"),
    ("mar", "march", "marco"),
    ("apr", "april", "abril"),
    ("may", "may", "maio"),
    ("jun", "june", "junho"),
    ("jul", "july", "julho"),
    ("aug", "august", "agosto"),
    ("sep", "september", "setembro"),
    ("oct", "october", "outubro"),
    ("nov", "november", "novembro"),
    ("dec", "december", "dezembro"),
];

pub fn month_of(name: &str) -> Option<i64> {
    let name = math::unaccent(name);
    MONTHS.iter().position(|(a, b, c)| name == *a || name == *b || name == *c || (name.len() >= 3 && (b.starts_with(&name) || c.starts_with(&name)))).map(|i| i as i64 + 1)
}

static WEEK: LazyLock<Regex> = LazyLock::new(|| re(r"^(?:week|semana|wk|kw)\s*(\d{1,2})(?:\s*(?:of|de|,)?\s*(\d{4}))?$"));
static WEEK_NOW: LazyLock<Regex> = LazyLock::new(|| re(r"^(?:what week is it|week number|current week|this week number|que semana (?:e|é) (?:hoje|essa))$"));

pub fn week(text: &str, today: i64, format_range: &dyn Fn(i64, i64) -> String) -> Option<Pure> {
    if WEEK_NOW.is_match(text) {
        let (year, week) = iso_week(today);
        return Some(pure(format!("Week {week}"), week.to_string(), format!("{year}")));
    }
    let caps = WEEK.captures(text)?;
    let week: i64 = caps[1].parse().ok()?;
    let year: i64 = caps.get(2).map_or(Some(iso_week(today).0), |y| y.as_str().parse().ok())?;
    if week < 1 || week > weeks_in(year) {
        return None;
    }
    let start = iso_week_start(year, week);
    let range = format_range(start, start + 6);
    Some(pure(range.clone(), range, "Result"))
}

static LEFT: LazyLock<Regex> = LazyLock::new(|| re(r"^(seconds|minutes|hours|days|weeks|months)\s+(?:left|remaining)\s+(?:in|until the end of)\s+(?:this\s+|the\s+)?(today|day|week|month|year)$"));
static SINCE: LazyLock<Regex> = LazyLock::new(|| re(r"^(seconds|minutes|hours|days|weeks|months|years)\s+(since|until|till)\s+(.+)$"));

fn unit_ms(unit: &str) -> f64 {
    match unit {
        "seconds" => 1000.0,
        "minutes" => 60_000.0,
        "hours" => 3_600_000.0,
        "weeks" => 7.0 * DAY_MS,
        "months" => 30.436875 * DAY_MS,
        "years" => 365.2425 * DAY_MS,
        _ => DAY_MS,
    }
}

pub fn time_left(text: &str, local_now_ms: f64) -> Option<Pure> {
    let caps = LEFT.captures(text)?;
    let today = (local_now_ms / DAY_MS).floor() as i64;
    let (year, month, _) = civil(today);
    let end_day = match &caps[2] {
        "today" | "day" => today + 1,
        "week" => today - (today + 3).rem_euclid(7) + 7,
        "month" => days_of(year, month, 1) + month_length(year, month),
        _ => days_of(year + 1, 1, 1),
    };
    let remaining = end_day as f64 * DAY_MS - local_now_ms;
    let unit = &caps[1];
    let count = if unit == "days" { (end_day - today - 1) as f64 } else { (remaining / unit_ms(unit)).floor() };
    let word = unit.trim_end_matches('s');
    let display = plural(count, word);
    Some(Pure { label: format!("Left in {}", if &caps[2] == "day" { "today" } else { &caps[2] }), ..pure(display, count.to_string(), "") })
}

pub fn since(text: &str, local_now_ms: f64, parse_day: &dyn Fn(&str, i64) -> Option<i64>) -> Option<Pure> {
    let caps = SINCE.captures(text)?;
    let today = (local_now_ms / DAY_MS).floor() as i64;
    let target = caps[3].trim();
    let day = match month_of(target) {
        Some(month) => {
            let year = civil(today).0;
            let this_year = days_of(year, month, 1);
            match (&caps[2], this_year > today) {
                ("since", true) => days_of(year - 1, month, 1),
                ("since", false) => this_year,
                (_, true) => this_year,
                _ => days_of(year + 1, month, 1),
            }
        }
        None => parse_day(target, today)?,
    };
    let unit = &caps[1];
    let span = if &caps[2] == "since" { today - day } else { day - today };
    let count = if unit == "days" { span as f64 } else { (span as f64 * DAY_MS / unit_ms(unit)).floor() };
    Some(pure(plural(count, unit.trim_end_matches('s')), count.to_string(), if &caps[2] == "since" { "Since" } else { "Until" }))
}

pub const ZONES: [(&str, &str); 34] = [
    ("utc", "UTC"),
    ("gmt", "UTC"),
    ("z", "UTC"),
    ("pt", "America/Los_Angeles"),
    ("pst", "America/Los_Angeles"),
    ("pdt", "America/Los_Angeles"),
    ("mt", "America/Denver"),
    ("mst", "America/Denver"),
    ("mdt", "America/Denver"),
    ("ct", "America/Chicago"),
    ("cst", "America/Chicago"),
    ("cdt", "America/Chicago"),
    ("et", "America/New_York"),
    ("est", "America/New_York"),
    ("edt", "America/New_York"),
    ("akst", "America/Anchorage"),
    ("hst", "Pacific/Honolulu"),
    ("brt", "America/Sao_Paulo"),
    ("art", "America/Argentina/Buenos_Aires"),
    ("bst", "Europe/London"),
    ("wet", "Europe/Lisbon"),
    ("cet", "Europe/Paris"),
    ("cest", "Europe/Paris"),
    ("eet", "Europe/Athens"),
    ("eest", "Europe/Athens"),
    ("msk", "Europe/Moscow"),
    ("ist", "Asia/Kolkata"),
    ("sgt", "Asia/Singapore"),
    ("hkt", "Asia/Hong_Kong"),
    ("cst8", "Asia/Shanghai"),
    ("jst", "Asia/Tokyo"),
    ("kst", "Asia/Seoul"),
    ("aest", "Australia/Sydney"),
    ("nzst", "Pacific/Auckland"),
];

pub fn zone_abbreviation(name: &str) -> Option<(String, &'static str)> {
    let key = name.trim().to_lowercase();
    ZONES.iter().find(|(abbr, _)| *abbr == key).map(|(_, zone)| (name.trim().to_uppercase(), *zone))
}

fn split_args(text: &str) -> Vec<String> {
    let (mut parts, mut depth, mut current) = (Vec::new(), 0, String::new());
    for c in text.chars() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => {
                parts.push(std::mem::take(&mut current));
                continue;
            }
            _ => {}
        }
        current.push(c);
    }
    parts.push(current);
    parts.into_iter().map(|p| p.trim().to_owned()).filter(|p| !p.is_empty()).collect()
}

static DURATION_PART: LazyLock<Regex> = LazyLock::new(|| re(r"(\d+(?:\.\d+)?)\s*(d|days?|h|hrs?|hours?|m|mins?|minutes?|s|secs?|seconds?)\b"));

pub fn parse_duration(text: &str) -> Option<f64> {
    let mut total = 0.0;
    let mut consumed = 0;
    for caps in DURATION_PART.captures_iter(text) {
        let value: f64 = caps[1].parse().ok()?;
        let unit = &caps[2];
        total += value
            * match unit.chars().next()? {
                'd' => 86400.0,
                'h' => 3600.0,
                'm' => 60.0,
                _ => 1.0,
            };
        consumed += caps[0].len();
    }
    let letters = text.chars().filter(|c| !c.is_whitespace() && *c != ',').count();
    (consumed > 0 && letters <= consumed).then_some(total)
}

static LIST: LazyLock<Regex> = LazyLock::new(|| re(r"^(sum|total|max|maximum|min|minimum|avg|average|mean|median|round)\s*\((.*)\)$"));

pub fn list_function(text: &str) -> Option<Pure> {
    let caps = LIST.captures(text)?;
    let args = split_args(&caps[2]);
    if args.is_empty() {
        return None;
    }
    let name = &caps[1];
    if name == "round" {
        let value = math::evaluate(args.first()?)?;
        let digits = args.get(1).map_or(Some(0.0), |d| math::evaluate(d))?.clamp(0.0, 12.0) as i32;
        let factor = 10f64.powi(digits);
        let rounded = (value * factor).round() / factor;
        return Some(pure(group(rounded, digits as usize), format!("{rounded:.*}", digits as usize), "Result"));
    }
    let numbers: Option<Vec<f64>> = args.iter().map(|a| math::evaluate(a)).collect();
    let (values, duration) = match numbers {
        Some(values) => (values, false),
        None => (args.iter().map(|a| parse_duration(a)).collect::<Option<Vec<f64>>>()?, true),
    };
    let mut sorted = values.clone();
    sorted.sort_by(|a, b| a.total_cmp(b));
    let result = match name {
        "sum" | "total" => values.iter().sum(),
        "max" | "maximum" => *sorted.last()?,
        "min" | "minimum" => *sorted.first()?,
        "median" => {
            let mid = sorted.len() / 2;
            if sorted.len() % 2 == 0 { (sorted[mid - 1] + sorted[mid]) / 2.0 } else { sorted[mid] }
        }
        _ => values.iter().sum::<f64>() / values.len() as f64,
    };
    if duration {
        let text = units::format_duration(result);
        return Some(pure(text.clone(), text, "Duration"));
    }
    Some(pure(format_number(result), plain_number(result), "Result"))
}

static NEAREST: LazyLock<Regex> = LazyLock::new(|| re(r"^(.+?)\s+(?:rounded\s+)?to\s+(?:the\s+)?nearest\s+(quarter|half|third|tenth|hundredth|thousandth|whole(?:\s+number)?|integer|ten|hundred|thousand|\d+(?:\.\d+)?)$"));

pub fn nearest(text: &str) -> Option<Pure> {
    let caps = NEAREST.captures(text)?;
    let value = math::evaluate(&caps[1])?;
    let step = match &caps[2] {
        "quarter" => 0.25,
        "half" => 0.5,
        "third" => 1.0 / 3.0,
        "tenth" => 0.1,
        "hundredth" => 0.01,
        "thousandth" => 0.001,
        "ten" => 10.0,
        "hundred" => 100.0,
        "thousand" => 1000.0,
        s if s.starts_with("whole") || s == "integer" => 1.0,
        s => s.parse().ok().filter(|v: &f64| *v > 0.0)?,
    };
    let rounded = to_precision((value / step).round() * step, 12);
    Some(pure(format_number(rounded), plain_number(rounded), "Result"))
}

pub fn fraction(value: f64) -> Option<(i64, i64)> {
    let negative = value < 0.0;
    let mut x = value.abs();
    let (mut h0, mut h1, mut k0, mut k1) = (0i64, 1i64, 1i64, 0i64);
    for _ in 0..32 {
        let a = x.floor() as i64;
        let (h2, k2) = (a.checked_mul(h1)?.checked_add(h0)?, a.checked_mul(k1)?.checked_add(k0)?);
        if k2 > 100_000 {
            break;
        }
        (h0, h1, k0, k1) = (h1, h2, k1, k2);
        if (h1 as f64 / k1 as f64 - value.abs()).abs() < 1e-9 {
            break;
        }
        let rest = x - a as f64;
        if rest < 1e-12 {
            break;
        }
        x = 1.0 / rest;
    }
    (k1 > 0).then_some((if negative { -h1 } else { h1 }, k1))
}

static FRACTION: LazyLock<Regex> = LazyLock::new(|| re(r"^(.+?)\s+(?:as|in|to)\s+(?:an?\s+)?(improper|mixed|fraction)(?:\s+fraction)?$"));

pub fn as_fraction(text: &str) -> Option<Pure> {
    let caps = FRACTION.captures(text)?;
    let value = math::evaluate(&caps[1])?;
    let (n, d) = fraction(value)?;
    if &caps[2] == "mixed" && d != 1 && n.abs() > d {
        let whole = n / d;
        let text = format!("{whole} {}/{d}", (n % d).abs());
        return Some(pure(text.clone(), text, "Mixed Fraction"));
    }
    let text = if d == 1 { n.to_string() } else { format!("{n}/{d}") };
    Some(pure(text.clone(), text, if &caps[2] == "improper" { "Improper Fraction" } else { "Fraction" }))
}

pub fn to_base(mut value: u128, base: u32) -> String {
    if value == 0 {
        return "0".into();
    }
    let digits = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut out = Vec::new();
    while value > 0 {
        out.push(digits[(value % base as u128) as usize]);
        value /= base as u128;
    }
    out.reverse();
    String::from_utf8(out).unwrap_or_default()
}

static BASE: LazyLock<Regex> = LazyLock::new(|| re(r"^(-?(?:0x[0-9a-f]+|0b[01]+|0o[0-7]+|[0-9a-z]+))(?:\s+(?:in\s+)?base\s*(\d{1,2}))?\s+(?:in|to|as)\s+(decimal|hex|hexadecimal|binary|octal|base\s*\d{1,2})$"));

pub fn base(text: &str) -> Option<Pure> {
    let caps = BASE.captures(text)?;
    let raw = caps[1].trim_start_matches('-');
    let negative = caps[1].starts_with('-');
    let (digits, from) = if let Some(rest) = raw.strip_prefix("0x") {
        (rest, 16)
    } else if let Some(rest) = raw.strip_prefix("0b") {
        (rest, 2)
    } else if let Some(rest) = raw.strip_prefix("0o") {
        (rest, 8)
    } else {
        (raw, caps.get(2).map_or(Some(10), |b| b.as_str().parse().ok())?)
    };
    let to: u32 = match &caps[3] {
        "decimal" => 10,
        "hex" | "hexadecimal" => 16,
        "binary" => 2,
        "octal" => 8,
        other => other.trim_start_matches("base").trim().parse().ok()?,
    };
    if !(2..=36).contains(&from) || !(2..=36).contains(&to) || (from == 10 && to == 10) {
        return None;
    }
    let value = u128::from_str_radix(digits, from).ok()?;
    let out = format!("{}{}", if negative { "-" } else { "" }, to_base(value, to));
    Some(pure(out.clone(), out, format!("Base {to}")))
}

static AMOUNT: LazyLock<Regex> = LazyLock::new(|| re(r"^(r\$|us\$|[$€£¥₹])?\s*(\d[\d,]*(?:\.\d+)?)\s*(usd|eur|gbp|brl|jpy|inr|cad|aud|r\$|[$€£¥₹])?$"));

fn amount(text: &str) -> Option<(f64, String)> {
    let caps = AMOUNT.captures(text.trim())?;
    let value: f64 = caps[2].replace(',', "").parse().ok()?;
    let symbol = caps.get(1).or(caps.get(3)).map_or("", |m| m.as_str());
    let symbol = match symbol {
        "usd" => "$",
        "eur" => "€",
        "gbp" => "£",
        "brl" | "r$" => "R$",
        "jpy" => "¥",
        "inr" => "₹",
        "cad" => "CA$",
        "aud" => "A$",
        "us$" => "$",
        other => other,
    };
    Some((value, symbol.to_owned()))
}

fn money(value: f64, symbol: &str) -> String {
    let body = group(value.abs(), 2);
    let body = if body.contains('.') {
        let decimals = body.split('.').nth(1).map_or(0, str::len);
        format!("{body}{}", "0".repeat(2 - decimals.min(2)))
    } else {
        format!("{body}.00")
    };
    format!("{}{symbol}{body}", if value < 0.0 { "-" } else { "" })
}

static PERCENT: LazyLock<Regex> = LazyLock::new(|| re(r"^(\d+(?:\.\d+)?)\s*%\s*(tip|gorjeta|tax|imposto|vat|iva|discount|desconto|off|increase|decrease|more|less)\s+(on|in|of|from|than|sobre|em|de)\s+(.+?)$"));
static OFF: LazyLock<Regex> = LazyLock::new(|| re(r"^(\d+(?:\.\d+)?)\s*%\s*off\s+(.+?)$"));
static SPLIT: LazyLock<Regex> = LazyLock::new(|| re(r"^(.+?)\s+(?:split|divided)\s+(?:by\s+|in\s+)?(\d+)(?:\s+ways|\s+people)?$"));

fn money_value(text: &str) -> Option<(f64, String, &'static str)> {
    if let Some(caps) = PERCENT.captures(text) {
        let rate: f64 = caps[1].parse::<f64>().ok()? / 100.0;
        let (base, symbol) = amount(&caps[4])?;
        let inside = matches!(&caps[3], "in" | "em");
        return Some(match &caps[2] {
            "tip" | "gorjeta" => (base * (1.0 + rate), symbol, "With Tip"),
            "tax" | "imposto" | "vat" | "iva" if inside => (base - base / (1.0 + rate), symbol, "Tax Included"),
            "tax" | "imposto" | "vat" | "iva" => (base * (1.0 + rate), symbol, "With Tax"),
            "increase" | "more" => (base * (1.0 + rate), symbol, "Result"),
            _ => (base * (1.0 - rate), symbol, "After Discount"),
        });
    }
    if let Some(caps) = OFF.captures(text) {
        let rate: f64 = caps[1].parse::<f64>().ok()? / 100.0;
        let (base, symbol) = amount(&caps[2])?;
        return Some((base * (1.0 - rate), symbol, "After Discount"));
    }
    let (value, symbol) = amount(text)?;
    Some((value, symbol, "Result"))
}

pub fn money_query(text: &str) -> Option<Pure> {
    if let Some(caps) = SPLIT.captures(text) {
        let ways: f64 = caps[2].parse().ok().filter(|w: &f64| *w > 0.0)?;
        let (total, symbol, _) = money_value(caps[1].trim())?;
        let each = total / ways;
        return Some(pure(money(each, &symbol), format!("{each:.2}"), "Per Person"));
    }
    if !PERCENT.is_match(text) && !OFF.is_match(text) {
        return None;
    }
    let (value, symbol, label) = money_value(text)?;
    Some(pure(if symbol.is_empty() { format_number(to_precision(value, 12)) } else { money(value, &symbol) }, format!("{value:.2}"), label))
}

pub struct Constant {
    pub names: &'static [&'static str],
    pub value: f64,
    pub unit: &'static str,
    pub label: &'static str,
}

pub const CONSTANTS: [Constant; 12] = [
    Constant { names: &["speed of light", "c", "velocidade da luz"], value: 299_792_458.0, unit: "m/s", label: "Speed of Light" },
    Constant { names: &["speed of sound", "velocidade do som"], value: 343.0, unit: "m/s", label: "Speed of Sound" },
    Constant { names: &["standard gravity", "gravity", "g0", "gravidade"], value: 9.806_65, unit: "m/s²", label: "Standard Gravity" },
    Constant { names: &["gravitational constant", "big g", "constante gravitacional"], value: 6.674_30e-11, unit: "m³/(kg·s²)", label: "Gravitational Constant" },
    Constant { names: &["planck constant", "planck's constant", "planck", "constante de planck"], value: 6.626_070_15e-34, unit: "J·s", label: "Planck Constant" },
    Constant { names: &["boltzmann constant", "boltzmann"], value: 1.380_649e-23, unit: "J/K", label: "Boltzmann Constant" },
    Constant { names: &["avogadro constant", "avogadro's number", "avogadro number", "avogadro"], value: 6.022_140_76e23, unit: "1/mol", label: "Avogadro Constant" },
    Constant { names: &["elementary charge", "electron charge"], value: 1.602_176_634e-19, unit: "C", label: "Elementary Charge" },
    Constant { names: &["electron mass", "mass of an electron", "massa do eletron"], value: 9.109_383_7015e-31, unit: "kg", label: "Electron Mass" },
    Constant { names: &["proton mass", "mass of a proton", "massa do proton"], value: 1.672_621_923_69e-27, unit: "kg", label: "Proton Mass" },
    Constant { names: &["gas constant", "ideal gas constant"], value: 8.314_462_618, unit: "J/(mol·K)", label: "Gas Constant" },
    Constant { names: &["golden ratio", "phi", "razao aurea"], value: 1.618_033_988_749_895, unit: "", label: "Golden Ratio" },
];

fn scientific(value: f64) -> String {
    if value != 0.0 && (value.abs() < 1e-4 || value.abs() >= 1e15) {
        let exponent = value.abs().log10().floor() as i32;
        let mantissa = to_precision(value / 10f64.powi(exponent), 10);
        return format!("{} × 10^{exponent}", plain_number(mantissa));
    }
    units::format_value(value)
}

pub fn constant(text: &str) -> Option<Pure> {
    let (name, target) = match text.rsplit_once(" in ").or_else(|| text.rsplit_once(" to ")).or_else(|| text.rsplit_once(" em ")) {
        Some((name, target)) if CONSTANTS.iter().any(|c| c.names.contains(&name.trim())) => (name.trim(), Some(target.trim())),
        _ => (text.trim(), None),
    };
    let found = CONSTANTS.iter().find(|c| c.names.contains(&name))?;
    if name == "c" && target.is_none() {
        return None;
    }
    let name_end = name.len();
    let mut dim = vec![(0, name_end)];
    match target {
        Some(target) => {
            let query = units::parse(&format!("{} {} in {target}", found.value, found.unit))?;
            dim = vec![(0, name_end + 3)];
            let display = format!("{} {}", units::format_value(query.value), query.to.id);
            Some(Pure { dim, unit_like: true, ..pure(display, plain_number(to_precision(query.value, 12)), "Result") })
        }
        None => {
            let display = format!("{} {}", scientific(found.value), found.unit).trim().to_owned();
            Some(Pure { dim, ..pure(display, found.value.to_string(), found.label) })
        }
    }
}

const ELEMENTS: [(&str, f64); 118] = [
    ("H", 1.008), ("He", 4.0026), ("Li", 6.94), ("Be", 9.0122), ("B", 10.81), ("C", 12.011), ("N", 14.007), ("O", 15.999), ("F", 18.998), ("Ne", 20.180),
    ("Na", 22.990), ("Mg", 24.305), ("Al", 26.982), ("Si", 28.085), ("P", 30.974), ("S", 32.06), ("Cl", 35.45), ("Ar", 39.95), ("K", 39.098), ("Ca", 40.078),
    ("Sc", 44.956), ("Ti", 47.867), ("V", 50.942), ("Cr", 51.996), ("Mn", 54.938), ("Fe", 55.845), ("Co", 58.933), ("Ni", 58.693), ("Cu", 63.546), ("Zn", 65.38),
    ("Ga", 69.723), ("Ge", 72.630), ("As", 74.922), ("Se", 78.971), ("Br", 79.904), ("Kr", 83.798), ("Rb", 85.468), ("Sr", 87.62), ("Y", 88.906), ("Zr", 91.224),
    ("Nb", 92.906), ("Mo", 95.95), ("Tc", 98.0), ("Ru", 101.07), ("Rh", 102.91), ("Pd", 106.42), ("Ag", 107.87), ("Cd", 112.41), ("In", 114.82), ("Sn", 118.71),
    ("Sb", 121.76), ("Te", 127.60), ("I", 126.90), ("Xe", 131.29), ("Cs", 132.91), ("Ba", 137.33), ("La", 138.91), ("Ce", 140.12), ("Pr", 140.91), ("Nd", 144.24),
    ("Pm", 145.0), ("Sm", 150.36), ("Eu", 151.96), ("Gd", 157.25), ("Tb", 158.93), ("Dy", 162.50), ("Ho", 164.93), ("Er", 167.26), ("Tm", 168.93), ("Yb", 173.05),
    ("Lu", 174.97), ("Hf", 178.49), ("Ta", 180.95), ("W", 183.84), ("Re", 186.21), ("Os", 190.23), ("Ir", 192.22), ("Pt", 195.08), ("Au", 196.97), ("Hg", 200.59),
    ("Tl", 204.38), ("Pb", 207.2), ("Bi", 208.98), ("Po", 209.0), ("At", 210.0), ("Rn", 222.0), ("Fr", 223.0), ("Ra", 226.0), ("Ac", 227.0), ("Th", 232.04),
    ("Pa", 231.04), ("U", 238.03), ("Np", 237.0), ("Pu", 244.0), ("Am", 243.0), ("Cm", 247.0), ("Bk", 247.0), ("Cf", 251.0), ("Es", 252.0), ("Fm", 257.0),
    ("Md", 258.0), ("No", 259.0), ("Lr", 266.0), ("Rf", 267.0), ("Db", 268.0), ("Sg", 269.0), ("Bh", 270.0), ("Hs", 277.0), ("Mt", 278.0), ("Ds", 281.0),
    ("Rg", 282.0), ("Cn", 285.0), ("Nh", 286.0), ("Fl", 289.0), ("Mc", 290.0), ("Lv", 293.0), ("Ts", 294.0), ("Og", 294.0),
];

pub fn molar_mass(formula: &str) -> Option<f64> {
    fn group_mass(chars: &[char], i: &mut usize, depth: usize) -> Option<f64> {
        let mut total = 0.0;
        while *i < chars.len() {
            let c = chars[*i];
            let part = if c == '(' || c == '[' {
                *i += 1;
                let inner = group_mass(chars, i, depth + 1)?;
                if *i >= chars.len() || !matches!(chars[*i], ')' | ']') {
                    return None;
                }
                *i += 1;
                inner
            } else if c == ')' || c == ']' {
                return (depth > 0).then_some(total);
            } else if c.is_ascii_uppercase() {
                let mut symbol = c.to_string();
                *i += 1;
                if *i < chars.len() && chars[*i].is_ascii_lowercase() {
                    symbol.push(chars[*i]);
                    *i += 1;
                }
                ELEMENTS.iter().find(|(s, _)| *s == symbol)?.1
            } else {
                return None;
            };
            let start = *i;
            while *i < chars.len() && chars[*i].is_ascii_digit() {
                *i += 1;
            }
            let count: f64 = if start == *i { 1.0 } else { chars[start..*i].iter().collect::<String>().parse().ok()? };
            total += part * count;
        }
        (depth == 0).then_some(total)
    }
    let chars: Vec<char> = formula.trim().chars().collect();
    let mut i = 0;
    group_mass(&chars, &mut i, 0).filter(|m| *m > 0.0)
}

static MOLAR: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^(?:molar mass|molecular weight|molecular mass|massa molar|peso molecular)\s+(?:of\s+|de\s+)?([A-Za-z0-9()\[\]]+)$").expect("molar pattern"));

pub fn chemistry(original: &str) -> Option<Pure> {
    let caps = MOLAR.captures(original.trim())?;
    let mass = molar_mass(&caps[1])?;
    let prefix = caps.get(1)?.start();
    Some(Pure { dim: vec![(0, prefix)], ..pure(format!("{} g/mol", group(mass, 3)), format!("{mass:.3}"), "Molar Mass") })
}

static PACE: LazyLock<Regex> = LazyLock::new(|| re(r"^(\d{1,2}):(\d{2})\s*(?:min)?\s*/\s*(km|mi|mile)\s+(?:in|to)\s+(km/h|kmh|kph|mph|min/km|min/mi)$"));
static SPEED_TO_PACE: LazyLock<Regex> = LazyLock::new(|| re(r"^(\d+(?:\.\d+)?)\s*(km/h|kmh|kph|mph)\s+(?:in|to|as)\s+(?:pace|min/km|min/mi|minutes per (?:km|mile))$"));

fn pace_text(seconds_per_unit: f64, unit: &str) -> String {
    let total = seconds_per_unit.round() as i64;
    format!("{}:{:02} min/{unit}", total / 60, total % 60)
}

pub fn pace(text: &str) -> Option<Pure> {
    const MILE: f64 = 1.609344;
    if let Some(caps) = PACE.captures(text) {
        let seconds = caps[1].parse::<f64>().ok()? * 60.0 + caps[2].parse::<f64>().ok()?;
        if seconds <= 0.0 {
            return None;
        }
        let per_km = if &caps[3] == "km" { seconds } else { seconds / MILE };
        let target = &caps[4];
        let display = match target {
            "mph" => format!("{} mph", group(3600.0 / per_km / MILE, 2)),
            "min/km" => pace_text(per_km, "km"),
            "min/mi" => pace_text(per_km * MILE, "mi"),
            _ => format!("{} km/h", group(3600.0 / per_km, 2)),
        };
        return Some(Pure { unit_like: true, ..pure(display.clone(), display, "Pace") });
    }
    let caps = SPEED_TO_PACE.captures(text)?;
    let speed: f64 = caps[1].parse().ok().filter(|s: &f64| *s > 0.0)?;
    let kmh = if &caps[2] == "mph" { speed * MILE } else { speed };
    let display = if text.contains("mi") && !text.contains("min/km") { pace_text(3600.0 / kmh * MILE, "mi") } else { pace_text(3600.0 / kmh, "km") };
    Some(Pure { unit_like: true, ..pure(display.clone(), display, "Pace") })
}

static TRANSFER: LazyLock<Regex> = LazyLock::new(|| re(r"^(?:transfer|download|upload|copy|baixar)\s+(\d+(?:\.\d+)?)\s*(b|kb|mb|gb|tb|pb|kib|mib|gib|tib)\s+(?:at|@|over|with|a)\s+(\d+(?:\.\d+)?)\s*(bps|kbps|mbps|gbps|kb/s|mb/s|gb/s|kbit/s|mbit/s|gbit/s)$"));

pub fn transfer(text: &str) -> Option<Pure> {
    let caps = TRANSFER.captures(text)?;
    let size: f64 = caps[1].parse().ok()?;
    let bytes = size
        * match &caps[2] {
            "kb" => 1e3,
            "mb" => 1e6,
            "gb" => 1e9,
            "tb" => 1e12,
            "pb" => 1e15,
            "kib" => 1024.0,
            "mib" => 1024f64.powi(2),
            "gib" => 1024f64.powi(3),
            "tib" => 1024f64.powi(4),
            _ => 1.0,
        };
    let rate: f64 = caps[3].parse().ok()?;
    let bits_per_second = rate
        * match &caps[4] {
            "kbps" | "kbit/s" => 1e3,
            "mbps" | "mbit/s" => 1e6,
            "gbps" | "gbit/s" => 1e9,
            "kb/s" => 8e3,
            "mb/s" => 8e6,
            "gb/s" => 8e9,
            _ => 1.0,
        };
    if bits_per_second <= 0.0 {
        return None;
    }
    let seconds = bytes * 8.0 / bits_per_second;
    let text = units::format_duration(seconds.round());
    Some(pure(text.clone(), format!("{seconds:.0}"), "Transfer Time"))
}

const NOTES: [&str; 12] = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
static NOTE: LazyLock<Regex> = LazyLock::new(|| re(r"^([a-g])(#|b|♯|♭)?(-?\d)\s+(?:to|in|as)\s+(?:hz|hertz|frequency)$"));
static HZ: LazyLock<Regex> = LazyLock::new(|| re(r"^(\d+(?:\.\d+)?)\s*(?:hz|hertz)\s+(?:to|in|as)\s+(?:note|pitch)$"));

pub fn note(text: &str) -> Option<Pure> {
    if let Some(caps) = NOTE.captures(text) {
        let base = NOTES.iter().position(|n| n.eq_ignore_ascii_case(&caps[1]))? as i32;
        let shift = match caps.get(2).map(|m| m.as_str()) {
            Some("#" | "♯") => 1,
            Some("b" | "♭") => -1,
            _ => 0,
        };
        let octave: i32 = caps[3].parse().ok()?;
        let midi = (octave + 1) * 12 + base + shift;
        let hz = 440.0 * 2f64.powf((midi - 69) as f64 / 12.0);
        return Some(pure(format!("{} Hz", group(hz, 2)), format!("{hz:.2}"), "Frequency"));
    }
    let caps = HZ.captures(text)?;
    let hz: f64 = caps[1].parse().ok().filter(|h: &f64| *h > 0.0)?;
    let midi = 69.0 + 12.0 * (hz / 440.0).log2();
    let nearest = midi.round();
    let cents = ((midi - nearest) * 100.0).round() as i32;
    let n = nearest as i32;
    let name = format!("{}{}", NOTES[n.rem_euclid(12) as usize], n.div_euclid(12) - 1);
    let detail = if cents == 0 { String::new() } else { format!(" {}{cents} cents", if cents > 0 { "+" } else { "" }) };
    Some(pure(format!("{name}{detail}"), name, "Note"))
}

static TEXT_TAKE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"(?i)^(first|last)\s+(\d+)\s+(characters?|chars?|letters?|words?)\s+(?:of|in|from)\s+["“](.*)["”]$"#).expect("text pattern"));
static TEXT_FN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"(?i)^(length|character count|char count|characters|word count|words|uppercase|upper case|lowercase|lower case|reverse|reversed|title case|capitalize)\s+(?:of\s+|in\s+)?["“](.*)["”]$"#).expect("text pattern"));

pub fn text_function(original: &str) -> Option<Pure> {
    let text = original.trim();
    if let Some(caps) = TEXT_TAKE.captures(text) {
        let count: usize = caps[2].parse().ok()?;
        let value = &caps[4];
        let words = caps[3].to_lowercase().starts_with('w');
        let first = caps[1].eq_ignore_ascii_case("first");
        let out = if words {
            let list: Vec<&str> = value.split_whitespace().collect();
            let slice = if first { &list[..count.min(list.len())] } else { &list[list.len().saturating_sub(count)..] };
            slice.join(" ")
        } else {
            let chars: Vec<char> = value.chars().collect();
            let slice = if first { &chars[..count.min(chars.len())] } else { &chars[chars.len().saturating_sub(count)..] };
            slice.iter().collect()
        };
        return Some(pure(out.clone(), out, "Text"));
    }
    let caps = TEXT_FN.captures(text)?;
    let value = &caps[2];
    let (out, label) = match caps[1].to_lowercase().as_str() {
        "length" | "character count" | "char count" | "characters" => (value.chars().count().to_string(), "Characters"),
        "word count" | "words" => (value.split_whitespace().count().to_string(), "Words"),
        "uppercase" | "upper case" => (value.to_uppercase(), "Text"),
        "lowercase" | "lower case" => (value.to_lowercase(), "Text"),
        "reverse" | "reversed" => (value.chars().rev().collect(), "Text"),
        _ => (value.split_whitespace().map(|w| w.chars().next().map(|c| c.to_uppercase().collect::<String>() + &w[c.len_utf8()..].to_lowercase()).unwrap_or_default()).collect::<Vec<_>>().join(" "), "Text"),
    };
    Some(pure(out.clone(), out, label))
}

pub fn haversine_km(a: (f64, f64), b: (f64, f64)) -> f64 {
    let (lat1, lon1, lat2, lon2) = (a.0.to_radians(), a.1.to_radians(), b.0.to_radians(), b.1.to_radians());
    let h = ((lat2 - lat1) / 2.0).sin().powi(2) + lat1.cos() * lat2.cos() * ((lon2 - lon1) / 2.0).sin().powi(2);
    2.0 * 6371.0088 * h.sqrt().asin()
}

pub fn dim_words(text: &str, words: &[&str]) -> Vec<(usize, usize)> {
    let lower = text.to_lowercase();
    let mut ranges = Vec::new();
    let mut at = 0;
    for token in lower.split(' ') {
        let end = at + token.len();
        if words.contains(&token) {
            ranges.push((at, end));
        }
        at = end + 1;
    }
    ranges
}

#[cfg(test)]
mod tests {
    use super::*;

    fn display(p: Option<Pure>) -> String {
        p.map(|p| p.display).unwrap_or_else(|| "<none>".into())
    }

    #[test]
    fn iso_weeks() {
        let start = iso_week_start(2026, 44);
        assert_eq!(civil(start), (2026, 10, 26));
        assert_eq!(civil(start + 6), (2026, 11, 1));
        assert_eq!(iso_week(days_of(2026, 11, 13)), (2026, 46));
        assert_eq!(iso_week(days_of(2021, 1, 1)), (2020, 53));
        let range = |a: i64, b: i64| format!("{:?}-{:?}", civil(a), civil(b));
        assert_eq!(display(week("week 44 2026", days_of(2026, 11, 13), &range)), "(2026, 10, 26)-(2026, 11, 1)");
        assert_eq!(display(week("week 54 2026", days_of(2026, 11, 13), &range)), "<none>");
    }

    #[test]
    fn time_left_and_since() {
        let now = days_of(2026, 11, 13) as f64 * DAY_MS + 15.0 * 3_600_000.0;
        assert_eq!(display(time_left("days left in this month", now)), "17 days");
        assert_eq!(display(time_left("hours left in today", now)), "9 hours");
        let parse = |_: &str, _: i64| None;
        assert_eq!(display(since("days since march", now, &parse)), "257 days");
        assert_eq!(display(since("days until december", now, &parse)), "18 days");
        assert_eq!(zone_abbreviation("PT").unwrap().1, "America/Los_Angeles");
    }

    #[test]
    fn functions_and_rounding() {
        assert_eq!(display(list_function("round(23.456, 1)")), "23.5");
        assert_eq!(display(list_function("sum(1, 2, 3)")), "6");
        assert_eq!(display(list_function("max(2 hours 30 min, 200 min)")), "3 hours 20 minutes");
        assert_eq!(display(list_function("avg(2, 4)")), "3");
        assert_eq!(display(nearest("3.3 to nearest quarter")), "3.25");
        assert_eq!(display(nearest("1234 to the nearest hundred")), "1,200");
        assert_eq!(display(as_fraction("1.75 as improper")), "7/4");
        assert_eq!(display(as_fraction("1.75 as mixed fraction")), "1 3/4");
        assert_eq!(display(base("255 in base 36")), "73");
        assert_eq!(display(base("ff base 16 in decimal")), "255");
        assert_eq!(display(base("0xff to binary")), "11111111");
    }

    #[test]
    fn money_queries() {
        assert_eq!(display(money_query("15% tip on $80")), "$92.00");
        assert_eq!(display(money_query("20% vat in 120")), "20");
        assert_eq!(display(money_query("20% vat on 100")), "120");
        assert_eq!(display(money_query("25% off $60 split 3")), "$15.00");
        assert_eq!(display(money_query("15% decrease from 813")), "691.05");
        assert_eq!(display(money_query("hello split 3")), "<none>");
    }

    #[test]
    fn science_and_text() {
        assert_eq!(display(constant("speed of light in km/s")), "299,792.458 km/s");
        assert_eq!(display(constant("speed of light")), "299,792,458 m/s");
        assert!(display(constant("planck constant")).starts_with("6.62607015 × 10^-34"));
        assert_eq!(display(chemistry("molar mass of C6H12O6")), "180.156 g/mol");
        assert_eq!(display(chemistry("molar mass of Ca(OH)2")), "74.092 g/mol");
        assert_eq!(display(chemistry("molar mass of Xx2")), "<none>");
        assert_eq!(display(pace("5:00 min/km in km/h")), "12 km/h");
        assert_eq!(display(pace("12 km/h to min/km")), "5:00 min/km");
        assert_eq!(display(transfer("transfer 1 tb at 100 mbps")), "22 hours 13 minutes");
        assert_eq!(display(note("c4 to hz")), "261.63 Hz");
        assert_eq!(display(note("a4 to hz")), "440 Hz");
        assert_eq!(display(note("440 hz to note")), "A4");
        assert_eq!(display(text_function(r#"first 3 characters of "hello""#)), "hel");
        assert_eq!(display(text_function(r#"word count "one two three""#)), "3");
        assert!((haversine_km((45.92375, 6.86933), (48.8566, 2.3522)) - 471.7).abs() < 2.0);
    }
}
