use super::history::{civil_from_days, days_from_civil};
use super::math::unaccent;
use crate::icu;
use regex::Regex;
use std::collections::HashMap;
use std::sync::LazyLock;

const DAY: f64 = 86_400_000.0;

#[derive(Clone, PartialEq, Debug)]
pub struct Place {
    pub name: String,
    pub zone: String,
}

pub static LANGUAGES: LazyLock<Vec<String>> = LazyLock::new(icu::languages);

pub fn locale() -> &'static str {
    LANGUAGES.first().map_or("en-US", String::as_str)
}

pub fn build_places(languages: &[String]) -> HashMap<String, Place> {
    let mut index = HashMap::new();
    for zone in icu::zones(None) {
        let city = zone.rsplit('/').next().unwrap_or(&zone).replace('_', " ");
        index.insert(city.to_lowercase(), Place { name: city, zone: zone.clone() });
    }
    for a in b'A'..=b'Z' {
        for b in b'A'..=b'Z' {
            let code = String::from_utf8(vec![a, b]).unwrap_or_default();
            if icu::country_name(&code, "en").is_none() {
                continue;
            }
            let Some(zone) = icu::zones(Some(&code)).into_iter().next() else { continue };
            for display in std::iter::once("en").chain(languages.iter().map(String::as_str)) {
                if let Some(local) = icu::country_name(&code, display) {
                    index.insert(local.to_lowercase(), Place { name: local, zone: zone.clone() });
                }
            }
        }
    }
    index.insert("utc".into(), Place { name: "UTC".into(), zone: "UTC".into() });
    index.insert("gmt".into(), Place { name: "GMT".into(), zone: "UTC".into() });
    index
}

pub static PLACES: LazyLock<HashMap<String, Place>> = LazyLock::new(|| build_places(&LANGUAGES));

const ALIASES: [(&str, &str); 6] = [("ldn", "london"), ("nyc", "new york"), ("sf", "los angeles"), ("la", "los angeles"), ("sp", "sao paulo"), ("hk", "hong kong")];

pub fn lookup(name: &str, index: &HashMap<String, Place>) -> Option<Place> {
    let key = unaccent(&name.trim().to_lowercase());
    let key = ALIASES.iter().find(|(a, _)| *a == key).map_or(key.as_str(), |(_, v)| v).to_owned();
    index.get(&key).cloned()
}


#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Clock {
    pub hour: i64,
    pub minute: i64,
}

static CLOCK: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(\d{1,2})(?::(\d{2}))?\s*(am|pm|h)?$").unwrap());
static CLOCK_SPLIT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)\s+(?:in|em|para)\s+").unwrap());
static CLOCK_LEFT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(\S+)\s+(.+)$").unwrap());

pub fn parse_clock(text: &str) -> Option<Clock> {
    let caps = CLOCK.captures(&text.to_lowercase()).map(|c| (c[1].to_owned(), c.get(2).map(|m| m.as_str().to_owned()), c.get(3).map_or(String::new(), |m| m.as_str().to_owned())))?;
    if caps.1.is_none() && caps.2.is_empty() {
        return None;
    }
    let mut hour: i64 = caps.0.parse().ok()?;
    let minute: i64 = caps.1.map_or(Some(0), |m| m.parse().ok())?;
    if caps.2 == "pm" && hour < 12 {
        hour += 12;
    }
    if caps.2 == "am" && hour == 12 {
        hour = 0;
    }
    (hour <= 23 && minute <= 59).then_some(Clock { hour, minute })
}

pub fn parse_clock_query(text: &str) -> Option<(Clock, String, String)> {
    let parts: Vec<&str> = CLOCK_SPLIT.split(text.trim()).collect();
    if parts.len() != 2 {
        return None;
    }
    let left = CLOCK_LEFT.captures(parts[0].trim())?;
    Some((parse_clock(&left[1])?, left[2].to_owned(), parts[1].to_owned()))
}

pub fn zoned_day(ms: f64, zone: &str) -> i64 {
    ((ms + icu::offset_minutes(ms, zone) as f64 * 60_000.0) / DAY).floor() as i64
}

pub fn instant_for(day: i64, time: Clock, zone: &str) -> f64 {
    let guess = day as f64 * DAY + (time.hour * 3_600_000 + time.minute * 60_000) as f64;
    guess - icu::offset_minutes(guess, zone) as f64 * 60_000.0
}

fn relative_unit(name: &str) -> Option<f64> {
    Some(match name {
        "second" | "seconds" | "segundo" | "segundos" => 1000.0,
        "minute" | "minutes" | "minuto" | "minutos" => 60_000.0,
        "hour" | "hours" | "hora" | "horas" => 3_600_000.0,
        "day" | "days" | "dia" | "dias" => 86_400_000.0,
        "week" | "weeks" | "semana" | "semanas" => 604_800_000.0,
        "month" | "months" | "mes" | "meses" => 2_629_746_000.0,
        "year" | "years" | "ano" | "anos" => 31_556_952_000.0,
        _ => return None,
    })
}

static FUTURE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(?:in|em)\s+(\d+(?:[.,]\d+)?)\s+([a-z]+)$").unwrap());
static PAST: LazyLock<[Regex; 2]> = LazyLock::new(|| [r"^(\d+(?:[.,]\d+)?)\s+([a-z]+)\s+ago$", r"^ha\s+(\d+(?:[.,]\d+)?)\s+([a-z]+)$"].map(|p| Regex::new(p).unwrap()));

pub fn parse_relative(text: &str) -> Option<f64> {
    let normalized = unaccent(&text.trim().to_lowercase());
    let amount = |caps: &regex::Captures| -> Option<f64> { Some(caps[1].replace(',', ".").parse::<f64>().ok()? * relative_unit(&caps[2])?) };
    if let Some(caps) = FUTURE.captures(&normalized) {
        return amount(&caps);
    }
    let caps = PAST.iter().find_map(|r| r.captures(&normalized))?;
    amount(&caps).map(|delta| -delta)
}

static ISO: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(\d{4})-(\d{1,2})-(\d{1,2})$").unwrap());
static DMY: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(\d{1,2})[/.-](\d{1,2})(?:[/.-](\d{4}))?$").unwrap());
static UNTIL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^(?:days? (?:until|to)|dias ate|dias até)\s+(.+)$").unwrap());

pub fn parse_target_day(text: &str, today: i64) -> Option<i64> {
    let trimmed = text.trim();
    let num = |s: &str| s.parse::<i64>().ok();
    if let Some(c) = ISO.captures(trimmed) {
        return Some(days_from_civil(num(&c[1])?, num(&c[2])?, num(&c[3])?));
    }
    let c = DMY.captures(trimmed)?;
    let year = c.get(3).map_or(Some(civil_from_days(today).0), |y| num(y.as_str()))?;
    Some(days_from_civil(year, num(&c[2])?, num(&c[1])?))
}

pub fn parse_days_until(text: &str) -> Option<String> {
    UNTIL.captures(text.trim()).map(|c| c[1].to_owned())
}

static DIFF: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(?:time (?:difference|diff)|diferenca de horario)\s+(?:between|entre)?\s*(.+?)\s+(?:and|e)\s+(.+)$").unwrap());

pub fn parse_time_diff(text: &str) -> Option<(String, String)> {
    let normalized = unaccent(&text.trim().to_lowercase());
    DIFF.captures(&normalized).map(|c| (c[1].to_owned(), c[2].to_owned()))
}

pub fn format_diff(minutes: i32) -> String {
    let (hours, rest) = (minutes.abs() / 60, minutes.abs() % 60);
    format!("{}{hours} h{}", if minutes < 0 { "-" } else { "+" }, if rest > 0 { format!(" {rest} min") } else { String::new() })
}

pub fn updated_ago(updated: u64, now: u64) -> String {
    let minutes = (now.saturating_sub(updated) as f64 / 60_000.0).round() as u64;
    let plural = |n: u64, unit: &str| format!("{n} {unit}{}", if n == 1 { "" } else { "s" });
    if minutes < 1 {
        return "Updated just now".into();
    }
    if minutes < 60 {
        return format!("Updated {} ago", plural(minutes, "minute"));
    }
    let hours = (minutes as f64 / 60.0).round() as u64;
    if hours < 24 {
        return format!("Updated about {} ago", plural(hours, "hour"));
    }
    format!("Updated {} ago", plural((hours as f64 / 24.0).round() as u64, "day"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_countries_and_cities() {
        let index = build_places(&["en".into(), "pt".into()]);
        assert_eq!(lookup("china", &index).map(|p| p.zone).as_deref(), Some("Asia/Shanghai"));
        assert_eq!(lookup("Tokyo", &index).map(|p| p.zone).as_deref(), Some("Asia/Tokyo"));
        assert_eq!(lookup("brasil", &index).map(|p| p.name).as_deref(), Some("Brasil"));
        assert_eq!(lookup("nowhere", &index), None);
        assert_eq!(lookup("nyc", &index).map(|p| p.zone).as_deref(), Some("America/New_York"));
        assert_eq!(parse_clock_query("4:00 PT in UTC"), Some((Clock { hour: 4, minute: 0 }, "PT".into(), "UTC".into())));
        assert_eq!(parse_clock("4"), None);
    }

    #[test]
    fn parses_clocks_relative_dates_and_differences() {
        assert_eq!(parse_clock("5pm"), Some(Clock { hour: 17, minute: 0 }));
        assert_eq!(parse_clock("12:30am"), Some(Clock { hour: 0, minute: 30 }));
        assert_eq!(parse_clock("25h"), None);
        assert_eq!(parse_clock_query("5pm london in tokyo"), Some((Clock { hour: 17, minute: 0 }, "london".into(), "tokyo".into())));
        assert_eq!(parse_relative("in 2 days"), Some(2.0 * DAY));
        assert_eq!(parse_relative("há 3 horas"), Some(-3.0 * 3_600_000.0));
        assert_eq!(parse_relative("in 2 parsecs"), None);
        let today = days_from_civil(2026, 11, 4);
        assert_eq!(parse_target_day("25/12", today).map(|d| d - today), Some(51));
        assert_eq!(parse_target_day("2027-01-01", today).map(|d| d - today), Some(58));
        assert_eq!(parse_days_until("days until 25/12").as_deref(), Some("25/12"));
        assert_eq!(parse_time_diff("time difference between london and tokyo"), Some(("london".into(), "tokyo".into())));
        assert_eq!(format_diff(330), "+5 h 30 min");
        assert_eq!(format_diff(-180), "-3 h");
        let morning = instant_for(days_from_civil(2026, 10, 19), Clock { hour: 6, minute: 32 }, "Asia/Shanghai");
        assert_eq!(morning, 1_792_362_720_000.0);
    }

    #[test]
    fn says_how_long_ago_rates_were_updated() {
        assert_eq!(updated_ago(0, 20_000), "Updated just now");
        assert_eq!(updated_ago(0, 60_000), "Updated 1 minute ago");
        assert_eq!(updated_ago(0, 3 * 3_600_000), "Updated about 3 hours ago");
        assert_eq!(updated_ago(0, 2 * 86_400_000), "Updated 2 days ago");
    }
}
