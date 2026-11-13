use super::extra;
use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

const GEOCODE: &str = "https://geocoding-api.open-meteo.com/v1/search";
const FORECAST: &str = "https://api.open-meteo.com/v1/forecast";
const REVERSE: &str = "https://api.bigdatacloud.net/data/reverse-geocode-client";
const ROUTES: &str = "https://routing.openstreetmap.de";
const AIRLINE: &str = "https://api.adsbdb.com/v0/airline/";
const ROUTE_OF: &str = "https://api.adsbdb.com/v0/callsign/";
const LIVE: &str = "https://api.adsb.lol/v2/callsign/";
const DEBOUNCE: Duration = Duration::from_millis(350);

enum Slot {
    Pending,
    Done(Instant, Result<Value, String>),
}

static CACHE: LazyLock<Mutex<HashMap<String, Slot>>> = LazyLock::new(|| Mutex::new(HashMap::new()));
static QUERY: Mutex<String> = Mutex::new(String::new());

pub fn begin(query: &str) {
    *lock(&QUERY) = query.trim().to_lowercase();
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

fn cached(ctx: &egui::Context, key: String, ttl: Duration, work: impl FnOnce() -> Result<Value, String> + Send + 'static) -> Option<Result<Value, String>> {
    let query = lock(&QUERY).clone();
    let mut cache = lock(&CACHE);
    match cache.get(&key) {
        Some(Slot::Done(at, result)) if at.elapsed() < ttl || result.is_err() && at.elapsed() < Duration::from_secs(30) => return Some(result.clone()),
        Some(Slot::Pending) => return None,
        _ => {}
    }
    cache.insert(key.clone(), Slot::Pending);
    drop(cache);
    let ctx = ctx.clone();
    std::thread::spawn(move || {
        std::thread::sleep(DEBOUNCE);
        let still_wanted = *lock(&QUERY) == query;
        if !still_wanted {
            lock(&CACHE).remove(&key);
            return;
        }
        let result = work();
        lock(&CACHE).insert(key, Slot::Done(Instant::now(), result));
        super::RATES_VERSION.fetch_add(1, Ordering::SeqCst);
        ctx.request_repaint();
    });
    None
}

fn get_json(url: &str) -> Result<Value, String> {
    match crate::http::get_json(url) {
        Ok(Some(value)) => Ok(value),
        Ok(None) => Err("not found".into()),
        Err(crate::http::Failure::Offline) => Err("You're offline".into()),
        Err(crate::http::Failure::Status(status)) => Err(format!("the service answered {status}")),
        Err(crate::http::Failure::Body(e)) => Err(e),
    }
}

fn with_query(base: &str, pairs: &[(&str, &str)]) -> String {
    let mut url = url::Url::parse(base).expect("valid base url");
    url.query_pairs_mut().extend_pairs(pairs);
    url.to_string()
}

#[derive(Clone, PartialEq, Debug)]
pub struct Geo {
    pub name: String,
    pub full: String,
    pub lat: f64,
    pub lon: f64,
    pub zone: String,
}

impl Geo {
    fn from_value(value: &Value) -> Option<Geo> {
        Some(Geo {
            name: value["name"].as_str()?.to_owned(),
            full: value["full"].as_str()?.to_owned(),
            lat: value["lat"].as_f64()?,
            lon: value["lon"].as_f64()?,
            zone: value["zone"].as_str().unwrap_or("UTC").to_owned(),
        })
    }

    fn to_value(&self) -> Value {
        serde_json::json!({ "name": self.name, "full": self.full, "lat": self.lat, "lon": self.lon, "zone": self.zone })
    }
}

const ALIASES: [(&str, &str); 8] = [("sf", "San Francisco"), ("nyc", "New York"), ("ny", "New York"), ("la", "Los Angeles"), ("ldn", "London"), ("sp", "Sao Paulo"), ("rj", "Rio de Janeiro"), ("dc", "Washington")];

pub fn geocode_blocking(query: &str) -> Result<Geo, String> {
    let trimmed = query.trim();
    let name = ALIASES.iter().find(|(a, _)| a.eq_ignore_ascii_case(trimmed)).map_or(trimmed, |(_, full)| full);
    let body = get_json(&with_query(GEOCODE, &[("name", name), ("count", "1"), ("language", "en"), ("format", "json")]))?;
    let found = body["results"].get(0).ok_or_else(|| format!("No place named “{trimmed}”"))?;
    let parts: Vec<&str> = ["name", "admin1", "country"].iter().filter_map(|k| found[k].as_str()).collect();
    let mut full: Vec<&str> = Vec::new();
    for part in parts {
        if !full.contains(&part) {
            full.push(part);
        }
    }
    Ok(Geo {
        name: found["name"].as_str().unwrap_or(trimmed).to_owned(),
        full: full.join(", "),
        lat: found["latitude"].as_f64().ok_or("missing latitude")?,
        lon: found["longitude"].as_f64().ok_or("missing longitude")?,
        zone: found["timezone"].as_str().unwrap_or("UTC").to_owned(),
    })
}

pub fn geocode(ctx: &egui::Context, query: &str) -> Option<Result<Geo, String>> {
    let owned = query.trim().to_owned();
    let result = cached(ctx, format!("geo:{}", owned.to_lowercase()), Duration::from_secs(24 * 3600), move || geocode_blocking(&owned).map(|g| g.to_value()))?;
    Some(result.and_then(|v| Geo::from_value(&v).ok_or_else(|| "bad place".into())))
}

fn here_blocking() -> Result<Geo, String> {
    use windows::Devices::Geolocation::{GeolocationAccessStatus, Geolocator};
    let denied = || "Turn on location in Windows Settings › Privacy › Location".to_string();
    let access = Geolocator::RequestAccessAsync().and_then(|op| op.get()).map_err(|_| denied())?;
    if access != GeolocationAccessStatus::Allowed {
        return Err(denied());
    }
    let locator = Geolocator::new().map_err(|e| e.message())?;
    let position = locator.GetGeopositionAsync().and_then(|op| op.get()).and_then(|p| p.Coordinate()).and_then(|c| c.Point()).and_then(|p| p.Position()).map_err(|_| denied())?;
    let (lat, lon) = (position.Latitude, position.Longitude);
    let place = get_json(&with_query(REVERSE, &[("latitude", &lat.to_string()), ("longitude", &lon.to_string()), ("localityLanguage", "en")])).ok();
    let city = place.as_ref().and_then(|p| p["city"].as_str().filter(|s| !s.is_empty()).or(p["locality"].as_str())).unwrap_or("Current Location").to_owned();
    let full = place.as_ref().map(|p| [p["city"].as_str(), p["principalSubdivision"].as_str(), p["countryName"].as_str()].into_iter().flatten().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(", ")).filter(|s| !s.is_empty()).unwrap_or_else(|| city.clone());
    let zone = local_zone().unwrap_or_else(|| "UTC".into());
    Ok(Geo { name: city, full, lat, lon, zone })
}

fn local_zone() -> Option<String> {
    windows::Globalization::Calendar::new().ok()?.GetTimeZone().ok().map(|z| z.to_string())
}

pub fn here(ctx: &egui::Context) -> Option<Result<Geo, String>> {
    let result = cached(ctx, "here".into(), Duration::from_secs(30 * 60), || here_blocking().map(|g| g.to_value()))?;
    Some(result.and_then(|v| Geo::from_value(&v).ok_or_else(|| "bad place".into())))
}

pub fn place(ctx: &egui::Context, name: Option<&str>) -> Option<Result<Geo, String>> {
    match name.map(str::trim).filter(|n| !n.is_empty() && !matches!(*n, "here" | "aqui" | "me")) {
        Some(name) => geocode(ctx, name),
        None => here(ctx),
    }
}

pub fn forecast(ctx: &egui::Context, geo: &Geo) -> Option<Result<Value, String>> {
    let unit = "celsius";
    let (lat, lon) = (format!("{:.4}", geo.lat), format!("{:.4}", geo.lon));
    let key = format!("wx:{lat},{lon},{unit}");
    cached(ctx, key, Duration::from_secs(10 * 60), move || {
        get_json(&with_query(
            FORECAST,
            &[
                ("latitude", &lat),
                ("longitude", &lon),
                ("current", "temperature_2m,apparent_temperature,weather_code,is_day"),
                ("hourly", "temperature_2m,apparent_temperature,weather_code,is_day"),
                ("daily", "weather_code,temperature_2m_max,temperature_2m_min"),
                ("timezone", "auto"),
                ("forecast_days", "8"),
                ("temperature_unit", unit),
                ("timeformat", "unixtime"),
            ],
        ))
    })
}

pub fn route(ctx: &egui::Context, from: &Geo, to: &Geo, profile: &'static str) -> Option<Result<(f64, f64), String>> {
    let coords = format!("{:.5},{:.5};{:.5},{:.5}", from.lon, from.lat, to.lon, to.lat);
    let key = format!("route:{profile}:{coords}");
    let result = cached(ctx, key, Duration::from_secs(3600), move || {
        let body = get_json(&format!("{ROUTES}/routed-{profile}/route/v1/driving/{coords}?overview=false"))?;
        if body["code"] != "Ok" {
            return Err("No route found".into());
        }
        Ok(body["routes"][0].clone())
    })?;
    Some(result.and_then(|r| Ok((r["duration"].as_f64().ok_or("No route found")?, r["distance"].as_f64().ok_or("No route found")?))))
}

#[derive(Clone, Debug, PartialEq)]
pub struct Airport {
    pub name: String,
    pub city: String,
    pub country: String,
    pub code: String,
    pub lat: f64,
    pub lon: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Flight {
    pub callsign: String,
    pub number: String,
    pub airline: String,
    pub origin: Airport,
    pub destination: Airport,
    pub destination_zone: Option<String>,
    pub live: Option<(f64, f64, f64, bool)>,
}

pub fn short_airport(name: &str, city: &str) -> String {
    let mut short = name.to_owned();
    for suffix in [" International Airport", " Intl Airport", " Airport", " Aeroporto Internacional", " Aerodrome", " Airfield"] {
        if let Some(stripped) = short.strip_suffix(suffix) {
            short = stripped.to_owned();
            break;
        }
    }
    if let Some(stripped) = short.strip_prefix(&format!("{city} ")) {
        if !stripped.is_empty() {
            short = stripped.to_owned();
        }
    }
    short
}

fn airport(value: &Value) -> Option<Airport> {
    Some(Airport {
        name: value["name"].as_str()?.to_owned(),
        city: value["municipality"].as_str().unwrap_or_default().to_owned(),
        country: value["country_name"].as_str().unwrap_or_default().to_owned(),
        code: value["iata_code"].as_str().or(value["icao_code"].as_str()).unwrap_or_default().to_owned(),
        lat: value["latitude"].as_f64()?,
        lon: value["longitude"].as_f64()?,
    })
}

static FLIGHT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(?:flight|voo|flt)\s+([a-z0-9]{2}|[a-z]{3})\s*(\d{1,4}[a-z]?)$").expect("flight pattern"));

pub fn parse_flight(text: &str) -> Option<(String, String)> {
    let caps = FLIGHT.captures(text)?;
    let airline = caps[1].to_uppercase();
    (airline.chars().any(|c| c.is_ascii_alphabetic())).then(|| (airline, caps[2].to_uppercase()))
}

fn flight_blocking(airline: &str, number: &str) -> Result<Value, String> {
    let icao = if airline.len() == 3 {
        airline.to_owned()
    } else {
        let body = get_json(&format!("{AIRLINE}{airline}")).map_err(|_| format!("No airline with code {airline}"))?;
        body["response"][0]["icao"].as_str().ok_or_else(|| format!("No airline with code {airline}"))?.to_owned()
    };
    let callsign = format!("{icao}{number}");
    let route = get_json(&format!("{ROUTE_OF}{callsign}")).map_err(|_| format!("No route found for {airline}{number}"))?;
    let route = route["response"]["flightroute"].clone();
    if route.is_null() {
        return Err(format!("No route found for {airline}{number}"));
    }
    let live = get_json(&format!("{LIVE}{callsign}")).ok().and_then(|v| v["ac"].get(0).cloned()).unwrap_or(Value::Null);
    let city = route["destination"]["municipality"].as_str().unwrap_or_default().to_owned();
    let zone = if city.is_empty() { None } else { geocode_blocking(&city).ok().map(|g| g.zone) };
    Ok(serde_json::json!({ "route": route, "live": live, "zone": zone, "callsign": callsign }))
}

pub fn flight(ctx: &egui::Context, airline: &str, number: &str) -> Option<Result<Flight, String>> {
    let (a, n) = (airline.to_owned(), number.to_owned());
    let result = cached(ctx, format!("flight:{airline}{number}"), Duration::from_secs(120), move || flight_blocking(&a, &n))?;
    Some(result.and_then(|v| {
        let route = &v["route"];
        let live = &v["live"];
        let on_ground = live["alt_baro"].as_str() == Some("ground");
        Ok(Flight {
            callsign: v["callsign"].as_str().unwrap_or_default().to_owned(),
            number: route["callsign_iata"].as_str().map_or_else(|| format!("{airline}{number}"), String::from),
            airline: route["airline"]["name"].as_str().unwrap_or_default().to_owned(),
            origin: airport(&route["origin"]).ok_or("The route has no origin")?,
            destination: airport(&route["destination"]).ok_or("The route has no destination")?,
            destination_zone: v["zone"].as_str().map(String::from),
            live: live["lat"].as_f64().zip(live["lon"].as_f64()).map(|(lat, lon)| (lat, lon, live["gs"].as_f64().unwrap_or(0.0), on_ground)),
        })
    }))
}

pub fn flight_eta_ms(flight: &Flight, now_ms: f64) -> Option<f64> {
    let (lat, lon, knots, ground) = flight.live?;
    if ground || knots < 80.0 {
        return None;
    }
    let remaining = extra::haversine_km((lat, lon), (flight.destination.lat, flight.destination.lon));
    Some(now_ms + remaining / (knots * 1.852) * 3_600_000.0)
}

#[derive(Debug, PartialEq)]
pub enum WeatherKind {
    Card,
    FeelsLike,
    Temperature(Option<&'static str>),
}

#[derive(Debug, PartialEq)]
pub struct WeatherQuery {
    pub place: Option<String>,
    pub hours: i64,
    pub days: i64,
    pub kind: WeatherKind,
}

static WEATHER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(?:weather|forecast|clima|tempo|previsao do tempo|previsao)\b(.*)$").expect("weather pattern"));
static WHEN_HOURS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b(?:in|em|daqui a)\s+(\d{1,3})\s+(hours?|horas?|days?|dias?)\b").expect("hours pattern"));
static WHEN_DAY: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b(tomorrow|amanha|today|hoje)\b").expect("day pattern"));
static WHERE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(?:in|at|for|em|de|no|na|para)\s+(.+)$").expect("where pattern"));
static FEELS: LazyLock<[Regex; 3]> = LazyLock::new(|| [r"^(.+?)\s+feels like$", r"^feels like(?:\s+in)?\s+(.+)$", r"^(?:sensacao termica)(?:\s+(?:em|no|na))?\s+(.+)$"].map(|p| Regex::new(p).expect("feels pattern")));
static TEMPERATURE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(?:temperature|temp|temperatura)(?:\s+(?:in|at|em|de|no|na))?\s+(.+?)(?:\s+(?:in|em|para)\s+(fahrenheit|celsius|kelvin|°f|°c|f|c|k))?$").expect("temperature pattern"));

pub fn parse_weather(text: &str) -> Option<WeatherQuery> {
    let text = super::math::unaccent(&text.to_lowercase());
    for pattern in FEELS.iter() {
        if let Some(caps) = pattern.captures(&text) {
            return Some(WeatherQuery { place: Some(caps[1].to_owned()), hours: 0, days: 0, kind: WeatherKind::FeelsLike });
        }
    }
    if let Some(caps) = TEMPERATURE.captures(&text) {
        let unit = caps.get(2).map(|u| match u.as_str() {
            "fahrenheit" | "°f" | "f" => "°F",
            "kelvin" | "k" => "K",
            _ => "°C",
        });
        return Some(WeatherQuery { place: Some(caps[1].to_owned()), hours: 0, days: 0, kind: WeatherKind::Temperature(unit) });
    }
    let caps = WEATHER.captures(&text)?;
    let mut rest = caps[1].to_owned();
    if !rest.is_empty() && !rest.starts_with(' ') {
        return None;
    }
    let (mut hours, mut days) = (0, 0);
    if let Some(m) = WHEN_HOURS.captures(&rest) {
        let count: i64 = m[1].parse().ok()?;
        if m[2].starts_with('h') {
            hours = count;
        } else {
            days = count;
        }
        rest = rest.replacen(&m[0], " ", 1);
    }
    if let Some(m) = WHEN_DAY.captures(&rest) {
        if matches!(&m[1], "tomorrow" | "amanha") {
            days = 1;
        }
        rest = rest.replacen(&m[0], " ", 1);
    }
    let rest = rest.split_whitespace().collect::<Vec<_>>().join(" ");
    let place = match rest.as_str() {
        "" => None,
        other => Some(WHERE.captures(other).map_or(other.to_owned(), |c| c[1].to_owned())),
    };
    Some(WeatherQuery { place, hours: hours.min(7 * 24), days: days.min(7), kind: WeatherKind::Card })
}

pub fn condition(code: i64, day: bool) -> (&'static str, &'static str) {
    match code {
        0 => ("Clear", if day { "☀️" } else { "🌙" }),
        1 => ("Mostly Clear", if day { "🌤️" } else { "🌙" }),
        2 => ("Partly Cloudy", "⛅"),
        3 => ("Cloudy", "☁️"),
        45 | 48 => ("Foggy", "🌫️"),
        51 | 53 | 55 => ("Drizzle", "🌦️"),
        56 | 57 => ("Freezing Drizzle", "🌧️"),
        61 => ("Light Rain", "🌧️"),
        63 => ("Rain", "🌧️"),
        65 => ("Heavy Rain", "🌧️"),
        66 | 67 => ("Freezing Rain", "🌧️"),
        71 => ("Light Snow", "🌨️"),
        73 => ("Snow", "🌨️"),
        75 => ("Heavy Snow", "🌨️"),
        77 => ("Snow Grains", "🌨️"),
        80 | 81 => ("Showers", "🌦️"),
        82 => ("Heavy Showers", "🌧️"),
        85 | 86 => ("Snow Showers", "🌨️"),
        95 => ("Thunderstorm", "⛈️"),
        96 | 99 => ("Thunderstorm with Hail", "⛈️"),
        _ => ("Unknown", "🌡️"),
    }
}

#[derive(Debug, PartialEq)]
pub struct Reading {
    pub temperature: f64,
    pub feels_like: f64,
    pub code: i64,
    pub day: bool,
}

pub fn hourly_index(forecast: &Value, at_s: f64) -> Option<usize> {
    let times = forecast["hourly"]["time"].as_array()?;
    times.iter().rposition(|t| t.as_f64().is_some_and(|t| t <= at_s))
}

pub fn reading_at(forecast: &Value, hours: i64, now_s: f64) -> Option<Reading> {
    if hours == 0 {
        let c = &forecast["current"];
        return Some(Reading { temperature: c["temperature_2m"].as_f64()?, feels_like: c["apparent_temperature"].as_f64()?, code: c["weather_code"].as_i64()?, day: c["is_day"].as_i64() != Some(0) });
    }
    let i = hourly_index(forecast, now_s + hours as f64 * 3600.0)?;
    let h = &forecast["hourly"];
    Some(Reading { temperature: h["temperature_2m"][i].as_f64()?, feels_like: h["apparent_temperature"][i].as_f64()?, code: h["weather_code"][i].as_i64()?, day: h["is_day"][i].as_i64() != Some(0) })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_weather_queries() {
        let q = parse_weather("weather in Guaruja").unwrap();
        assert_eq!((q.place.as_deref(), q.hours, q.days, q.kind), (Some("guaruja"), 0, 0, WeatherKind::Card));
        let q = parse_weather("weather tomorrow").unwrap();
        assert_eq!((q.place, q.days), (None, 1));
        let q = parse_weather("weather in 4 hours").unwrap();
        assert_eq!((q.place, q.hours), (None, 4));
        let q = parse_weather("weather tomorrow in Sao Paulo").unwrap();
        assert_eq!((q.place.as_deref(), q.days), (Some("sao paulo"), 1));
        assert_eq!(parse_weather("London feels like").unwrap().kind, WeatherKind::FeelsLike);
        let q = parse_weather("temperature in Paris in fahrenheit").unwrap();
        assert_eq!((q.place.as_deref(), q.kind), (Some("paris"), WeatherKind::Temperature(Some("°F"))));
        assert!(parse_weather("weatherman").is_none());
    }

    #[test]
    fn parses_flights_and_airports() {
        assert_eq!(parse_flight("flight ba117"), Some(("BA".into(), "117".into())));
        assert_eq!(parse_flight("flight baw117"), Some(("BAW".into(), "117".into())));
        assert_eq!(parse_flight("flight 117"), None);
        assert_eq!(short_airport("London Heathrow Airport", "London"), "Heathrow");
        assert_eq!(short_airport("John F Kennedy International Airport", "New York"), "John F Kennedy");
    }

    #[test]
    #[ignore = "network"]
    fn looks_up_a_real_flight() {
        let value = flight_blocking("BA", "117").unwrap();
        assert_eq!(value["route"]["origin"]["iata_code"], "LHR");
        assert!(value["zone"].as_str().is_some());
    }

    #[test]
    fn estimates_arrival_from_live_position() {
        let at = |lat, lon| Airport { name: String::new(), city: String::new(), country: String::new(), code: String::new(), lat, lon };
        let mut f = Flight { callsign: "X".into(), number: "X1".into(), airline: String::new(), origin: at(0.0, 0.0), destination: at(0.0, 1.0), destination_zone: None, live: Some((0.0, 0.0, 450.0, false)) };
        let eta = flight_eta_ms(&f, 0.0).unwrap();
        assert!((eta / 60_000.0 - 8.0).abs() < 0.5, "{}", eta / 60_000.0);
        f.live = Some((0.0, 0.0, 0.0, true));
        assert_eq!(flight_eta_ms(&f, 0.0), None);
    }

    #[test]
    fn reads_current_and_hourly_weather() {
        let forecast = serde_json::json!({
            "current": { "temperature_2m": 79.0, "apparent_temperature": 82.0, "weather_code": 2, "is_day": 1 },
            "hourly": { "time": [0, 3600, 7200], "temperature_2m": [70.0, 71.0, 72.0], "apparent_temperature": [70.0, 71.0, 73.0], "weather_code": [0, 3, 61], "is_day": [0, 1, 1] }
        });
        assert_eq!(reading_at(&forecast, 0, 0.0).unwrap().temperature, 79.0);
        assert_eq!(reading_at(&forecast, 2, 100.0).unwrap().code, 61);
        assert_eq!(condition(2, true), ("Partly Cloudy", "⛅"));
    }
}
