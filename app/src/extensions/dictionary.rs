use super::command;
use super::web::encode_query;
use crate::extension::{frontmost_app, paste_title, Action, Command, Empty, Extension, Host, Icon, Item, Load, Page, Request, Section, Shortcut, Toast, Tone, View, ViewFactory};
use crate::http::Failure;
use crate::theme;
use egui::{Key, RichText};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::cell::Cell;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::LazyLock;
use std::time::{Duration, Instant};

const ICON: Icon = Icon::Tile("book-open", Tone::Orange);
const SUGGEST_URL: &str = "https://api.datamuse.com";
const DEFINE_URL: &str = "https://api.dictionaryapi.dev/api/v2/entries/en/";
const WIKTIONARY_URL: &str = "https://pt.wiktionary.org";
const LIMIT: usize = 8;
const RECENT_LIMIT: usize = 100;
const SNIPPET_LIMIT: usize = 120;
const DEBOUNCE: Duration = Duration::from_millis(200);
const ATTEMPTS: u32 = 3;
static VERSION: AtomicU64 = AtomicU64::new(0);

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Suggestion {
    pub word: String,
    #[serde(default)]
    pub definition: String,
    #[serde(default)]
    pub is_guess: bool,
}

#[derive(Clone, PartialEq, Debug, Default)]
pub struct Sense {
    pub definition: String,
    pub example: Option<String>,
}

#[derive(Clone, PartialEq, Debug, Default)]
pub struct Meaning {
    pub part_of_speech: String,
    pub senses: Vec<Sense>,
    pub synonyms: Vec<String>,
}

#[derive(Clone, PartialEq, Debug, Default)]
pub struct Entry {
    pub word: String,
    pub phonetic: String,
    pub meanings: Vec<Meaning>,
    pub source_url: Option<String>,
}

#[derive(Serialize, Deserialize, Default, Clone)]
struct Data {
    #[serde(default)]
    recent: Vec<Suggestion>,
    #[serde(default)]
    language: Option<String>,
}

fn path() -> std::path::PathBuf {
    crate::data_dir().join("dictionary.json")
}

fn load() -> Data {
    crate::store::read_json(&path())
}

fn save_recent(recent: Vec<Suggestion>) -> Result<(), String> {
    let data = Data { recent, ..load() };
    crate::store::write_json(&path(), &data).map_err(|e| e.to_string())?;
    VERSION.fetch_add(1, Ordering::SeqCst);
    Ok(())
}

pub fn dictionary_language() -> String {
    load().language.unwrap_or_else(|| "en".into())
}

pub fn set_dictionary_language(language: &str) -> Result<(), String> {
    let data = Data { language: Some(language.to_owned()), ..load() };
    crate::store::write_json(&path(), &data).map_err(|e| e.to_string())?;
    VERSION.fetch_add(1, Ordering::SeqCst);
    Ok(())
}

fn portuguese() -> bool {
    load().language.as_deref() == Some("pt")
}

fn snippet(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.chars().count() > SNIPPET_LIMIT {
        format!("{}…", trimmed.chars().take(SNIPPET_LIMIT).collect::<String>())
    } else {
        trimmed.to_owned()
    }
}

fn exact_first(query: &str, mut words: Vec<Suggestion>) -> Vec<Suggestion> {
    let wanted = query.trim().to_lowercase();
    if let Some(i) = words.iter().position(|w| w.word.to_lowercase() == wanted).filter(|i| *i > 0) {
        let exact = words.remove(i);
        words.insert(0, exact);
    }
    words
}

pub fn parse_suggestions(query: &str, body: &Value, is_guess: bool) -> Vec<Suggestion> {
    let Some(list) = body.as_array() else { return Vec::new() };
    let wanted = query.trim().to_lowercase();
    let words = list
        .iter()
        .filter_map(|w| {
            let word = w.get("word")?.as_str()?;
            if is_guess && word.to_lowercase() == wanted {
                return None;
            }
            let def = w.get("defs").and_then(|d| d.get(0)).and_then(Value::as_str).map(|d| d.split('\t').skip(1).collect::<Vec<_>>().join("\t")).unwrap_or_default();
            Some(Suggestion { word: word.into(), definition: snippet(&def), is_guess })
        })
        .collect();
    exact_first(query, words)
}

pub fn suggestion_url(query: &str, limit: usize) -> String {
    format!("{SUGGEST_URL}/words?sp={}&md=d&max={limit}", encode_query(&format!("{}*", query.trim())).replace("%2A", "*"))
}

pub fn guess_url(query: &str, limit: usize) -> String {
    format!("{SUGGEST_URL}/sug?s={}&max={limit}", encode_query(query.trim()))
}

fn form_encode(text: &str) -> String {
    text.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'*' | b'-' | b'.' | b'_' => (b as char).to_string(),
            b' ' => "+".into(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

pub fn wiktionary_url(params: &[(&str, &str)]) -> String {
    let all = params.iter().copied().chain([("format", "json"), ("formatversion", "2"), ("origin", "*")]);
    format!("{WIKTIONARY_URL}/w/api.php?{}", all.map(|(k, v)| format!("{}={}", form_encode(k), form_encode(v))).collect::<Vec<_>>().join("&"))
}

pub fn parse_entries(body: &Value) -> Vec<Entry> {
    let Some(list) = body.as_array() else { return Vec::new() };
    let text = |v: &Value, key: &str| v.get(key).and_then(Value::as_str).unwrap_or("").to_owned();
    let strings = |v: &Value, key: &str| v.get(key).and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_str).map(String::from).collect()).unwrap_or_default();
    list.iter()
        .map(|e| {
            let phonetic = Some(text(e, "phonetic")).filter(|p| !p.is_empty()).or_else(|| e.get("phonetics").and_then(Value::as_array).and_then(|p| p.iter().map(|x| text(x, "text")).find(|t| !t.is_empty()))).unwrap_or_default();
            let meanings = e
                .get("meanings")
                .and_then(Value::as_array)
                .map(|m| {
                    m.iter()
                        .map(|m| Meaning {
                            part_of_speech: text(m, "partOfSpeech"),
                            senses: m.get("definitions").and_then(Value::as_array).map(|d| d.iter().filter(|d| !text(d, "definition").is_empty()).map(|d| Sense { definition: text(d, "definition"), example: Some(text(d, "example")).filter(|x| !x.is_empty()) }).collect()).unwrap_or_default(),
                            synonyms: strings(m, "synonyms"),
                        })
                        .collect()
                })
                .unwrap_or_default();
            Entry { word: text(e, "word"), phonetic, meanings, source_url: e.get("sourceUrls").and_then(|s| s.get(0)).and_then(Value::as_str).map(String::from) }
        })
        .collect()
}

pub fn definition_text(entries: &[Entry]) -> String {
    entries
        .iter()
        .map(|e| {
            let head = [e.word.as_str(), e.phonetic.as_str()].iter().filter(|s| !s.is_empty()).copied().collect::<Vec<_>>().join(" ");
            let meanings = e.meanings.iter().map(|m| std::iter::once(m.part_of_speech.clone()).chain(m.senses.iter().enumerate().map(|(i, s)| format!("{}. {}{}", i + 1, s.definition, s.example.as_ref().map_or(String::new(), |x| format!(" \"{x}\""))))).collect::<Vec<_>>().join("\n"));
            std::iter::once(head).chain(meanings).collect::<Vec<_>>().join("\n\n")
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

pub fn remember(recent: &[Suggestion], word: &Suggestion) -> Vec<Suggestion> {
    if recent.first().is_some_and(|w| w.word == word.word) {
        return recent.to_vec();
    }
    std::iter::once(Suggestion { is_guess: false, ..word.clone() }).chain(recent.iter().filter(|w| w.word != word.word).cloned()).take(RECENT_LIMIT).collect()
}

fn titles(query: &str, list: Vec<String>, is_guess: bool) -> Vec<Suggestion> {
    let wanted = query.trim().to_lowercase();
    let mut seen = std::collections::HashSet::new();
    let words = list.into_iter().filter(|w| seen.insert(w.clone())).filter(|w| !is_guess || w.to_lowercase() != wanted).map(|word| Suggestion { word, definition: String::new(), is_guess }).collect();
    exact_first(query, words)
}

pub fn parse_open_search(query: &str, body: &Value) -> Vec<Suggestion> {
    match body.get(1).and_then(Value::as_array) {
        Some(list) if body.is_array() => titles(query, list.iter().filter_map(Value::as_str).map(String::from).collect(), false),
        _ => Vec::new(),
    }
}

pub fn parse_search(query: &str, body: &Value) -> Vec<Suggestion> {
    let list = body.pointer("/query/search").and_then(Value::as_array).map(|l| l.iter().filter_map(|r| r.get("title")?.as_str().map(String::from)).collect()).unwrap_or_default();
    titles(query, list, true)
}

static ESCOPO: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\{\{escopo2?\|(?:[a-z-]+\|)?([^}|]+)[^}]*\}\}").unwrap());
static TEMPLATE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\{\{[^{}]*\}\}").unwrap());
static PIPED_LINK: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\[\[[^\]|]*\|([^\]]*)\]\]").unwrap());
static LINK: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\[\[([^\]]*)\]\]").unwrap());
static QUOTES: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"'{2,}").unwrap());
static TAG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<[^>]+>").unwrap());
static SPACE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+").unwrap());
static NEXT_LANGUAGE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?m)^=[^=]").unwrap());
static HEADING: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(=+)\s*(.*?)\s*=+$").unwrap());
static SKIPPED: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(Pronúncia|Etimologia|Ver também|Tradução|Referências|Anagramas|Formas alternativas|Expressões)").unwrap());
static LINK_TARGET: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\[\[(?:[^\]|]*\|)?([^\]]+)\]\]").unwrap());
static AFI: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\{\{AFI\|([^}|]+)").unwrap());
static EXAMPLE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^#[:*]+").unwrap());

fn clean_wikitext(text: &str) -> String {
    let text = ESCOPO.replace_all(text, "($1)");
    let text = TEMPLATE.replace_all(&text, "");
    let text = PIPED_LINK.replace_all(&text, "$1");
    let text = LINK.replace_all(&text, "$1");
    let text = QUOTES.replace_all(&text, "");
    let text = TAG.replace_all(&text, "");
    SPACE.replace_all(&text, " ").trim().to_owned()
}

pub fn parse_wikitext(word: &str, wikitext: &str) -> Vec<Entry> {
    const MARK: &str = "={{-pt-}}=";
    let Some(start) = wikitext.find(MARK) else { return Vec::new() };
    let rest = &wikitext[start + MARK.len()..];
    let section = NEXT_LANGUAGE.find(rest).map_or(rest, |m| &rest[..m.start()]);
    let mut meanings: Vec<Meaning> = Vec::new();
    let mut current: Option<usize> = None;
    let mut synonyms = false;
    for raw in section.split('\n') {
        let line = raw.trim();
        if let Some(heading) = HEADING.captures(line) {
            let title = clean_wikitext(&heading[2]);
            synonyms = heading[1].len() == 3 && title.starts_with("Sinônimo");
            if heading[1].len() == 2 {
                current = if SKIPPED.is_match(&title) {
                    None
                } else {
                    meanings.push(Meaning { part_of_speech: title.to_lowercase(), ..Default::default() });
                    Some(meanings.len() - 1)
                };
            }
            continue;
        }
        let Some(m) = current.map(|i| &mut meanings[i]) else { continue };
        if line.starts_with("#:") || line.starts_with("#*") {
            let example = clean_wikitext(&EXAMPLE.replace(line, ""));
            if let Some(last) = m.senses.last_mut().filter(|l| l.example.is_none()) {
                if !example.is_empty() {
                    last.example = Some(example);
                }
            }
        } else if line.starts_with('#') {
            let definition = clean_wikitext(line.trim_start_matches('#'));
            if !definition.is_empty() {
                m.senses.push(Sense { definition, example: None });
            }
        } else if synonyms && line.starts_with('*') {
            for link in LINK_TARGET.captures_iter(line) {
                if !m.synonyms.contains(&link[1].to_string()) {
                    m.synonyms.push(link[1].to_string());
                }
            }
        }
    }
    let phonetic = AFI.captures(section).map(|c| c[1].trim().to_owned()).unwrap_or_default();
    let kept: Vec<Meaning> = meanings.into_iter().filter(|m| !m.senses.is_empty()).collect();
    if kept.is_empty() {
        return Vec::new();
    }
    vec![Entry { word: word.into(), phonetic, meanings: kept, source_url: Some(format!("{WIKTIONARY_URL}/wiki/{}", encode_query(word))) }]
}

fn get_json(url: &str) -> Result<Value, String> {
    let mut last = String::new();
    for attempt in 0..ATTEMPTS {
        if attempt > 0 {
            std::thread::sleep(Duration::from_millis(600));
        }
        match crate::http::get_json(url) {
            Ok(value) => return Ok(value.unwrap_or(Value::Array(Vec::new()))),
            Err(Failure::Status(status)) => last = format!("The dictionary service returned an error ({status})."),
            Err(Failure::Offline) => last = "The dictionary service didn't respond. Check your connection and try again.".into(),
            Err(Failure::Body(e)) => last = e,
        }
    }
    Err(last)
}

fn suggest(text: &str, pt: bool) -> Result<Vec<Suggestion>, String> {
    let limit = LIMIT.to_string();
    if pt {
        let found = parse_open_search(text, &get_json(&wiktionary_url(&[("action", "opensearch"), ("search", text.trim()), ("limit", &limit), ("namespace", "0")]))?);
        return if found.is_empty() { Ok(parse_search(text, &get_json(&wiktionary_url(&[("action", "query"), ("list", "search"), ("srsearch", text.trim()), ("srlimit", &limit)]))?)) } else { Ok(found) };
    }
    let found = parse_suggestions(text, &get_json(&suggestion_url(text, LIMIT))?, false);
    if found.is_empty() {
        Ok(parse_suggestions(text, &get_json(&guess_url(text, LIMIT))?, true))
    } else {
        Ok(found)
    }
}

fn define(word: &str, pt: bool) -> Result<Vec<Entry>, String> {
    if !pt {
        return Ok(parse_entries(&get_json(&format!("{DEFINE_URL}{}", encode_query(word.trim())))?));
    }
    let body = get_json(&wiktionary_url(&[("action", "parse"), ("page", word), ("prop", "wikitext"), ("redirects", "1")]))?;
    let title = body.pointer("/parse/title").and_then(Value::as_str).unwrap_or(word);
    Ok(body.pointer("/parse/wikitext").and_then(Value::as_str).filter(|w| !w.is_empty()).map(|w| parse_wikitext(title, w)).unwrap_or_default())
}

fn copy(text: String, title: &'static str) -> impl Fn(&mut Host) -> Result<(), String> {
    move |host| {
        host.copy_and_close(&text, title);
        Ok(())
    }
}

struct Definition {
    word: String,
    pt: bool,
    app: Option<String>,
    ctx: egui::Context,
    load: Load<Vec<Entry>>,
    retry: Rc<Cell<bool>>,
}

impl Definition {
    fn factory(word: String, app: Option<String>) -> ViewFactory {
        Rc::new(move |ctx| {
            let pt = portuguese();
            let owned = word.clone();
            Box::new(Definition { word: word.clone(), pt, app: app.clone(), ctx: ctx.clone(), load: Load::start(ctx, move || define(&owned, pt)), retry: Rc::new(Cell::new(false)) })
        })
    }
}

impl View for Definition {
    fn placeholder(&self) -> String {
        String::new()
    }

    fn footer(&self) -> (String, Icon) {
        (self.word.clone(), ICON)
    }

    fn sections(&mut self) -> Vec<Section> {
        Vec::new()
    }

    fn page(&mut self) -> Option<crate::extension::Page> {
        if self.retry.replace(false) {
            let (word, pt) = (self.word.clone(), self.pt);
            self.load = Load::start(&self.ctx, move || define(&word, pt));
        }
        let state = self.load.get();
        let entries = state.as_ref().and_then(|r| r.as_ref().ok()).cloned().unwrap_or_default();
        let mut primary = vec![
            Action::new("copy-word", "Copy Word to Clipboard", copy(self.word.clone(), "Copied word to clipboard")).icon("clipboard").shortcut(Shortcut::key(Key::C).ctrl().shift()),
            {
                let word = self.word.clone();
                Action::new("paste", paste_title(&self.app), move |host| host.paste(&word)).icon("clipboard-paste").shortcut(Shortcut::key(Key::Enter).ctrl())
            },
        ];
        if !entries.is_empty() {
            primary.push(Action::new("copy-definition", "Copy Definition", copy(definition_text(&entries), "Copied definition")).icon("book-text").shortcut(Shortcut::key(Key::C).ctrl().alt()));
        }
        let mut actions = vec![primary];
        if matches!(state, Some(Err(_))) {
            let retry = self.retry.clone();
            actions.push(vec![Action::new("retry", "Try Again", move |_| {
                retry.set(true);
                Ok(())
            })
            .icon("rotate-cw")
            .shortcut(Shortcut::key(Key::R).ctrl())
            .keep_open()]);
        }
        if let Some(url) = entries.iter().find_map(|e| e.source_url.clone()) {
            actions.push(vec![Action::new("open-source", "Open in Browser", move |_| crate::apps::open_url(&url, None)).icon("external-link").shortcut(Shortcut::key(Key::O).ctrl())]);
        }
        Some(Page { title: self.word.clone(), dropdowns: Vec::new(), actions, enter_runs_primary: true, backspace_pops: true })
    }

    fn page_ui(&mut self, ui: &mut egui::Ui, rect: egui::Rect) {
        let Some(state) = self.load.get() else { return };
        let entries = match state {
            Ok(entries) if !entries.is_empty() => entries,
            other => {
                let (title, description) = match other {
                    Err(e) => ("Couldn't load the definition".to_string(), e),
                    _ => ("No Definitions Found".to_string(), format!("The dictionary has no entry for \"{}\". Copy the word or try another spelling.", self.word)),
                };
                let center = rect.center();
                let title_rect = ui.painter().text(center - egui::vec2(0.0, 10.0), egui::Align2::CENTER_CENTER, title, theme::weighted(13.0, theme::MEDIUM), theme::FG());
                let galley = crate::root::wrapped(ui, &description, theme::font(13.0), theme::FG_60(), 420.0);
                ui.painter().galley(egui::pos2(center.x - galley.size().x / 2.0, title_rect.bottom() + 4.0), galley, theme::FG_60());
                return;
            }
        };
        egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
            egui::Frame::NONE.inner_margin(egui::Margin::symmetric(24, 16)).show(ui, |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                for entry in &entries {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&entry.word).font(theme::weighted(24.0, theme::SEMIBOLD)).color(theme::FG()));
                        if !entry.phonetic.is_empty() {
                            ui.add_space(12.0);
                            ui.label(RichText::new(&entry.phonetic).font(theme::font(15.0)).color(theme::FG_60()));
                        }
                    });
                    for meaning in &entry.meanings {
                        ui.add_space(16.0);
                        ui.label(RichText::new(&meaning.part_of_speech).font(theme::weighted(12.0, theme::SEMIBOLD)).italics().color(theme::FG_60()));
                        for (i, sense) in meaning.senses.iter().enumerate() {
                            ui.add_space(8.0);
                            ui.horizontal_top(|ui| {
                                ui.add_sized([20.0, 16.0], egui::Label::new(RichText::new(format!("{}.", i + 1)).font(theme::font(13.0)).color(theme::FG_40())));
                                ui.vertical(|ui| {
                                    ui.add(egui::Label::new(RichText::new(&sense.definition).font(theme::font(13.0)).color(theme::FG())).wrap());
                                    if let Some(example) = &sense.example {
                                        ui.add_space(2.0);
                                        ui.add(egui::Label::new(RichText::new(format!("“{example}”")).font(theme::font(13.0)).italics().color(theme::FG_60())).wrap());
                                    }
                                });
                            });
                        }
                        if !meaning.synonyms.is_empty() {
                            ui.add_space(8.0);
                            let list = meaning.synonyms.iter().take(8).cloned().collect::<Vec<_>>().join(", ");
                            let mut job = egui::text::LayoutJob::default();
                            job.append("Synonyms: ", 0.0, egui::TextFormat { font_id: theme::weighted(12.0, theme::MEDIUM), color: theme::FG_60(), ..Default::default() });
                            job.append(&list, 0.0, egui::TextFormat { font_id: theme::font(12.0), color: theme::FG_60(), ..Default::default() });
                            ui.add(egui::Label::new(job).wrap());
                        }
                    }
                    ui.add_space(24.0);
                }
            });
        });
    }
}

struct DefineWord {
    ctx: egui::Context,
    pt: bool,
    query: String,
    changed: Instant,
    recent: Vec<Suggestion>,
    version: u64,
    search: Option<(String, Load<Vec<Suggestion>>)>,
    results: Vec<Suggestion>,
    error: Option<String>,
    app: Option<String>,
}

impl DefineWord {
    fn item(&self, s: &Suggestion, prefix: &str, searching: bool) -> Item {
        let shown = s.clone();
        let details = Definition::factory(s.word.clone(), self.app.clone());
        let mut actions = vec![vec![
            Action::new("details", "Show Details", move |host| {
                save_recent(remember(&load().recent, &shown))?;
                host.requests.push(Request::Push(details.clone()));
                Ok(())
            })
            .icon("panel-right")
            .keep_open(),
            {
                let word = s.word.clone();
                Action::new("paste", paste_title(&self.app), move |host| host.paste(&word)).icon("clipboard-paste").shortcut(Shortcut::key(Key::Enter).ctrl().shift())
            },
            Action::new("copy", "Copy Word to Clipboard", copy(s.word.clone(), "Copied word to clipboard")).icon("clipboard").shortcut(Shortcut::key(Key::C).ctrl().shift()),
        ]];
        if !searching {
            let word = s.word.clone();
            actions.push(vec![
                Action::new("remove", "Remove from Recent", move |host| {
                    save_recent(load().recent.into_iter().filter(|w| w.word != word).collect())?;
                    host.toast = Some(Toast::Success("Removed from recent words".into()));
                    Ok(())
                })
                .icon("trash-2")
                .destructive()
                .keep_open()
                .shortcut(Shortcut::key(Key::D).ctrl()),
                Action::new("clear", "Clear All Recent Words", |host| {
                    save_recent(Vec::new())?;
                    host.toast = Some(Toast::Success("Cleared recent words".into()));
                    Ok(())
                })
                .icon("trash-2")
                .destructive()
                .keep_open()
                .shortcut(Shortcut::key(Key::D).ctrl().alt()),
            ]);
        }
        Item { id: format!("{prefix}:{}", s.word), title: s.word.clone(), subtitle: Some(s.definition.clone()).filter(|d| !d.is_empty()), icon: Icon::Glyph("book-open"), accessories: Vec::new(), keywords: Vec::new(), actions }
    }

    fn poll(&mut self) {
        let text = self.query.trim().to_owned();
        if text.is_empty() {
            self.search = None;
            self.results.clear();
            self.error = None;
            return;
        }
        if let Some((query, load)) = &self.search {
            if *query == text {
                if let Some(result) = load.get() {
                    match result {
                        Ok(list) => {
                            self.results = list;
                            self.error = None;
                        }
                        Err(e) => {
                            self.results.clear();
                            self.error = Some(e);
                        }
                    }
                }
                return;
            }
        }
        let left = DEBOUNCE.saturating_sub(self.changed.elapsed());
        if left > Duration::ZERO {
            self.ctx.request_repaint_after(left);
            return;
        }
        let (owned, pt) = (text.clone(), self.pt);
        self.search = Some((text, Load::start(&self.ctx, move || suggest(&owned, pt))));
    }
}

impl View for DefineWord {
    fn placeholder(&self) -> String {
        "Search word…".into()
    }

    fn footer(&self) -> (String, Icon) {
        ("Define Word".into(), ICON)
    }

    fn filters(&self) -> bool {
        false
    }

    fn set_query(&mut self, query: &str) {
        if query != self.query {
            self.query = query.to_owned();
            self.changed = Instant::now();
        }
    }

    fn loading(&self) -> bool {
        !self.query.trim().is_empty() && self.search.as_ref().is_none_or(|(q, l)| *q != self.query.trim() || l.get().is_none())
    }

    fn empty(&self) -> Empty {
        if let Some(error) = &self.error {
            return Empty { title: "Failed to get dictionary suggestions".into(), description: Some(error.clone()) };
        }
        if self.query.trim().is_empty() {
            let description = if self.pt { "Portuguese definitions, pronunciations and examples from Wiktionary." } else { "English definitions, pronunciations and examples." };
            Empty { title: "Type a word to look up in dictionary".into(), description: Some(description.into()) }
        } else {
            Empty { title: "No Results Found".into(), description: Some("Check the spelling or try another word.".into()) }
        }
    }

    fn sections(&mut self) -> Vec<Section> {
        let version = VERSION.load(Ordering::SeqCst);
        if version != self.version {
            self.version = version;
            self.recent = load().recent;
        }
        self.poll();
        if self.query.trim().is_empty() {
            let items: Vec<Item> = self.recent.iter().map(|s| self.item(s, "recent", false)).collect();
            return if items.is_empty() { Vec::new() } else { vec![Section { title: "Recent Words".into(), items }] };
        }
        let showing = self.search.as_ref().is_some_and(|(q, _)| *q == self.query.trim());
        if !showing {
            return Vec::new();
        }
        let found: Vec<Item> = self.results.iter().filter(|r| !r.is_guess).map(|s| self.item(s, "result", true)).collect();
        let guesses: Vec<Item> = self.results.iter().filter(|r| r.is_guess).map(|s| self.item(s, "guess", true)).collect();
        [("Results", found), ("Did you mean?", guesses)].into_iter().filter(|s| !s.1.is_empty()).map(|(title, items)| Section { title: title.into(), items }).collect()
    }
}

pub struct Dictionary;

impl Extension for Dictionary {
    fn id(&self) -> &'static str {
        "dictionary"
    }

    fn commands(&self) -> Vec<Command> {
        let mut define = command(self.id(), "defineWord", "Define Word", ICON, "Command", "Open Command", |_| Ok(()));
        define.subtitle = Some("Dictionary".into());
        define.keywords = ["dictionary", "define", "lexicon", "thesaurus", "glossary", "spell", "meaning", "dicionário", "definir", "significado"].map(String::from).to_vec();
        define.view = Some(Rc::new(|ctx| {
            Box::new(DefineWord { ctx: ctx.clone(), pt: portuguese(), query: String::new(), changed: Instant::now(), recent: Vec::new(), version: u64::MAX, search: None, results: Vec::new(), error: None, app: frontmost_app() })
        }));
        vec![define]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn s(word: &str, definition: &str, is_guess: bool) -> Suggestion {
        Suggestion { word: word.into(), definition: definition.into(), is_guess }
    }

    #[test]
    fn reads_datamuse_words_and_puts_the_exact_word_first() {
        let body = json!([{ "word": "help", "defs": ["v\tTo provide assistance. "] }, { "word": "hel", "defs": ["n\tA word."] }, { "word": "helm" }]);
        assert_eq!(parse_suggestions("Hel", &body, false), [s("hel", "A word.", false), s("help", "To provide assistance.", false), s("helm", "", false)]);
        assert!(parse_suggestions("x", &json!({ "error": true }), false).is_empty());
    }

    #[test]
    fn drops_the_typed_word_from_guesses_and_shortens_long_definitions() {
        assert_eq!(parse_suggestions("helo", &json!([{ "word": "helo" }, { "word": "hello" }]), true), [s("hello", "", true)]);
        let long = json!([{ "word": "a", "defs": [format!("n\t{}", "x".repeat(130))] }]);
        assert_eq!(parse_suggestions("a", &long, false)[0].definition.chars().count(), 121);
    }

    #[test]
    fn encodes_the_query_in_the_request_urls() {
        assert_eq!(suggestion_url(" ice cream ", 8), "https://api.datamuse.com/words?sp=ice%20cream*&md=d&max=8");
        assert_eq!(guess_url("helo", 8), "https://api.datamuse.com/sug?s=helo&max=8");
        assert_eq!(wiktionary_url(&[("action", "parse"), ("page", "ação")]), "https://pt.wiktionary.org/w/api.php?action=parse&page=a%C3%A7%C3%A3o&format=json&formatversion=2&origin=*");
    }

    #[test]
    fn parses_dictionaryapi_entries_with_phonetics_and_examples() {
        let body = json!([{
            "word": "hello",
            "phonetics": [{ "audio": "" }, { "text": "/həˈləʊ/" }],
            "meanings": [{ "partOfSpeech": "noun", "definitions": [{ "definition": "A greeting.", "example": "she said hello" }, { "definition": "" }], "synonyms": ["greeting"], "antonyms": [] }],
            "sourceUrls": ["https://en.wiktionary.org/wiki/hello"]
        }]);
        let entries = parse_entries(&body);
        let sense = Sense { definition: "A greeting.".into(), example: Some("she said hello".into()) };
        assert_eq!(entries, [Entry { word: "hello".into(), phonetic: "/həˈləʊ/".into(), meanings: vec![Meaning { part_of_speech: "noun".into(), senses: vec![sense], synonyms: vec!["greeting".into()] }], source_url: Some("https://en.wiktionary.org/wiki/hello".into()) }]);
        assert_eq!(definition_text(&entries), "hello /həˈləʊ/\n\nnoun\n1. A greeting. \"she said hello\"");
        assert!(parse_entries(&json!({ "title": "No Definitions Found" })).is_empty());
    }

    #[test]
    fn keeps_recent_words_unique_and_newest_first() {
        let (a, b) = (s("a", "", false), s("b", "", true));
        assert_eq!(remember(&[a.clone(), b.clone()], &b), [s("b", "", false), a.clone()]);
        assert_eq!(remember(&[a.clone()], &a), [a]);
    }

    #[test]
    fn reads_portuguese_suggestions_from_wiktionary() {
        let words = |list: Vec<Suggestion>| list.into_iter().map(|w| w.word).collect::<Vec<_>>();
        assert_eq!(words(parse_open_search("Sauda", &json!(["sauda", ["saudação", "saudade", "sauda", "saudade"], [], []]))), ["sauda", "saudação", "saudade"]);
        assert_eq!(parse_search("acao", &json!({ "query": { "search": [{ "title": "acao" }, { "title": "ação" }] } })), [s("ação", "", true)]);
        assert!(parse_open_search("x", &json!({ "error": 1 })).is_empty());
    }

    #[test]
    fn parses_the_portuguese_section_of_a_wiktionary_entry() {
        let wikitext = [
            "={{-pt-}}=",
            "==Substantivo==",
            "{{flex.pt|fs=ação|fp=ações}}",
            "# efeito de [[atuar]]",
            "#: ''Foi uma '''ação''' rápida.''",
            "# {{escopo|pt|Economia}} [[papel]] que [[representar|representa]] [[parte]] do capital",
            "===Sinônimos===",
            "* De '''1''' (efeito de atuar): [[ato]], [[feito]]",
            "===Tradução===",
            "* {{t|en|action}}",
            "==Pronúncia==",
            "* AFI: {{AFI|/a.ˈsɐ̃w̃/}}",
            "# não é definição",
            "={{-es-}}=",
            "==Substantivo==",
            "# otra lengua",
        ]
        .join("\n");
        let senses = vec![Sense { definition: "efeito de atuar".into(), example: Some("Foi uma ação rápida.".into()) }, Sense { definition: "(Economia) papel que representa parte do capital".into(), example: None }];
        let expected = Entry { word: "ação".into(), phonetic: "/a.ˈsɐ̃w̃/".into(), meanings: vec![Meaning { part_of_speech: "substantivo".into(), senses, synonyms: vec!["ato".into(), "feito".into()] }], source_url: Some("https://pt.wiktionary.org/wiki/a%C3%A7%C3%A3o".into()) };
        assert_eq!(parse_wikitext("ação", &wikitext), [expected]);
        assert!(parse_wikitext("house", "={{-en-}}=\n==Noun==\n# a building").is_empty());
    }
}
