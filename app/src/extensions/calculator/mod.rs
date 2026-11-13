pub mod color;
pub mod history;
pub mod math;
pub mod time;
pub mod units;

use super::command;
use crate::extension::{frontmost_app, paste_title, Accessory, Action, Command, Empty, Extension, Host, Icon, Inline, Item, Request, Section, Shortcut, Side, Tone, View};
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
}

enum Computed {
    Color(color::ParsedColor),
    Answer(Answer),
}

static CURRENCY_HINT: LazyLock<[Regex; 3]> = LazyLock::new(|| [r"(?i)[a-z$€£¥₹₩₽₺₿]", r"\d", r"(?i)\s(?:in|to|as|para|em|como|pra|pro|=)\s"].map(|p| Regex::new(p).unwrap()));

fn money(amount: f64, code: &str) -> String {
    icu::format_money(amount, code, "en-US", if amount.abs() < 1.0 { 6 } else { 2 }).unwrap_or_else(|| format!("{} {code}", math::format_number(amount)))
}

fn currency_name(code: &str) -> String {
    math::capitalize(&icu::currency_name(code, time::locale()))
}

fn compute(ctx: &egui::Context, query: &str) -> Option<Computed> {
    let text = query.trim();
    if text.is_empty() {
        return None;
    }
    if let Some(parsed) = color::parse(text) {
        return Some(Computed::Color(parsed));
    }
    let answer = |kind, display: String, plain: String, input_label: String, label: String, note: Option<String>| Some(Computed::Answer(Answer { kind, input: text.to_owned(), input_label, display, plain, label, note }));
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
    let now = crate::store::now_ms() as f64;
    let locale = time::locale();
    let index = &*time::PLACES;
    if let Some((clock, from, to)) = time::parse_clock_query(text) {
        if let (Some(from), Some(to)) = (time::lookup(&from, index), time::lookup(&to, index)) {
            let instant = time::instant_for(time::zoned_day(now, &from.zone), clock, &from.zone);
            let when = icu::format_date(instant, locale, Style::None, Style::Short, Some(&to.zone));
            let day = icu::format_date(instant, locale, Style::Medium, Style::None, Some(&to.zone));
            return answer(Kind::DateTime, when.clone(), when, from.name, to.name, Some(day));
        }
    }
    if let Some(delta) = time::parse_relative(text) {
        let when = icu::format_date(now + delta, locale, Style::Full, Style::Short, None);
        return answer(Kind::DateTime, when.clone(), when, "Date".into(), "Date".into(), None);
    }
    let today = ((now + icu::local_offset_minutes(now) as f64 * 60_000.0) / 86_400_000.0).floor() as i64;
    if let Some(target) = time::parse_days_until(text).and_then(|t| time::parse_target_day(&t, today)) {
        let days = target - today;
        let label = format!("{} day{}", days.abs(), if days.abs() == 1 { "" } else { "s" });
        return answer(Kind::DateTime, label, days.to_string(), "Date".into(), "Date".into(), (days < 0).then(|| "Past".into()));
    }
    if let Some((from, to)) = time::parse_time_diff(text) {
        if let (Some(from), Some(to)) = (time::lookup(&from, index), time::lookup(&to, index)) {
            let diff = time::format_diff(icu::offset_minutes(now, &to.zone) - icu::offset_minutes(now, &from.zone));
            return answer(Kind::DateTime, diff.clone(), diff, "Time difference".into(), format!("{} → {}", from.name, to.name), None);
        }
    }
    if let Some(place) = time::parse_time_query(text, index) {
        let (when, offset) = time::format_time_in(now, &place.zone, locale);
        return answer(Kind::DateTime, when.clone(), when, place.name, offset, None);
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
        Computed::Color(_) => None,
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
                    input: Side { text: query.trim().into(), label: parsed.format.into(), swatch: None },
                    output: Side { text: String::new(), label: color::name(&parsed.rgba).into(), swatch: Some(swatch(&parsed.rgba)) },
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
                })
            }
            Computed::Answer(a) => {
                let (input, display, kind) = (a.input.clone(), a.display.clone(), a.kind);
                let save: Rc<dyn Fn()> = Rc::new(move || history::record(&input, &display, kind));
                let app = frontmost_app();
                let mut actions = copy_paste(&a.plain, &app, save.clone());
                actions.push(copy_question_and_answer(&a.input, &a.display, save));
                Some(Inline {
                    section: "Calculator".into(),
                    input: Side { text: a.input, label: a.input_label, swatch: None },
                    output: Side { text: a.display, label: a.label, swatch: None },
                    note: a.note,
                    actions: vec![actions],
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
