use super::command;
use crate::extension::{frontmost_app, paste_title, Action, Command, Dropdown, Extension, Host, Icon, Load, Page, Section, Shortcut, Tone, View};
use crate::http::Failure;
use crate::theme;
use egui::{pos2, vec2, Align2, Key, Rect, Sense, UiBuilder};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::{Duration, Instant};

const ICON: Icon = Icon::Tile("languages", Tone::Blue);
const ENDPOINT: &str = "https://translate.googleapis.com/translate_a/single";
const FALLBACK: &str = "https://clients5.google.com/translate_a/t";
const AUTO: &str = "auto";
const DEFAULT_TARGET: &str = "en";
const DEBOUNCE: Duration = Duration::from_millis(1000);
const ALIASES: [(&str, &str); 5] = [("zh", "zh-CN"), ("he", "iw"), ("jv", "jw"), ("fil", "tl"), ("nb", "no")];
pub const LANGUAGES: [(&str, &str); 133] = [
    ("af", "Afrikaans"),
    ("ak", "Akan"),
    ("sq", "Albanian"),
    ("am", "Amharic"),
    ("ar", "Arabic"),
    ("hy", "Armenian"),
    ("as", "Assamese"),
    ("ay", "Aymara"),
    ("az", "Azerbaijani"),
    ("bm", "Bambara"),
    ("eu", "Basque"),
    ("be", "Belarusian"),
    ("bn", "Bengali"),
    ("bho", "Bhojpuri"),
    ("bs", "Bosnian"),
    ("bg", "Bulgarian"),
    ("ca", "Catalan"),
    ("ceb", "Cebuano"),
    ("ny", "Chichewa"),
    ("zh-CN", "Chinese (Simplified)"),
    ("zh-TW", "Chinese (Traditional)"),
    ("co", "Corsican"),
    ("hr", "Croatian"),
    ("cs", "Czech"),
    ("da", "Danish"),
    ("dv", "Divehi"),
    ("doi", "Dogri"),
    ("nl", "Dutch"),
    ("en", "English"),
    ("eo", "Esperanto"),
    ("et", "Estonian"),
    ("ee", "Ewe"),
    ("tl", "Filipino"),
    ("fi", "Finnish"),
    ("fr", "French"),
    ("fy", "Frisian"),
    ("gl", "Galician"),
    ("lg", "Ganda"),
    ("ka", "Georgian"),
    ("de", "German"),
    ("gom", "Goan Konkani"),
    ("el", "Greek"),
    ("gn", "Guarani"),
    ("gu", "Gujarati"),
    ("ht", "Haitian Creole"),
    ("ha", "Hausa"),
    ("haw", "Hawaiian"),
    ("iw", "Hebrew"),
    ("hi", "Hindi"),
    ("hmn", "Hmong"),
    ("hu", "Hungarian"),
    ("is", "Icelandic"),
    ("ig", "Igbo"),
    ("ilo", "Iloko"),
    ("id", "Indonesian"),
    ("ga", "Irish"),
    ("it", "Italian"),
    ("ja", "Japanese"),
    ("jw", "Javanese"),
    ("kn", "Kannada"),
    ("kk", "Kazakh"),
    ("km", "Khmer"),
    ("rw", "Kinyarwanda"),
    ("ko", "Korean"),
    ("kri", "Krio"),
    ("ku", "Kurdish (Kurmanji)"),
    ("ckb", "Kurdish (Sorani)"),
    ("ky", "Kyrgyz"),
    ("lo", "Lao"),
    ("la", "Latin"),
    ("lv", "Latvian"),
    ("ln", "Lingala"),
    ("lt", "Lithuanian"),
    ("lb", "Luxembourgish"),
    ("mk", "Macedonian"),
    ("mai", "Maithili"),
    ("mg", "Malagasy"),
    ("ms", "Malay"),
    ("ml", "Malayalam"),
    ("mt", "Maltese"),
    ("mni-Mtei", "Manipuri (Meitei Mayek)"),
    ("mi", "Maori"),
    ("mr", "Marathi"),
    ("lus", "Mizo"),
    ("mn", "Mongolian"),
    ("my", "Myanmar (Burmese)"),
    ("ne", "Nepali"),
    ("nso", "Northern Sotho"),
    ("no", "Norwegian"),
    ("or", "Odia (Oriya)"),
    ("om", "Oromo"),
    ("ps", "Pashto"),
    ("fa", "Persian"),
    ("pl", "Polish"),
    ("pt", "Portuguese"),
    ("pa", "Punjabi"),
    ("qu", "Quechua"),
    ("ro", "Romanian"),
    ("ru", "Russian"),
    ("sm", "Samoan"),
    ("sa", "Sanskrit"),
    ("gd", "Scots Gaelic"),
    ("sr", "Serbian"),
    ("st", "Sesotho"),
    ("sn", "Shona"),
    ("sd", "Sindhi"),
    ("si", "Sinhala"),
    ("sk", "Slovak"),
    ("sl", "Slovenian"),
    ("so", "Somali"),
    ("es", "Spanish"),
    ("su", "Sundanese"),
    ("sw", "Swahili"),
    ("sv", "Swedish"),
    ("tg", "Tajik"),
    ("ta", "Tamil"),
    ("tt", "Tatar"),
    ("te", "Telugu"),
    ("th", "Thai"),
    ("ti", "Tigrinya"),
    ("ts", "Tsonga"),
    ("tr", "Turkish"),
    ("tk", "Turkmen"),
    ("uk", "Ukrainian"),
    ("ur", "Urdu"),
    ("ug", "Uyghur"),
    ("uz", "Uzbek"),
    ("vi", "Vietnamese"),
    ("cy", "Welsh"),
    ("xh", "Xhosa"),
    ("yi", "Yiddish"),
    ("yo", "Yoruba"),
    ("zu", "Zulu"),
];

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct Pair {
    pub source: String,
    pub target: String,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Translation {
    pub text: String,
    pub detected: Option<String>,
}

#[derive(Clone)]
struct Outcome {
    text: String,
    translated: String,
    pair: Pair,
    detected: Option<String>,
}

pub fn find_language(code: Option<&str>) -> Option<&'static str> {
    let code = code?;
    if let Some((value, _)) = LANGUAGES.iter().find(|(v, _)| v.eq_ignore_ascii_case(code)) {
        return Some(value);
    }
    let base = code.split('-').next().unwrap_or(code).to_lowercase();
    ALIASES.iter().find(|(a, _)| *a == base).map(|(_, v)| *v).or_else(|| LANGUAGES.iter().find(|(v, _)| *v == base).map(|(v, _)| *v))
}

pub fn language_label(code: &str) -> String {
    find_language(Some(code)).and_then(|v| LANGUAGES.iter().find(|(x, _)| *x == v)).map_or_else(|| code.to_owned(), |(_, l)| (*l).to_owned())
}

pub fn restore(stored: &Value) -> Pair {
    let text = |key: &str| stored.get(key).and_then(Value::as_str);
    let source = text("source").filter(|s| *s != AUTO).and_then(|s| find_language(Some(s)));
    let target = text("target").and_then(|s| find_language(Some(s)));
    Pair { source: source.unwrap_or(AUTO).into(), target: target.unwrap_or(DEFAULT_TARGET).into() }
}

pub fn same_language(pair: &Pair) -> bool {
    pair.source != AUTO && pair.source == pair.target
}

pub fn swap(pair: &Pair, detected: Option<&str>) -> Option<Pair> {
    let from = if pair.source == AUTO { find_language(detected)?.to_owned() } else { pair.source.clone() };
    Some(Pair { source: pair.target.clone(), target: from })
}

pub fn count_label(text: &str) -> String {
    let words = text.split_whitespace().count();
    let characters = text.chars().count();
    format!("{words} {} · {characters} {}", if words == 1 { "Word" } else { "Words" }, if characters == 1 { "Character" } else { "Characters" })
}

pub fn parse(body: &Value) -> Result<Translation, String> {
    let segments = body.get(0).and_then(Value::as_array).ok_or("No translation received")?;
    let text: String = segments.iter().filter_map(|s| s.get(0).and_then(Value::as_str)).collect();
    if text.is_empty() {
        return Err("No translation received".into());
    }
    Ok(Translation { text, detected: body.get(2).and_then(Value::as_str).map(str::to_owned) })
}

pub fn parse_fallback(body: &Value) -> Result<Translation, String> {
    let first = body.get(0).ok_or("No translation received")?;
    let (text, detected) = match first {
        Value::Array(pair) => (pair.first().and_then(Value::as_str), pair.get(1).and_then(Value::as_str)),
        other => (other.as_str(), None),
    };
    match text {
        Some(text) if !text.is_empty() => Ok(Translation { text: text.to_owned(), detected: detected.map(str::to_owned) }),
        _ => Err("No translation received".into()),
    }
}

fn translate(text: &str, pair: &Pair) -> Result<Translation, String> {
    let url = format!("{ENDPOINT}?client=gtx&sl={}&tl={}&dt=t", pair.source, pair.target);
    let fallback = || crate::http::post_form(&format!("{FALLBACK}?client=dict-chrome-ex&sl={}&tl={}", pair.source, pair.target), &[("q", text)]);
    match crate::http::post_form(&url, &[("q", text)]) {
        Ok(body) => parse(&body),
        Err(Failure::Status(429)) => match fallback() {
            Ok(body) => parse_fallback(&body),
            Err(_) => Err("Google is limiting translations right now. Wait a few seconds and try again.".into()),
        },
        Err(Failure::Status(status)) => Err(format!("Translation service returned {status}")),
        Err(Failure::Offline) => Err("The translation service didn't respond. Check your connection and try again.".into()),
        Err(Failure::Body(e)) => Err(e),
    }
}

fn path() -> std::path::PathBuf {
    crate::data_dir().join("translate.json")
}

enum Request {
    Swap,
    Choose(Pair),
}

struct Translate {
    ctx: egui::Context,
    pair: Pair,
    text: String,
    changed: Instant,
    outcome: Option<Outcome>,
    error: Option<String>,
    running: Option<(String, Pair, Load<Translation>)>,
    app: Option<String>,
    requests: Rc<RefCell<Vec<Request>>>,
    focus: Cell<bool>,
}

impl Translate {
    fn choose(&mut self, pair: Pair) {
        let _ = crate::store::write_json(&path(), &pair);
        self.pair = pair;
        self.error = None;
    }

    fn detected(&self) -> Option<String> {
        self.outcome.as_ref().filter(|o| self.pair.source == AUTO && o.pair.source == AUTO).and_then(|o| o.detected.clone())
    }

    fn swapped(&self) -> Option<Pair> {
        let ready = self.pair.source != AUTO || self.outcome.as_ref().is_some_and(|o| o.text == self.text);
        if ready {
            swap(&self.pair, self.detected().as_deref())
        } else {
            None
        }
    }

    fn translation(&self) -> String {
        self.outcome.as_ref().filter(|_| !self.text.trim().is_empty()).map(|o| o.translated.clone()).unwrap_or_default()
    }

    fn pending(&self) -> bool {
        !self.text.trim().is_empty() && !same_language(&self.pair) && self.error.is_none() && !self.outcome.as_ref().is_some_and(|o| o.text == self.text && o.pair == self.pair)
    }

    fn do_swap(&mut self) {
        let Some(swapped) = self.swapped() else { return };
        let translation = self.translation();
        if !translation.is_empty() {
            let original = self.outcome.as_ref().map(|o| o.text.clone()).unwrap_or_default();
            self.text = translation.clone();
            self.outcome = Some(Outcome { text: translation, translated: original, pair: swapped.clone(), detected: None });
        }
        self.choose(swapped);
        self.focus.set(true);
    }

    fn update(&mut self) {
        let requests = std::mem::take(&mut *self.requests.borrow_mut());
        for request in requests {
            match request {
                Request::Swap => self.do_swap(),
                Request::Choose(pair) => self.choose(pair),
            }
        }
        if self.text.trim().is_empty() {
            self.outcome = None;
            self.running = None;
            self.error = None;
            return;
        }
        if same_language(&self.pair) {
            self.outcome = Some(Outcome { text: self.text.clone(), translated: self.text.clone(), pair: self.pair.clone(), detected: None });
            return;
        }
        if !self.pending() {
            return;
        }
        if let Some((text, pair, load)) = &self.running {
            if *text == self.text && *pair == self.pair {
                match load.get() {
                    Some(Ok(t)) => self.outcome = Some(Outcome { text: text.clone(), translated: t.text, pair: pair.clone(), detected: t.detected }),
                    Some(Err(e)) => self.error = Some(e),
                    None => {}
                }
                return;
            }
        }
        let left = DEBOUNCE.saturating_sub(self.changed.elapsed());
        if left > Duration::ZERO {
            self.ctx.request_repaint_after(left);
            return;
        }
        let (text, pair) = (self.text.clone(), self.pair.clone());
        let (t, p) = (text.clone(), pair.clone());
        self.running = Some((text, pair, Load::start(&self.ctx, move || translate(&t, &p))));
    }

    fn sources(&self) -> Vec<(String, Icon)> {
        let auto = match self.detected() {
            Some(code) => format!("{} (Detected)", language_label(&code)),
            None => "Detect Language".into(),
        };
        std::iter::once((auto, Icon::Glyph("wand-sparkles"))).chain(targets()).collect()
    }
}

fn targets() -> impl Iterator<Item = (String, Icon)> {
    LANGUAGES.iter().map(|(_, label)| ((*label).to_owned(), Icon::Glyph("flag")))
}

fn copy(text: String, title: &'static str) -> impl Fn(&mut Host) -> Result<(), String> {
    move |host| {
        host.copy_and_close(&text, title);
        Ok(())
    }
}

impl View for Translate {
    fn placeholder(&self) -> String {
        String::new()
    }

    fn footer(&self) -> (String, Icon) {
        ("Translator".into(), ICON)
    }

    fn sections(&mut self) -> Vec<Section> {
        Vec::new()
    }

    fn page(&mut self) -> Option<Page> {
        self.update();
        let translation = self.translation();
        let mut primary = Vec::new();
        if !translation.is_empty() {
            primary.push(Action::new("copy", "Copy Translation", copy(translation.clone(), "Copied translation to clipboard")).icon("clipboard").shortcut(Shortcut::key(Key::Enter).ctrl()));
            let pasted = translation.clone();
            primary.push(Action::new("paste", paste_title(&self.app), move |host| host.paste(&pasted)).icon("clipboard-paste").shortcut(Shortcut::key(Key::Enter).ctrl().alt()));
        }
        if !self.text.is_empty() {
            primary.push(Action::new("copySource", "Copy Source Text", copy(self.text.clone(), "Copied source text to clipboard")).icon("clipboard").shortcut(Shortcut::key(Key::Enter).ctrl().shift()));
        }
        let mut languages = Vec::new();
        if self.swapped().is_some() {
            let requests = self.requests.clone();
            languages.push(
                Action::new("swap", "Swap Languages", move |_| {
                    requests.borrow_mut().push(Request::Swap);
                    Ok(())
                })
                .icon("arrow-right-left")
                .shortcut(Shortcut::key(Key::S).ctrl())
                .keep_open(),
            );
        }
        for (id, title, key, source) in [("source", "Change Source Language…", Shortcut::key(Key::P).ctrl().shift(), true), ("target", "Change Target Language…", Shortcut::key(Key::P).ctrl(), false)] {
            let (pair, requests) = (self.pair.clone(), self.requests.clone());
            let options: Vec<(String, String)> = if source { std::iter::once((AUTO.to_owned(), self.sources()[0].0.clone())).chain(LANGUAGES.iter().map(|(v, l)| ((*v).to_owned(), (*l).to_owned()))).collect() } else { LANGUAGES.iter().map(|(v, l)| ((*v).to_owned(), (*l).to_owned())).collect() };
            languages.push(Action::new(id, title, |_| Ok(())).icon("flag").shortcut(key).keep_open().submenu(move || {
                options
                    .iter()
                    .map(|(value, label)| {
                        let next = if source { Pair { source: value.clone(), ..pair.clone() } } else { Pair { target: value.clone(), ..pair.clone() } };
                        let current = if source { &pair.source } else { &pair.target };
                        let requests = requests.clone();
                        let action = Action::new("language", label.clone(), move |_| {
                            requests.borrow_mut().push(Request::Choose(next.clone()));
                            Ok(())
                        })
                        .keep_open();
                        if current == value {
                            action.accessory("Selected")
                        } else {
                            action
                        }
                    })
                    .collect()
            }));
        }
        let source_index = if self.pair.source == AUTO { 0 } else { LANGUAGES.iter().position(|(v, _)| *v == self.pair.source).map_or(0, |i| i + 1) };
        let target_index = LANGUAGES.iter().position(|(v, _)| *v == self.pair.target).unwrap_or(0);
        Some(Page {
            title: "Translate".into(),
            dropdowns: vec![Dropdown { label: "Source language", options: self.sources(), selected: source_index }, Dropdown { label: "Target language", options: targets().collect(), selected: target_index }],
            actions: [primary, languages].into_iter().filter(|g| !g.is_empty()).collect(),
            enter_runs_primary: false,
            backspace_pops: self.text.is_empty(),
        })
    }

    fn page_select(&mut self, slot: usize, index: usize) {
        let value = match (slot, index) {
            (0, 0) => AUTO,
            (0, i) => LANGUAGES[i - 1].0,
            (_, i) => LANGUAGES[i].0,
        };
        let next = if slot == 0 { Pair { source: value.into(), ..self.pair.clone() } } else { Pair { target: value.into(), ..self.pair.clone() } };
        self.choose(next);
    }

    fn page_ui(&mut self, ui: &mut egui::Ui, rect: Rect) {
        let area = rect.shrink(8.0);
        let mid = area.center().x;
        let left = Rect::from_min_max(area.min, pos2(mid, area.bottom()));
        let right = Rect::from_min_max(pos2(mid, area.top()), area.max);
        ui.painter().rect_filled(left, 2.0, theme::fg(5));
        ui.painter().circle_filled(pos2(mid, area.center().y), 22.0, theme::BG());
        let input = Rect::from_min_max(pos2(left.left() + 8.0 + 24.0, left.top() + 40.0 + 8.0), pos2(left.right() - 8.0 - 46.0, left.bottom() - 8.0 - 28.0 - 16.0));
        let font = theme::font(24.0);
        let id = egui::Id::unique("translate-input");
        let hint = egui::RichText::new(crate::i18n::tr("Speak now")).color(theme::FG_40()).font(font.clone());
        let before = self.text.clone();
        let mut child = ui.new_child(UiBuilder::new().max_rect(input));
        let response = egui::ScrollArea::vertical()
            .id_salt("translate-input-scroll")
            .max_height(input.height())
            .auto_shrink(false)
            .show(&mut child, |ui| ui.add(egui::TextEdit::multiline(&mut self.text).id(id).frame(egui::Frame::NONE).margin(egui::Margin::ZERO).font(font.clone()).text_color(theme::FG()).hint_text(hint).desired_width(input.width()).desired_rows(1)))
            .inner;
        response.widget_info(|| egui::WidgetInfo::text_edit(true, "Text to translate", &self.text, ""));
        if self.focus.replace(false) || !ui.memory(|m| m.focused().is_some()) {
            ui.memory_mut(|m| m.request_focus(id));
        }
        if self.text != before {
            self.changed = Instant::now();
            self.error = None;
        }
        if !self.text.is_empty() {
            ui.painter().text(pos2(left.right() - 8.0 - 14.0, left.bottom() - 8.0 - 14.0), Align2::RIGHT_CENTER, count_label(&self.text), theme::font(12.0), theme::FG_40());
        }
        let output = Rect::from_min_max(pos2(right.left() + 8.0 + 38.0, right.top() + 40.0 + 8.0), pos2(right.right() - 8.0 - 22.0, right.bottom() - 8.0 - 28.0 - 16.0));
        let translation = self.translation();
        let pending = self.pending();
        let (text, color) = match &self.error {
            Some(e) => (format!("Translation failed: {e}"), theme::RED()),
            None if translation.is_empty() && !pending => ("Translation".to_owned(), theme::FG_40()),
            None => (format!("{translation}{}", if pending { "…" } else { "" }), theme::FG()),
        };
        let mut out = ui.new_child(UiBuilder::new().max_rect(output));
        egui::ScrollArea::vertical().id_salt("translate-output").auto_shrink(false).show(&mut out, |ui| {
            let label = ui.add(egui::Label::new(egui::RichText::new(&text).font(font.clone()).color(color)).wrap().selectable(true));
            label.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Label, true, format!("Translation: {text}")));
        });
        let button = Rect::from_center_size(pos2(mid, area.center().y), vec2(36.0, 36.0));
        let enabled = self.swapped().is_some();
        let swap = ui.interact(button, egui::Id::unique("translate-swap"), if enabled { Sense::click() } else { Sense::hover() });
        swap.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, enabled, "Swap languages"));
        let fill = if enabled && swap.hovered() { theme::CONTROL() } else { theme::fg(5) };
        ui.painter().circle_filled(button.center(), 18.0, fill);
        crate::icons::paint_glyph(ui, "arrow-right-left", Rect::from_center_size(button.center(), vec2(16.0, 16.0)), if enabled { theme::FG() } else { theme::FG_40() });
        if swap.clicked() {
            self.do_swap();
        }
    }
}

pub struct Translator;

impl Extension for Translator {
    fn id(&self) -> &'static str {
        "translator"
    }

    fn commands(&self) -> Vec<Command> {
        let mut open = command(self.id(), "translate", "Translate", ICON, "Command", "Open Command", |_| Ok(()));
        open.subtitle = Some("Translator".into());
        open.keywords = ["translate", "translator", "language", "google", "traduzir", "tradução"].map(String::from).to_vec();
        open.view = Some(Rc::new(|ctx| {
            let stored: Value = crate::store::read_json(&path());
            Box::new(Translate { ctx: ctx.clone(), pair: restore(&stored), text: String::new(), changed: Instant::now(), outcome: None, error: None, running: None, app: frontmost_app(), requests: Rc::default(), focus: Cell::new(true) })
        }));
        vec![open]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn language_list_has_unique_codes_and_sorted_labels() {
        let codes: std::collections::HashSet<_> = LANGUAGES.iter().map(|l| l.0).collect();
        assert_eq!(codes.len(), LANGUAGES.len());
        assert!(LANGUAGES.windows(2).all(|w| w[0].1.to_lowercase() <= w[1].1.to_lowercase()));
    }

    #[test]
    fn maps_detected_codes_and_labels() {
        assert_eq!(find_language(Some("fr")), Some("fr"));
        assert_eq!(find_language(Some("zh-cn")), Some("zh-CN"));
        assert_eq!(find_language(Some("zh")), Some("zh-CN"));
        assert_eq!(find_language(Some("he")), Some("iw"));
        assert_eq!(find_language(Some("pt-BR")), Some("pt"));
        assert_eq!(find_language(Some("xx")), None);
        assert_eq!(find_language(None), None);
        assert_eq!(language_label("zh-TW"), "Chinese (Traditional)");
        assert_eq!(language_label("xx"), "xx");
    }

    #[test]
    fn restores_saved_languages() {
        let pair = |s: &str, t: &str| Pair { source: s.into(), target: t.into() };
        assert_eq!(restore(&Value::Null), pair(AUTO, "en"));
        assert_eq!(restore(&json!({ "source": "de", "target": "fr" })), pair("de", "fr"));
        assert_eq!(restore(&json!({ "source": "auto", "target": "ja" })), pair(AUTO, "ja"));
        assert_eq!(restore(&json!({ "source": "xx", "target": 3 })), pair(AUTO, "en"));
    }

    #[test]
    fn swaps_languages_and_counts_text() {
        let pair = |s: &str, t: &str| Pair { source: s.into(), target: t.into() };
        assert_eq!(swap(&pair("en", "fr"), None), Some(pair("fr", "en")));
        assert_eq!(swap(&pair(AUTO, "fr"), Some("en")), Some(pair("fr", "en")));
        assert_eq!(swap(&pair(AUTO, "fr"), None), None);
        assert!(same_language(&pair("fr", "fr")) && !same_language(&pair(AUTO, "fr")));
        assert_eq!(count_label("Hello world"), "2 Words · 11 Characters");
        assert_eq!(count_label("a"), "1 Word · 1 Character");
    }

    #[test]
    fn parses_translation_bodies() {
        let body = json!([[["Bonjour. ", "Hello. ", null, null, 10], ["Comment ça va ?", "How are you?", null, null, 10]], null, "en"]);
        assert_eq!(parse(&body), Ok(Translation { text: "Bonjour. Comment ça va ?".into(), detected: Some("en".into()) }));
        assert_eq!(parse(&json!([[["Hola", "Hello"], [null, null, "Ola"]], null, "en"])).unwrap().text, "Hola");
        assert!(parse(&json!([null, null, "en"])).is_err() && parse(&json!([[], null, "en"])).is_err() && parse(&json!({ "error": "bad" })).is_err());
        assert_eq!(parse(&json!([[["Hallo", "Hello"]]])).unwrap().detected, None);
    }

    #[test]
    fn parses_fallback_bodies() {
        assert_eq!(parse_fallback(&json!([["Hello world", "pt"]])), Ok(Translation { text: "Hello world".into(), detected: Some("pt".into()) }));
        assert_eq!(parse_fallback(&json!(["hello world"])), Ok(Translation { text: "hello world".into(), detected: None }));
        assert!(parse_fallback(&json!([])).is_err() && parse_fallback(&json!([""])).is_err() && parse_fallback(&json!({ "error": "bad" })).is_err());
    }

    #[test]
    #[ignore = "network"]
    fn translates_over_the_network() {
        let result = translate("good morning", &Pair { source: AUTO.into(), target: "pt".into() });
        assert!(result.as_ref().is_ok_and(|t| t.detected.as_deref() == Some("en")), "{result:?}");
    }
}
