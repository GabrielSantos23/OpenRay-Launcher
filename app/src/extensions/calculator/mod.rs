pub mod color;
pub mod extra;
pub mod places;
pub mod history;
pub mod math;
pub mod time;
pub mod units;

use super::command;
use crate::extension::{frontmost_app, paste_title, Accessory, Action, CardKind, Command, Empty, Extension, FlightCard, Forecast, Host, Icon, Inline, Item, Request, Section, Shortcut, Side, Tone, View, WeatherCard};
use crate::icu::{self, Style};
use egui::Key;
use history::{Calculation, Kind};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::cell::Cell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{LazyLock, Mutex};

const RATES_URL: &str = "https://open.er-api.com/v6/latest/USD";
const RATES_MAX_AGE_MS: u64 = 6 * 3_600_000;
const CRYPTO_URL: &str = "https://api.coingecko.com/api/v3/simple/price?ids=bitcoin,ethereum,solana,ripple,cardano,dogecoin,litecoin,tether&vs_currencies=usd";
const CRYPTO_MAX_AGE_MS: u64 = 10 * 60_000;
const CRYPTO_CODES: [(&str, &str); 8] = [("bitcoin", "BTC"), ("ethereum", "ETH"), ("solana", "SOL"), ("ripple", "XRP"), ("cardano", "ADA"), ("dogecoin", "DOGE"), ("litecoin", "LTC"), ("tether", "USDT")];

#[derive(Serialize, Deserialize, Clone, Default)]
struct Rates {
    rates: HashMap<String, f64>,
    updated: u64,
    fetched: u64,
}

#[derive(Default)]
struct RateState {
    fiat: Option<Rates>,
    crypto: Option<Rates>,
    loaded: bool,
    fiat_pending: bool,
    crypto_pending: bool,
}

static RATES: Mutex<RateState> = Mutex::new(RateState { fiat: None, crypto: None, loaded: false, fiat_pending: false, crypto_pending: false });
pub static RATES_VERSION: AtomicU64 = AtomicU64::new(0);

fn rates_path(name: &str) -> std::path::PathBuf {
    crate::data_dir().join(format!("{name}.json"))
}

fn fetch_fiat() -> Result<Rates, String> {
    let body: serde_json::Value = serde_json::from_slice(&crate::icons::fetch(RATES_URL)?).map_err(|e| e.to_string())?;
    if body["result"] != "success" {
        return Err(body["error-type"].as_str().unwrap_or("rates unavailable").to_owned());
    }
    let rates = serde_json::from_value(body["rates"].clone()).map_err(|e| e.to_string())?;
    Ok(Rates { rates, updated: body["time_last_update_unix"].as_u64().unwrap_or(0) * 1000, fetched: crate::store::now_ms() })
}

pub fn crypto_rates_from_prices(prices: &serde_json::Value) -> HashMap<String, f64> {
    CRYPTO_CODES.iter().filter_map(|(id, code)| prices[id]["usd"].as_f64().filter(|p| *p != 0.0).map(|usd| (code.to_string(), 1.0 / usd))).collect()
}

fn fetch_crypto() -> Result<Rates, String> {
    let prices: serde_json::Value = serde_json::from_slice(&crate::icons::fetch(CRYPTO_URL)?).map_err(|e| e.to_string())?;
    let rates = crypto_rates_from_prices(&prices);
    if rates.is_empty() {
        return Err("crypto rates unavailable".into());
    }
    let now = crate::store::now_ms();
    Ok(Rates { rates, updated: now, fetched: now })
}

fn refresh(ctx: &egui::Context, crypto: bool) {
    let ctx = ctx.clone();
    std::thread::spawn(move || {
        let (name, result) = if crypto { ("crypto-rates", fetch_crypto()) } else { ("rates", fetch_fiat()) };
        let mut state = RATES.lock().unwrap_or_else(|e| e.into_inner());
        match result {
            Ok(rates) => {
                let _ = crate::store::write_json(&rates_path(name), &rates);
                if crypto {
                    state.crypto = Some(rates);
                } else {
                    state.fiat = Some(rates);
                }
            }
            Err(e) => eprintln!("openray: {name} unavailable: {e}"),
        }
        if crypto {
            state.crypto_pending = false;
        } else {
            state.fiat_pending = false;
        }
        RATES_VERSION.fetch_add(1, Ordering::SeqCst);
        ctx.request_repaint();
    });
}

fn rates(ctx: &egui::Context) -> Option<Rates> {
    let mut state = RATES.lock().unwrap_or_else(|e| e.into_inner());
    if !state.loaded {
        state.loaded = true;
        state.fiat = std::fs::read(rates_path("rates")).ok().and_then(|b| serde_json::from_slice(&b).ok());
        state.crypto = std::fs::read(rates_path("crypto-rates")).ok().and_then(|b| serde_json::from_slice(&b).ok());
    }
    let now = crate::store::now_ms();
    if !state.fiat_pending && state.fiat.as_ref().is_none_or(|r| now.saturating_sub(r.fetched) >= RATES_MAX_AGE_MS) {
        state.fiat_pending = true;
        refresh(ctx, false);
    }
    if !state.crypto_pending && state.crypto.as_ref().is_none_or(|r| now.saturating_sub(r.fetched) >= CRYPTO_MAX_AGE_MS) {
        state.crypto_pending = true;
        refresh(ctx, true);
    }
    let mut merged = state.fiat.clone()?;
    merged.rates.extend(state.crypto.as_ref().map(|c| c.rates.clone()).unwrap_or_default());
    Some(merged)
}

pub struct Answer {
    pub kind: Kind,
    pub input: String,
    pub input_label: String,
    pub display: String,
    pub plain: String,
    pub label: String,
    pub note: Option<String>,
    pub dim: Option<Vec<(usize, usize)>>,
}

enum Computed {
    Color(color::ParsedColor),
    Answer(Answer),
    Weather { card: WeatherCard, open: String, copy: String },
    Flight { card: FlightCard, open: String, copy: String },
    Loading { section: &'static str, open_title: &'static str, open: String, fallback: Option<String> },
    PairLoading,
}

macro_rules! ready {
    ($lookup:expr) => {
        match $lookup {
            Some(result) => result.ok()?,
            None => return Some(Computed::PairLoading),
        }
    };
}

static CURRENCY_HINT: LazyLock<[Regex; 3]> = LazyLock::new(|| [r"(?i)[a-z$€£¥₹₩₽₺₿]", r"\d", r"(?i)\s(?:in|to|as|para|em|como|pra|pro|=)\s"].map(|p| Regex::new(p).unwrap()));
static TIME_AT: LazyLock<[Regex; 2]> = LazyLock::new(|| {
    [r"^(?:time|now|date|hora|horas|agora|data|que\s+horas)\s+(?:in|at|em|no|na|de)\s+(.+?)(?:\s*([+-])\s*(\d+(?:\.\d+)?)\s*(?:h|hrs?|hours?|horas?)?)?$", r"^(.+?)\s+time(?:\s*([+-])\s*(\d+(?:\.\d+)?)\s*(?:h|hrs?|hours?)?)?$"].map(|p| Regex::new(p).unwrap())
});
static DISTANCE: LazyLock<[Regex; 2]> = LazyLock::new(|| {
    [r"^(?:distance|distancia)\s+(?:from|between|de|entre)\s+(.+?)\s+(?:to|and|ate|e|a|para)\s+(.+?)(?:\s+in\s+(km|kilometers|kilometres|miles|mi|nautical miles|nmi))?$", r"^(?:distance|distancia)\s+(?:to|ate|para)\s+(.+?)(?:\s+in\s+(km|kilometers|kilometres|miles|mi|nautical miles|nmi))?$"].map(|p| Regex::new(p).unwrap())
});
static TRAVEL: LazyLock<[Regex; 3]> = LazyLock::new(|| {
    [
        r"^(?:travel\s+)?time\s+to\s+(.+?)\s+by\s+(car|foot|walking|bike|bicycle|cycling)$",
        r"^(walking|driving|cycling|biking)\s+time\s+(?:between|from)\s+(.+?)\s+(?:and|to)\s+(.+)$",
        r"^(walking|driving|cycling|biking)\s+time\s+to\s+(.+)$",
    ]
    .map(|p| Regex::new(p).unwrap())
});

fn money(amount: f64, code: &str) -> String {
    icu::format_money(amount, code, "en-US", if amount.abs() < 1.0 { 6 } else { 2 }).unwrap_or_else(|| format!("{} {code}", math::format_number(amount)))
}

fn currency_name(code: &str) -> String {
    math::capitalize(&icu::currency_name(code, time::locale()))
}

fn degrees(value: f64) -> String {
    format!("{}°", value.round())
}

fn profile_of(word: &str) -> (&'static str, &'static str) {
    match word {
        "foot" | "walking" => ("foot", "Walking"),
        "bike" | "bicycle" | "cycling" | "biking" => ("bike", "By Bike"),
        _ => ("car", "By Car"),
    }
}

fn find_zone(name: &str) -> Option<time::Place> {
    extra::zone_abbreviation(name).map(|(name, zone)| time::Place { name, zone: zone.to_owned() }).or_else(|| time::lookup(name, &time::PLACES))
}

fn weather_card(forecast: &serde_json::Value, geo: &places::Geo, query: &places::WeatherQuery, now_s: f64) -> Option<WeatherCard> {
    let locale = "en-US";
    let zone = forecast["timezone"].as_str().unwrap_or(&geo.zone).to_owned();
    let daily = &forecast["daily"];
    let hourly = &forecast["hourly"];
    let day = query.days.clamp(0, 7) as usize;
    let (temperature, code, is_day) = if day > 0 {
        (daily["temperature_2m_max"][day].as_f64()?, daily["weather_code"][day].as_i64()?, true)
    } else {
        let r = places::reading_at(forecast, query.hours, now_s)?;
        (r.temperature, r.code, r.day)
    };
    let target_s = if day > 0 { daily["time"][day].as_f64()? + 9.0 * 3600.0 } else { now_s + query.hours as f64 * 3600.0 };
    let range_day = if day > 0 { day } else { daily["time"].as_array()?.iter().rposition(|t| t.as_f64().is_some_and(|t| t <= target_s)).unwrap_or(0) };
    let (condition, icon) = places::condition(code, is_day);
    let start = places::hourly_index(forecast, target_s)?;
    let today = (0..8)
        .filter_map(|i| {
            let index = start + i;
            let at = hourly["time"][index].as_f64()?;
            let (_, icon) = places::condition(hourly["weather_code"][index].as_i64()?, hourly["is_day"][index].as_i64() != Some(0));
            let label = if i == 0 && day == 0 && query.hours == 0 { "Now".to_owned() } else { icu::format_pattern(at * 1000.0, locale, "HH:mm", Some(&zone)) };
            Some(Forecast { label, icon: icon.into(), high: degrees(hourly["temperature_2m"][index].as_f64()?), low: None })
        })
        .collect();
    let week = (0..8)
        .filter_map(|i| {
            let at = daily["time"][i].as_f64()?;
            let (_, icon) = places::condition(daily["weather_code"][i].as_i64()?, true);
            let label = if i == 0 { "Today".to_owned() } else { icu::format_pattern(at * 1000.0 + 12.0 * 3_600_000.0, locale, "EEE", Some(&zone)) };
            Some(Forecast { label, icon: icon.into(), high: degrees(daily["temperature_2m_max"][i].as_f64()?), low: Some(degrees(daily["temperature_2m_min"][i].as_f64()?)) })
        })
        .collect();
    Some(WeatherCard {
        temperature: degrees(temperature),
        icon: icon.into(),
        place: geo.full.clone(),
        condition: condition.into(),
        range: format!("H: {} L: {}", degrees(daily["temperature_2m_max"][range_day].as_f64()?), degrees(daily["temperature_2m_min"][range_day].as_f64()?)),
        today,
        week,
    })
}

fn flight_card(flight: &places::Flight, now_ms: f64) -> FlightCard {
    let locale = "en-US";
    let eta = places::flight_eta_ms(flight, now_ms);
    let place = |a: &places::Airport| [a.city.as_str(), a.country.as_str()].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(", ");
    let arrival = match (eta, &flight.destination_zone) {
        (Some(at), Some(zone)) => format!("Arrives about {} ({})", icu::format_pattern(at, locale, "EEE d MMM 'at' HH:mm", Some(zone)), icu::format_pattern(at, locale, "O", Some(zone))),
        _ => place(&flight.destination),
    };
    let (status, live) = match flight.live {
        Some((_, _, _, true)) => ("On the Ground", false),
        Some(_) => ("In Flight", true),
        None => ("Not in the Air", false),
    };
    FlightCard {
        from: (places::short_airport(&flight.origin.name, &flight.origin.city), place(&flight.origin), flight.origin.code.clone()),
        to: (places::short_airport(&flight.destination.name, &flight.destination.city), arrival, flight.destination.code.clone()),
        status: status.into(),
        live,
        note: None,
    }
}

fn compute(ctx: &egui::Context, query: &str) -> Option<Computed> {
    let text = query.trim();
    if text.is_empty() {
        return None;
    }
    if let Some(parsed) = color::parse(text) {
        return Some(Computed::Color(parsed));
    }
    let lower = text.to_lowercase();
    let folded = math::unaccent(&lower);
    places::begin(text);
    let answer = |kind, display: String, plain: String, input_label: String, label: String, note: Option<String>| Some(Computed::Answer(Answer { kind, input: text.to_owned(), input_label, display, plain, label, note, dim: None }));
    let dimmed = |kind, display: String, plain: String, label: String, note: Option<String>, dim: Vec<(usize, usize)>| Some(Computed::Answer(Answer { kind, input: text.to_owned(), input_label: "Expression".into(), display, plain, label, note, dim: Some(dim) }));
    let now = crate::store::now_ms() as f64;
    let locale = "en-US";
    if let Some((airline, number)) = places::parse_flight(&lower) {
        let Some(flight) = places::flight(ctx, &airline, &number) else {
            let open = format!("https://www.flightradar24.com/data/flights/{}{}", airline.to_lowercase(), number);
            return Some(Computed::Loading { section: "Flight Tracker", open_title: "Open Flight Tracker", open, fallback: None });
        };
        let flight = flight.ok()?;
        let card = flight_card(&flight, now);
        let copy = format!("{} {} → {}: {}", flight.number, flight.origin.code, flight.destination.code, card.status);
        return Some(Computed::Flight { card, open: format!("https://www.flightradar24.com/data/flights/{}", flight.number.to_lowercase()), copy });
    }
    if let Some(weather) = places::parse_weather(text) {
        let loading = || Some(if !matches!(weather.kind, places::WeatherKind::Card) { Computed::PairLoading } else { Computed::Loading { section: "Weather Forecast", open_title: "Open in Weather", open: "msnweather://forecast".into(), fallback: Some("https://www.msn.com/en-us/weather/forecast".into()) } });
        let Some(geo) = places::place(ctx, weather.place.as_deref()) else { return loading() };
        let geo = geo.ok()?;
        let Some(forecast) = places::forecast(ctx, &geo) else { return loading() };
        let forecast = forecast.ok()?;
        let unit = "°C";
        let now_s = now / 1000.0;
        match weather.kind {
            places::WeatherKind::Card => {
                let card = weather_card(&forecast, &geo, &weather, now_s)?;
                let copy = format!("{} {}, {} in {}", card.temperature.trim_end_matches('°'), unit, card.condition, geo.full);
                let open = format!("msnweather://forecast?la={:.4}&lo={:.4}", geo.lat, geo.lon);
                return Some(Computed::Weather { card, open, copy });
            }
            places::WeatherKind::FeelsLike => {
                let reading = places::reading_at(&forecast, weather.hours, now_s)?;
                let value = format!("{}{unit}", reading.feels_like.round());
                return dimmed(Kind::UnitConversion, value.clone(), value, format!("Feels Like in {}", geo.name), None, extra::dim_words(text, &["feels", "like", "in"]));
            }
            places::WeatherKind::Temperature(target) => {
                let reading = places::reading_at(&forecast, weather.hours, now_s)?;
                let (from, to) = (units::parse(&format!("0 {unit} in {unit}"))?.from, target.unwrap_or(unit));
                let to = units::parse(&format!("0 {to} in {to}"))?.to;
                let value = units::convert(reading.temperature, &from, &to)?;
                let display = format!("{}{}", math::format_number(math::to_precision(value, 3)), if to.id == "K" { " K" } else { to.id });
                return dimmed(Kind::UnitConversion, display.clone(), format!("{value:.1}"), format!("Temperature in {}", geo.name), None, extra::dim_words(text, &["temperature", "temp", "in"]));
            }
        }
    }
    if let Some(caps) = TRAVEL.iter().find_map(|r| r.captures(&folded)) {
        let (word, from, to) = match caps.len() {
            3 if caps.get(2).is_some_and(|m| ["car", "foot", "walking", "bike", "bicycle", "cycling"].contains(&m.as_str())) => (caps[2].to_owned(), None, caps[1].to_owned()),
            4 => (caps[1].to_owned(), Some(caps[2].to_owned()), caps[3].to_owned()),
            _ => (caps[1].to_owned(), None, caps[2].to_owned()),
        };
        let (profile, label) = profile_of(&word);
        let origin = ready!(places::place(ctx, from.as_deref()));
        let target = ready!(places::geocode(ctx, &to));
        let (seconds, meters) = ready!(places::route(ctx, &origin, &target, profile));
        let duration = units::format_duration((seconds / 60.0).round() * 60.0);
        let distance = format!("{} km", math::group(meters / 1000.0, 1));
        return dimmed(Kind::DateTime, duration.clone(), format!("{}", (seconds / 60.0).round()), label.into(), Some(distance), extra::dim_words(text, &["time", "to", "by", "between", "from", "and", "walking", "driving", "cycling", "biking", "travel"]));
    }
    if let Some(caps) = DISTANCE.iter().find_map(|r| r.captures(&folded)) {
        let two = caps.get(2).is_some_and(|m| !["km", "kilometers", "kilometres", "miles", "mi", "nautical miles", "nmi"].contains(&m.as_str()));
        let (from, to, unit) = if two { (Some(caps[1].to_owned()), caps[2].to_owned(), caps.get(3)) } else { (None, caps[1].to_owned(), caps.get(2)) };
        let a = ready!(places::place(ctx, from.as_deref()));
        let b = ready!(places::geocode(ctx, &to));
        let km = extra::haversine_km((a.lat, a.lon), (b.lat, b.lon));
        let unit = unit.map_or("km", |m| match m.as_str() {
            "miles" | "mi" => "mi",
            "nautical miles" | "nmi" => "nmi",
            _ => "km",
        });
        let (value, label) = match unit {
            "mi" => (km / 1.609344, "Miles"),
            "nmi" => (km / 1.852, "Nautical Miles"),
            _ => (km, "Kilometers"),
        };
        let display = format!("{} {unit}", math::group(value, 4));
        return dimmed(Kind::UnitConversion, display, format!("{value:.4}"), label.into(), None, extra::dim_words(text, &["distance", "distancia", "from", "between", "to", "and", "de", "entre", "ate", "para", "in"]));
    }
    let today = ((now + icu::local_offset_minutes(now) as f64 * 60_000.0) / extra::DAY_MS).floor() as i64;
    let local_now = now + icu::local_offset_minutes(now) as f64 * 60_000.0;
    let range = |a: i64, b: i64| {
        let (ya, _, _) = extra::civil(a);
        let (yb, _, _) = extra::civil(b);
        let first = icu::format_skeleton(a as f64 * extra::DAY_MS, "en-US", if ya == yb { "MMMd" } else { "yMMMd" }, Some("UTC"));
        let last = icu::format_skeleton(b as f64 * extra::DAY_MS, "en-US", "yMMMd", Some("UTC"));
        format!("{first}–{last}")
    };
    let parse_day = |t: &str, today: i64| time::parse_target_day(t, today);
    let pure = extra::week(&folded, today, &range)
        .or_else(|| extra::time_left(&folded, local_now))
        .or_else(|| extra::since(&folded, local_now, &parse_day))
        .or_else(|| extra::list_function(&folded))
        .or_else(|| extra::nearest(&folded))
        .or_else(|| extra::as_fraction(&folded))
        .or_else(|| extra::base(&folded))
        .or_else(|| extra::money_query(&lower))
        .or_else(|| extra::constant(&folded))
        .or_else(|| extra::chemistry(text))
        .or_else(|| extra::pace(&folded))
        .or_else(|| extra::transfer(&folded))
        .or_else(|| extra::note(&lower))
        .or_else(|| extra::text_function(text));
    if let Some(p) = pure {
        let mut answer = Answer { kind: Kind::Math, input: text.to_owned(), input_label: "Expression".into(), display: p.display, plain: p.plain, label: p.label, note: None, dim: (!p.dim.is_empty()).then_some(p.dim) };
        if p.unit_like {
            answer.input = answer.input.replace('/', "÷");
        }
        return Some(Computed::Answer(answer));
    }
    if let Some(unit) = units::parse(text) {
        let category = unit.from.category.label().to_owned();
        let note = (unit.from.category == units::Category::Time).then(|| units::format_duration(unit.value * unit.to.factor));
        return answer(Kind::UnitConversion, format!("{} {}", units::format_value(unit.value), unit.to.id), format!("{}", math::to_precision(unit.value, 12)), category.clone(), category, note);
    }
    if CURRENCY_HINT.iter().all(|r| r.is_match(text)) {
        if let Some(table) = rates(ctx) {
            let known: HashSet<String> = table.rates.keys().cloned().collect();
            if let Some(parsed) = math::parse_currency(text, &known) {
                if let Some(value) = math::convert_currency(&parsed, &table.rates) {
                    return answer(Kind::UnitConversion, money(value, &parsed.to), format!("{value:.2}"), currency_name(&parsed.from), currency_name(&parsed.to), Some(time::updated_ago(table.updated, crate::store::now_ms())));
                }
            }
        }
    }
    if let Some((clock, from, to)) = time::parse_clock_query(text) {
        if let (Some(from), Some(to)) = (find_zone(&from), find_zone(&to)) {
            let instant = time::instant_for(time::zoned_day(now, &from.zone), clock, &from.zone);
            let when = icu::format_date(instant, locale, Style::None, Style::Short, Some(&to.zone));
            let day = icu::format_skeleton(instant, locale, "EEEEMMMMd", Some(&to.zone));
            return answer(Kind::DateTime, when.clone(), when, "Expression".into(), day, Some(format!("{} → {}", from.name, to.name)));
        }
    }
    if let Some(delta) = time::parse_relative(text) {
        let when = icu::format_date(now + delta, locale, Style::Full, Style::Short, None);
        return answer(Kind::DateTime, when.clone(), when, "Date".into(), "Date".into(), None);
    }
    if let Some(target) = time::parse_days_until(text).and_then(|t| time::parse_target_day(&t, today)) {
        let days = target - today;
        let label = format!("{} day{}", days.abs(), if days.abs() == 1 { "" } else { "s" });
        return answer(Kind::DateTime, label, days.to_string(), "Date".into(), "Date".into(), (days < 0).then(|| "Past".into()));
    }
    if let Some((from, to)) = time::parse_time_diff(text) {
        if let (Some(from), Some(to)) = (find_zone(&from), find_zone(&to)) {
            let diff = time::format_diff(icu::offset_minutes(now, &to.zone) - icu::offset_minutes(now, &from.zone));
            return answer(Kind::DateTime, diff.clone(), diff, "Time difference".into(), format!("{} → {}", from.name, to.name), None);
        }
    }
    if let Some(caps) = TIME_AT.iter().find_map(|r| r.captures(&folded)) {
        let name = caps[1].trim().to_owned();
        let zone = match find_zone(&name) {
            Some(place) => place.zone,
            None if TIME_AT[0].is_match(&folded) && name.split_whitespace().count() <= 5 => ready!(places::geocode(ctx, &name)).zone,
            // "X time" also matches phrases like "screen time", so it waits silently instead of flashing a skeleton
            None if name.split_whitespace().count() <= 5 => places::geocode(ctx, &name)?.ok()?.zone,
            None => return None,
        };
        let shift = caps.get(3).and_then(|n| n.as_str().parse::<f64>().ok()).map_or(0.0, |h| if &caps[2] == "-" { -h } else { h });
        let at = now + shift * 3_600_000.0;
        let when = icu::format_date(at, locale, Style::None, Style::Short, Some(&zone));
        let day = icu::format_skeleton(at, locale, "EEEEMMMMd", Some(&zone));
        return answer(Kind::DateTime, when.clone(), when, "Expression".into(), day, None);
    }
    if !math::is_expression(text) {
        return None;
    }
    let value = math::evaluate(text)?;
    if math::plain_number(value) == text {
        return None;
    }
    answer(Kind::Math, math::format_number(value), math::plain_number(value), "Expression".into(), math::number_to_words(value).unwrap_or_else(|| "Result".into()), None)
}

pub fn calculate(ctx: &egui::Context, query: &str) -> Option<Answer> {
    match compute(ctx, query)? {
        Computed::Answer(answer) => Some(answer),
        _ => None,
    }
}

fn copy_paste(text: &str, app: &Option<String>, after: Rc<dyn Fn()>) -> Vec<Action> {
    let (copy, paste) = (text.to_owned(), text.to_owned());
    let then = after.clone();
    vec![
        Action::new("copy", "Copy Answer", move |host| {
            host.copy_and_close(&copy, "Copied to Clipboard");
            then();
            Ok(())
        })
        .icon("clipboard")
        .shortcut(Shortcut::key(Key::C).ctrl().shift()),
        Action::new("paste", paste_title(app), move |host| {
            host.paste(&paste)?;
            after();
            Ok(())
        })
        .icon("clipboard-paste")
        .shortcut(Shortcut::key(Key::Enter).ctrl()),
    ]
}

fn copy_question_and_answer(question: &str, answer: &str, after: Rc<dyn Fn()>) -> Action {
    let text = format!("{question} = {answer}");
    Action::new("copy-question-and-answer", "Copy Question and Answer", move |host| {
        host.copy_and_close(&text, "Copied to Clipboard");
        after();
        Ok(())
    })
    .icon("clipboard")
    .shortcut(Shortcut::key(Key::C).alt().shift())
}

pub fn swatch(c: &color::Rgba) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(c.r.round() as u8, c.g.round() as u8, c.b.round() as u8, (c.a * 255.0).round() as u8)
}

fn open_and_copy(open_title: &str, open: String, copy_title: &str, copy: String, fallback: Option<String>) -> Vec<Action> {
    vec![
        Action::new("open", open_title, move |_| crate::apps::open_url(&open, None).or_else(|e| fallback.as_deref().map_or(Err(e), |url| crate::apps::open_url(url, None)))).icon("external-link"),
        Action::new("copy", copy_title, move |host| {
            host.copy_and_close(&copy, "Copied to Clipboard");
            Ok(())
        })
        .icon("clipboard")
        .shortcut(Shortcut::key(Key::C).ctrl().shift()),
    ]
}

fn side(text: String, label: String) -> Side {
    Side { text, label, swatch: None, dim: None }
}

pub struct Calculator;

impl Extension for Calculator {
    fn id(&self) -> &'static str {
        "calculator"
    }

    fn commands(&self) -> Vec<Command> {
        std::thread::spawn(|| {
            LazyLock::force(&time::PLACES);
        });
        let mut cmd = command(self.id(), "history", "Calculator History", Icon::Tile("calculator", Tone::Orange), "Command", "Open Command", |_| Ok(()));
        cmd.subtitle = Some("Calculator".into());
        cmd.keywords = ["calculator", "math", "history", "calculations", "conversions", "calculadora", "matemática", "conversões", "histórico"].map(String::from).to_vec();
        cmd.view = Some(Rc::new(|ctx| Box::new(CalculatorHistory::new(ctx))));
        vec![cmd]
    }

    fn inline(&self, ctx: &egui::Context, query: &str) -> Option<Inline> {
        match compute(ctx, query)? {
            Computed::Color(parsed) => {
                let list = color::formats(&parsed.rgba);
                let items: Vec<(&'static str, &'static str, String)> = list.into_iter().map(|f| (f.id, f.label, f.value)).collect();
                Some(Inline {
                    section: "Color".into(),
                    input: side(query.trim().into(), parsed.format.into()),
                    output: Side { swatch: Some(swatch(&parsed.rgba)), ..side(String::new(), color::name(&parsed.rgba).into()) },
                    note: None,
                    actions: vec![vec![Action::new("copy-color-as", "Copy Color as…", |_| Ok(()))
                        .icon("clipboard")
                        .submenu(move || {
                            items
                                .iter()
                                .map(|(id, label, value)| {
                                    let (copied, title) = (value.clone(), format!("Copied {label} to Clipboard"));
                                    Action::new(id, *label, move |host| {
                                        host.copy_and_close(&copied, &title);
                                        Ok(())
                                    })
                                    .accessory(value.clone())
                                })
                                .collect()
                        })]],
                    kind: CardKind::Pair,
                })
            }
            Computed::Weather { card, open, copy } => {
                let web = format!("https://www.msn.com/en-us/weather/forecast/in-{}", card.place.replace(", ", ",").replace(' ', "-"));
                Some(Inline {
                    section: "Weather Forecast".into(),
                    input: side(query.trim().into(), String::new()),
                    output: side(card.temperature.clone(), card.condition.clone()),
                    note: None,
                    actions: vec![open_and_copy("Open in Weather", open, "Copy Weather", copy, Some(web))],
                    kind: CardKind::Weather(card),
                })
            }
            Computed::Loading { section, open_title, open, fallback } => Some(Inline {
                section: section.into(),
                input: side(query.trim().into(), String::new()),
                output: side(String::new(), String::new()),
                note: None,
                actions: vec![vec![Action::new("open", open_title, move |_| crate::apps::open_url(&open, None).or_else(|e| fallback.as_deref().map_or(Err(e), |url| crate::apps::open_url(url, None)))).icon("external-link")]],
                kind: CardKind::Loading,
            }),
            Computed::PairLoading => Some(Inline {
                section: "Calculator".into(),
                input: side(query.trim().into(), "Expression".into()),
                output: side(String::new(), String::new()),
                note: None,
                actions: Vec::new(),
                kind: CardKind::LoadingPair,
            }),
            Computed::Flight { card, open, copy } => Some(Inline {
                section: "Flight Tracker".into(),
                input: side(query.trim().into(), String::new()),
                output: side(card.status.clone(), card.to.0.clone()),
                note: None,
                actions: vec![open_and_copy("Open Flight Tracker", open, "Copy Flight Status", copy, None)],
                kind: CardKind::Flight(card),
            }),
            Computed::Answer(a) => {
                let (input, display, kind) = (a.input.clone(), a.display.clone(), a.kind);
                let save: Rc<dyn Fn()> = Rc::new(move || history::record(&input, &display, kind));
                let app = frontmost_app();
                let mut actions = copy_paste(&a.plain, &app, save.clone());
                actions.push(copy_question_and_answer(&a.input, &a.display, save));
                Some(Inline {
                    section: "Calculator".into(),
                    input: Side { dim: a.dim, ..side(a.input, a.input_label) },
                    output: side(a.display, a.label),
                    note: a.note,
                    actions: vec![actions],
                    kind: CardKind::Pair,
                })
            }
        }
    }
}

fn kind_glyph(c: &Calculation) -> &'static str {
    let last = c.expression.trim().chars().last();
    if last.is_some_and(|ch| "+-*/^%×÷".contains(ch)) {
        return "file-text";
    }
    match c.kind {
        Kind::Math => "calculator",
        Kind::UnitConversion => "arrow-left-right",
        Kind::DateTime => "clock",
    }
}

struct CalculatorHistory {
    ctx: egui::Context,
    list: Vec<Calculation>,
    dirty: Rc<Cell<bool>>,
    query: String,
    app: Option<String>,
}

impl CalculatorHistory {
    fn new(ctx: &egui::Context) -> Self {
        Self { ctx: ctx.clone(), list: history::load(), dirty: Rc::new(Cell::new(false)), query: String::new(), app: frontmost_app() }
    }

    fn change(&self, id: &'static str, title: &str, f: impl Fn(Vec<Calculation>) -> Vec<Calculation> + 'static) -> Action {
        let dirty = self.dirty.clone();
        let f = Rc::new(f);
        Action::new(id, title, move |_| {
            let f = f.clone();
            history::update(move |list| f(list))?;
            dirty.set(true);
            Ok(())
        })
        .keep_open()
    }

    fn put_in_search(text: String) -> impl Fn(&mut Host) -> Result<(), String> {
        move |host| {
            host.requests.push(Request::SetQuery(text.clone()));
            Ok(())
        }
    }

    fn entry_actions(&self, c: &Calculation, order: &[Calculation]) -> Vec<Vec<Action>> {
        let nothing: Rc<dyn Fn()> = Rc::new(|| {});
        let (id, expression, result) = (c.id.clone(), c.expression.clone(), c.result.clone());
        let mut copies = copy_paste(&result, &self.app, nothing.clone());
        let question = expression.clone();
        copies.push(
            Action::new("copy-question", "Copy Question", move |host| {
                host.copy_and_close(&question, "Copied to Clipboard");
                Ok(())
            })
            .icon("clipboard")
            .shortcut(Shortcut::key(Key::C).ctrl().alt()),
        );
        copies.push(copy_question_and_answer(&expression, &result, nothing));
        let put = vec![
            Action::new("put-question", "Put Question in Search Bar", Self::put_in_search(format!("{expression} "))).icon("text-cursor-input").shortcut(Shortcut::key(Key::Enter).alt().shift()).keep_open(),
            Action::new("put-answer", "Put Answer in Search Bar", Self::put_in_search(result.clone())).icon("text-cursor-input").shortcut(Shortcut::key(Key::Enter).ctrl().shift()).keep_open(),
        ];
        let index = order.iter().position(|p| p.id == c.id);
        let mut pins = Vec::new();
        let pin_id = id.clone();
        if c.pinned.is_some() {
            pins.push(self.change("unpin", "Unpin Entry", move |l| history::set_pinned(l, &pin_id, false)).icon("pin-off").shortcut(Shortcut::key(Key::Period).ctrl()));
            if index.is_some_and(|i| i > 0) {
                let up = id.clone();
                pins.push(self.change("move-up", "Move Up in Pinned", move |l| history::move_pinned(l, &up, -1)).icon("arrow-up").shortcut(Shortcut::key(Key::ArrowUp).ctrl().alt()));
            }
            if index.is_some_and(|i| i + 1 < order.len()) {
                let down = id.clone();
                pins.push(self.change("move-down", "Move Down in Pinned", move |l| history::move_pinned(l, &down, 1)).icon("arrow-down").shortcut(Shortcut::key(Key::ArrowDown).ctrl().alt()));
            }
        } else {
            pins.push(self.change("pin", "Pin Entry", move |l| history::set_pinned(l, &pin_id, true)).icon("pin").shortcut(Shortcut::key(Key::Period).ctrl()));
        }
        let delete_id = id.clone();
        let deletes = vec![
            self.change("delete", "Delete Entry", move |l| l.into_iter().filter(|e| e.id != delete_id).collect()).icon("trash-2").destructive().shortcut(Shortcut::key(Key::D).ctrl()),
            self.change("delete-all", "Delete All Entries", |l| l.into_iter().filter(|e| e.pinned.is_some()).collect())
                .icon("trash-2")
                .destructive()
                .shortcut(Shortcut::key(Key::D).ctrl().alt())
                .confirm("Delete All Entries", "Are you sure you want to proceed with deleting all calculator history entries? Only pinned entries will remain.", "Delete All"),
        ];
        vec![copies, put, pins, deletes]
    }

    fn live_item(&self, live: Answer) -> Item {
        let (input, display, kind) = (live.input.clone(), live.display.clone(), live.kind);
        let dirty = self.dirty.clone();
        let save: Rc<dyn Fn()> = Rc::new(move || {
            history::record(&input, &display, kind);
            dirty.set(true);
        });
        let mut copies = copy_paste(&live.display, &self.app, save.clone());
        copies.push(copy_question_and_answer(&live.input, &live.display, save));
        let saved = self.list.iter().find(|c| c.expression == live.input.trim() && c.result == live.display).cloned();
        let pin = match saved.filter(|c| c.pinned.is_some()) {
            Some(c) => self.change("unpin", "Unpin Entry", move |l| history::set_pinned(l, &c.id, false)).icon("pin-off"),
            None => {
                let (input, display) = (live.input.clone(), live.display.clone());
                self.change("pin", "Pin Entry", move |l| {
                    let (list, id) = history::upsert(l, &input, &display, kind, crate::store::now_ms());
                    history::set_pinned(list, &id, true)
                })
                .icon("pin")
            }
        }
        .shortcut(Shortcut::key(Key::Period).ctrl());
        Item {
            id: "live".into(),
            title: live.input.clone(),
            subtitle: Some(live.label.clone()),
            icon: Icon::Glyph("calculator"),
            accessories: vec![Accessory::Text(live.display.clone())],
            keywords: Vec::new(),
            actions: vec![copies, vec![Action::new("put-answer", "Put Answer in Search Bar", Self::put_in_search(live.display.clone())).icon("text-cursor-input").shortcut(Shortcut::key(Key::Enter).ctrl().shift()).keep_open(), pin]],
        }
    }
}

impl View for CalculatorHistory {
    fn placeholder(&self) -> String {
        "Do math, convert units, or search your past calculations...".into()
    }

    fn footer(&self) -> (String, Icon) {
        ("Calculator History".into(), Icon::Tile("calculator", Tone::Orange))
    }

    fn filters(&self) -> bool {
        false
    }

    fn set_query(&mut self, query: &str) {
        self.query = query.to_owned();
    }

    fn empty(&self) -> Empty {
        if self.list.is_empty() && self.query.is_empty() {
            Empty { title: "You have no recent calculations".into(), description: Some("Try \"200 + 10%\", \"100 usd in eur\", or \"Tokyo time\"".into()) }
        } else {
            Empty { title: "No results".into(), description: Some("Try a different expression".into()) }
        }
    }

    fn sections(&mut self) -> Vec<Section> {
        if self.dirty.replace(false) {
            self.list = history::load();
        }
        let mut sections = Vec::new();
        if let Some(live) = calculate(&self.ctx, &self.query) {
            sections.push(Section { title: String::new(), items: vec![self.live_item(live)] });
        }
        let order = history::pinned(&self.list);
        let offset = icu::local_offset_minutes(crate::store::now_ms() as f64) as i64 * 60_000;
        for (title, items) in history::group(&history::search(&self.list, &self.query), crate::store::now_ms(), offset) {
            sections.push(Section {
                title: title.into(),
                items: items
                    .iter()
                    .map(|c| Item { id: c.id.clone(), title: c.expression.clone(), subtitle: None, icon: Icon::Glyph(kind_glyph(c)), accessories: vec![Accessory::Text(c.result.clone())], keywords: Vec::new(), actions: self.entry_actions(c, &order) })
                    .collect(),
            });
        }
        sections
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crypto_rates_invert_usd_prices() {
        let prices = serde_json::json!({ "bitcoin": { "usd": 50000.0 }, "ethereum": { "usd": 0 }, "solana": {} });
        let rates = crypto_rates_from_prices(&prices);
        assert_eq!(rates.len(), 1);
        assert_eq!(rates["BTC"], 1.0 / 50000.0);
    }
}

#[cfg(test)]
mod prefix_tests {
    #[test]
    fn typing_never_panics() {
        let ctx = egui::Context::default();
        for text in ["calculator history", "2+2*3", "time in tokyo", "#4fa3f8", "days until 25/12", "5pm london in tokyo"] {
            for end in text.char_indices().map(|(i, c)| i + c.len_utf8()) {
                let _ = super::compute(&ctx, &text[..end]);
            }
        }
    }
}
