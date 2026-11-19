use egui::Color32;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

#[allow(dead_code)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tone {
    Blue,
    Orange,
    Red,
    Yellow,
    Purple,
    Green,
    Pink,
    Slate,
}

impl Tone {
    pub fn gradient(self) -> (Color32, Color32) {
        match self {
            Tone::Blue => (Color32::from_rgb(0x57, 0xA8, 0xFF), Color32::from_rgb(0x2D, 0x69, 0xDE)),
            Tone::Orange => (Color32::from_rgb(0xFF, 0xA5, 0x4B), Color32::from_rgb(0xE9, 0x5C, 0x12)),
            Tone::Red => (Color32::from_rgb(0xFD, 0x72, 0x73), Color32::from_rgb(0xD7, 0x23, 0x46)),
            Tone::Yellow => (Color32::from_rgb(0xF9, 0xC6, 0x35), Color32::from_rgb(0xE7, 0x81, 0x00)),
            Tone::Purple => (Color32::from_rgb(0xAC, 0x8F, 0xF8), Color32::from_rgb(0x72, 0x4E, 0xD8)),
            Tone::Green => (Color32::from_rgb(0x50, 0xD4, 0x92), Color32::from_rgb(0x00, 0x9B, 0x69)),
            Tone::Pink => (Color32::from_rgb(0xFA, 0x7B, 0xA7), Color32::from_rgb(0xD3, 0x31, 0x6A)),
            Tone::Slate => (Color32::from_rgb(0x82, 0x8D, 0x9F), Color32::from_rgb(0x4C, 0x56, 0x66)),
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub enum Icon {
    Tile(&'static str, Tone),
    File(String),
    Thumbnail(String),
    Image(String),
    Swatch(Color32),
    Glyph(&'static str),
    Boxed(&'static str),
    Url(String),
    Emoji(String),
    Initials(String),
    Dot(Color32),
    Time(Option<Color32>, String, bool),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Shortcut {
    pub key: Option<egui::Key>,
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
}

impl Shortcut {
    pub const fn key(key: egui::Key) -> Self {
        Self { key: Some(key), ctrl: false, alt: false, shift: false }
    }

    pub const fn ctrl(mut self) -> Self {
        self.ctrl = true;
        self
    }

    pub const fn alt(mut self) -> Self {
        self.alt = true;
        self
    }

    pub const fn shift(mut self) -> Self {
        self.shift = true;
        self
    }

    pub fn labels(&self) -> Vec<String> {
        let mut labels: Vec<String> = [(self.ctrl, "Ctrl"), (self.alt, "Alt"), (self.shift, "Shift")].iter().filter(|(on, _)| *on).map(|(_, l)| l.to_string()).collect();
        if let Some(key) = self.key {
            labels.push(match key {
                egui::Key::Enter => "↵".into(),
                egui::Key::ArrowUp => "↑".into(),
                egui::Key::ArrowDown => "↓".into(),
                egui::Key::ArrowLeft => "←".into(),
                egui::Key::ArrowRight => "→".into(),
                egui::Key::Backspace => "⌫".into(),
                egui::Key::Delete => "Del".into(),
                egui::Key::Escape => "Esc".into(),
                egui::Key::Space => "Space".into(),
                other => other.symbol_or_name().to_uppercase(),
            });
        }
        labels
    }

    pub fn matches(&self, key: egui::Key, modifiers: egui::Modifiers) -> bool {
        self.key == Some(key) && modifiers.ctrl == self.ctrl && modifiers.alt == self.alt && modifiers.shift == self.shift
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct Confirmation {
    pub title: String,
    pub message: String,
    pub confirm_title: String,
}

pub type Run = Rc<dyn Fn(&mut Host) -> Result<(), String>>;
pub type TextRun = Rc<dyn Fn(&str, &mut Host) -> Result<(), String>>;

#[derive(Clone)]
pub struct Action {
    pub id: &'static str,
    pub title: String,
    pub icon: Option<&'static str>,
    pub shortcut: Option<Shortcut>,
    pub destructive: bool,
    pub keep_open: bool,
    pub accessory: Option<String>,
    pub confirm: Option<Confirmation>,
    pub submenu: Option<Rc<dyn Fn() -> Vec<Action>>>,
    pub run: Run,
}

impl Action {
    pub fn new(id: &'static str, title: impl Into<String>, run: impl Fn(&mut Host) -> Result<(), String> + 'static) -> Self {
        Self { id, title: title.into(), icon: None, shortcut: None, destructive: false, keep_open: false, accessory: None, confirm: None, submenu: None, run: Rc::new(run) }
    }

    pub fn icon(mut self, icon: &'static str) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn shortcut(mut self, shortcut: Shortcut) -> Self {
        self.shortcut = Some(shortcut);
        self
    }

    pub fn keep_open(mut self) -> Self {
        self.keep_open = true;
        self
    }

    pub fn destructive(mut self) -> Self {
        self.destructive = true;
        self
    }

    pub fn accessory(mut self, text: impl Into<String>) -> Self {
        self.accessory = Some(text.into());
        self
    }

    pub fn confirm(mut self, title: &str, message: &str, confirm_title: &str) -> Self {
        self.confirm = Some(Confirmation { title: title.into(), message: message.into(), confirm_title: confirm_title.into() });
        self
    }

    pub fn submenu(mut self, items: impl Fn() -> Vec<Action> + 'static) -> Self {
        self.submenu = Some(Rc::new(items));
        self
    }
}

#[derive(Clone)]
pub struct Argument {
    pub placeholder: String,
    pub optional: bool,
    pub run: TextRun,
}

#[derive(Clone)]
pub struct Query {
    pub matches: fn(&str) -> bool,
    pub run: TextRun,
}

pub type ViewFactory = Rc<dyn Fn(&egui::Context) -> Box<dyn View>>;

#[derive(Clone)]
pub struct Command {
    pub key: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub icon: Icon,
    pub keywords: Vec<String>,
    pub kind: &'static str,
    pub primary_action_title: &'static str,
    pub confirm: Option<Confirmation>,
    pub actions: Vec<Vec<Action>>,
    pub run: Run,
    pub view: Option<ViewFactory>,
    pub fallback: Option<TextRun>,
    pub query: Option<Query>,
    pub argument: Option<Argument>,
}

#[derive(Clone, PartialEq, Debug)]
pub enum Accessory {
    Text(String),
    Tag(String),
    Volume { level: f32, muted: bool, input: bool },
}

#[derive(Clone)]
pub struct Item {
    pub id: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub icon: Icon,
    pub accessories: Vec<Accessory>,
    pub keywords: Vec<String>,
    pub actions: Vec<Vec<Action>>,
}

pub struct Section {
    pub title: String,
    pub items: Vec<Item>,
}

pub struct Empty {
    pub title: String,
    pub description: Option<String>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Side {
    pub text: String,
    pub label: String,
    pub swatch: Option<Color32>,
    pub dim: Option<Vec<(usize, usize)>>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Forecast {
    pub label: String,
    pub icon: String,
    pub high: String,
    pub low: Option<String>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct WeatherCard {
    pub temperature: String,
    pub icon: String,
    pub place: String,
    pub condition: String,
    pub range: String,
    pub today: Vec<Forecast>,
    pub week: Vec<Forecast>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct FlightCard {
    pub from: (String, String, String),
    pub to: (String, String, String),
    pub status: String,
    pub live: bool,
    pub note: Option<String>,
}

#[derive(Clone, PartialEq, Debug)]
pub enum CardKind {
    Pair,
    Weather(WeatherCard),
    Flight(FlightCard),
    Loading,
    LoadingPair,
}

#[derive(Clone)]
pub struct Inline {
    pub section: String,
    pub input: Side,
    pub output: Side,
    pub note: Option<String>,
    pub actions: Vec<Vec<Action>>,
    pub kind: CardKind,
}

#[derive(Clone)]
pub struct Dropdown {
    pub label: &'static str,
    pub options: Vec<(String, Icon)>,
    pub selected: usize,
}

#[derive(Clone)]
pub enum FieldKind {
    Text,
    Area(f32),
    Choice(Vec<(String, String, Icon)>),
    Toggle,
    Tokens(Tokens),
}

#[derive(Clone)]
pub struct Tokens {
    pub options: Vec<(String, String, Icon)>,
    pub create: Option<(String, fn(&str) -> Option<String>)>,
    pub describe: fn(&str) -> (String, Icon),
}

#[derive(Clone)]
pub struct Field {
    pub label: &'static str,
    pub kind: FieldKind,
    pub value: String,
    pub placeholder: String,
    pub help: Option<String>,
    pub error: Option<String>,
    pub mono: bool,
    pub secret: bool,
}

impl Field {
    pub fn new(label: &'static str, kind: FieldKind, value: impl Into<String>) -> Self {
        Self { label, kind, value: value.into(), placeholder: String::new(), help: None, error: None, mono: false, secret: false }
    }

    pub fn placeholder(mut self, text: impl Into<String>) -> Self {
        self.placeholder = text.into();
        self
    }

    pub fn help(mut self, text: impl Into<String>) -> Self {
        self.help = Some(text.into());
        self
    }

    pub fn mono(mut self) -> Self {
        self.mono = true;
        self
    }
}

pub struct Form {
    pub title: String,
    pub submit: &'static str,
    pub icon: Icon,
    pub fields: Vec<Field>,
    pub focus: Option<usize>,
}

pub struct Page {
    pub title: String,
    pub dropdowns: Vec<Dropdown>,
    pub actions: Vec<Vec<Action>>,
    pub enter_runs_primary: bool,
    pub backspace_pops: bool,
}

pub trait View {
    fn placeholder(&self) -> String;
    fn dropdown(&self) -> Option<Dropdown> {
        None
    }
    fn select(&mut self, _index: usize) {}
    fn has_detail(&self) -> bool {
        false
    }
    fn grid(&self) -> Option<usize> {
        None
    }
    fn page(&mut self) -> Option<Page> {
        None
    }
    fn page_ui(&mut self, _ui: &mut egui::Ui, _rect: egui::Rect) {}
    fn page_select(&mut self, _slot: usize, _index: usize) {}
    fn detail(&mut self, _ui: &mut egui::Ui, _rect: egui::Rect, _id: &str) {}
    fn form(&mut self) -> Option<&mut Form> {
        None
    }
    fn submit(&mut self, _host: &mut Host) -> Result<bool, String> {
        Ok(false)
    }
    fn form_changed(&mut self, _index: usize) {}
    fn empty_actions(&self) -> Vec<Vec<Action>> {
        Vec::new()
    }
    fn item_argument(&self, _id: &str) -> Option<(String, String)> {
        None
    }
    fn set_argument(&mut self, _id: &str, _value: &str) {}
    fn filters(&self) -> bool {
        true
    }
    fn set_query(&mut self, _query: &str) {}
    fn footer(&self) -> (String, Icon);
    fn sections(&mut self) -> Vec<Section>;
    fn loading(&self) -> bool {
        false
    }
    fn empty(&self) -> Empty {
        Empty { title: "No Results".into(), description: None }
    }
    fn tab_cycles(&self) -> bool {
        false
    }
    fn initial_selection(&self) -> Option<String> {
        None
    }
}

pub struct Load<T>(Arc<Mutex<Option<Result<T, String>>>>);

impl<T: Clone + Send + 'static> Load<T> {
    pub fn start(ctx: &egui::Context, work: impl FnOnce() -> Result<T, String> + Send + 'static) -> Self {
        let slot = Arc::new(Mutex::new(None));
        let (target, ctx) = (slot.clone(), ctx.clone());
        std::thread::spawn(move || {
            let result = work();
            *target.lock().unwrap_or_else(|e| e.into_inner()) = Some(result);
            ctx.request_repaint();
        });
        Self(slot)
    }

    pub fn get(&self) -> Option<Result<T, String>> {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }
}

pub enum Toast {
    Success(String),
    Failure(String, String),
}

pub enum Request {
    OpenSettings,
    OpenNotes { create: bool },
    Quit,
    SetFavorites(Vec<String>),
    ResetRanking(String),
    Hide(bool),
    Hud(Toast),
    Push(ViewFactory),
    FocusArgument,
    SetQuery(String),
}

pub type MakeView = Box<dyn FnOnce(&egui::Context) -> Box<dyn View> + Send>;

pub enum Async {
    Show,
    Hide(bool),
    Hud(Toast),
    Toast(Toast),
    PopToRoot,
    SetQuery(String),
    Confirm(Confirmation, Box<dyn FnOnce(bool) + Send>),
    Launch(String, Option<String>),
    Push(MakeView),
}

pub static ASYNC: std::sync::Mutex<Vec<Async>> = std::sync::Mutex::new(Vec::new());

pub fn post(ctx: &egui::Context, request: Async) {
    ASYNC.lock().unwrap_or_else(|e| e.into_inner()).push(request);
    ctx.request_repaint();
}

pub static NOTICES: std::sync::Mutex<Vec<Toast>> = std::sync::Mutex::new(Vec::new());

pub static TOASTS: std::sync::Mutex<Vec<Toast>> = std::sync::Mutex::new(Vec::new());

pub fn notify_toast(ctx: &egui::Context, toast: Toast) {
    TOASTS.lock().unwrap_or_else(|e| e.into_inner()).push(toast);
    ctx.request_repaint();
}

pub fn notify(ctx: &egui::Context, toast: Toast) {
    NOTICES.lock().unwrap_or_else(|e| e.into_inner()).push(toast);
    ctx.request_repaint();
}

pub struct Host {
    pub ctx: egui::Context,
    pub toast: Option<Toast>,
    pub requests: Vec<Request>,
}

impl Host {
    pub fn copy(&mut self, text: &str, what: &str) {
        self.ctx.copy_text(text.to_owned());
        self.toast = Some(Toast::Success(format!("Copied {what} to clipboard")));
    }

    pub fn copy_and_close(&mut self, text: &str, title: &str) {
        self.ctx.copy_text(text.to_owned());
        self.hud(Ok(title.to_owned()));
    }

    pub fn paste(&mut self, text: &str) -> Result<(), String> {
        crate::clipboard::write_text(text)?;
        self.hide(true);
        std::thread::spawn(|| {
            std::thread::sleep(std::time::Duration::from_millis(80));
            crate::clipboard::send_paste();
        });
        Ok(())
    }

    pub fn hide(&mut self, restore_focus: bool) {
        self.requests.push(Request::Hide(restore_focus));
    }

    pub fn hud(&mut self, result: Result<String, String>) {
        self.hide(true);
        self.requests.push(Request::Hud(match result {
            Ok(title) => Toast::Success(title),
            Err(error) => Toast::Failure(error, String::new()),
        }));
    }
}

pub trait Extension {
    fn id(&self) -> &'static str;
    fn commands(&self) -> Vec<Command>;
    fn icon(&self) -> Option<Icon> {
        None
    }
    fn inline(&self, _ctx: &egui::Context, _query: &str) -> Option<Inline> {
        None
    }
}

pub fn frontmost_app() -> Option<String> {
    let previous = crate::launcher::previous_window();
    if previous.is_invalid() {
        return None;
    }
    crate::clipboard::window_app(previous).map(|(name, _)| name)
}

pub fn paste_title(app: &Option<String>) -> String {
    app.as_ref().map_or_else(|| "Paste".into(), |app| format!("Paste to {app}"))
}
