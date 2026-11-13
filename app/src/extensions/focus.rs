use super::{command, GENERATION};
use crate::extension::{notify_toast, Accessory, Action, Command, Empty, Extension, Field, FieldKind, Form, Host, Icon, Item, Request, Section, Shortcut, Toast, Tokens, Tone, View, ViewFactory};
use crate::focus::{self, Mode, Session, Status};
use crate::focus_ui::{self, ICON};
use egui::Key;
use serde::{Deserialize, Serialize};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

static VERSION: AtomicU64 = AtomicU64::new(0);
static SUBTITLE: Mutex<String> = Mutex::new(String::new());
const DEFAULT_TITLE: &str = "Focus session";
const NO_SESSION: &str = "No focus session is running";
pub const CATEGORY_ICONS: [(&str, &str, &str); 12] = [
    ("target", "target", "Target"),
    ("users", "users", "Users"),
    ("message", "message-circle", "Message"),
    ("game", "gamepad-2", "Game"),
    ("play", "play", "Play"),
    ("news", "newspaper", "News"),
    ("cart", "shopping-cart", "Cart"),
    ("plane", "plane", "Plane"),
    ("code", "code", "Code"),
    ("book", "book-open", "Book"),
    ("music", "music", "Music"),
    ("work", "briefcase", "Work"),
];
const PAUSE_CHOICES: [(&str, &str, Option<u64>); 5] =
    [("pause-1", "1 minute", Some(60_000)), ("pause-2", "2 minutes", Some(120_000)), ("pause-5", "5 minutes", Some(300_000)), ("pause-10", "10 minutes", Some(600_000)), ("pause-forever", "Indefinitely", None)];

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct Category {
    pub id: String,
    pub title: String,
    pub icon: String,
    pub apps: Vec<String>,
    pub websites: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct Preferences {
    pub snooze_ms: Option<u64>,
    pub play_sound: bool,
    pub restore_apps: bool,
    pub allow_all_websites: bool,
}

impl Default for Preferences {
    fn default() -> Self {
        Self { snooze_ms: Some(180_000), play_sound: true, restore_apps: false, allow_all_websites: false }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct FocusForm {
    pub goal: String,
    pub duration_ms: Option<u64>,
    pub mode: Mode,
    pub items: Vec<String>,
}

impl Default for FocusForm {
    fn default() -> Self {
        Self { goal: String::new(), duration_ms: None, mode: Mode::Block, items: Vec::new() }
    }
}

fn builtin(id: &str, title: &str, icon: &str, apps: &[&str], websites: &[&str]) -> Category {
    Category { id: id.into(), title: title.into(), icon: icon.into(), apps: apps.iter().map(|s| s.to_string()).collect(), websites: websites.iter().map(|s| s.to_string()).collect() }
}

pub fn builtin_categories() -> Vec<Category> {
    vec![
        builtin("social", "Social", "users", &[], &["facebook.com", "instagram.com", "x.com", "twitter.com", "reddit.com", "tiktok.com", "linkedin.com", "pinterest.com", "threads.net", "bsky.app", "tumblr.com", "snapchat.com"]),
        builtin("messaging", "Messaging", "message", &["discord.exe", "slack.exe", "telegram.exe", "signal.exe", "ms-teams.exe", "whatsapp.exe"], &["web.whatsapp.com", "discord.com", "web.telegram.org", "messenger.com", "slack.com"]),
        builtin("gaming", "Gaming", "game", &["steam.exe", "epicgameslauncher.exe", "battle.net.exe", "riotclientservices.exe", "eadesktop.exe", "galaxyclient.exe"], &["store.steampowered.com", "steamcommunity.com", "epicgames.com", "itch.io", "gog.com"]),
        builtin("streaming", "Streaming", "play", &[], &["youtube.com", "netflix.com", "primevideo.com", "disneyplus.com", "max.com", "hulu.com", "twitch.tv", "crunchyroll.com", "globoplay.globo.com"]),
        builtin("news", "News", "news", &[], &["news.ycombinator.com", "bbc.com", "cnn.com", "nytimes.com", "theguardian.com", "reuters.com", "g1.globo.com", "uol.com.br"]),
        builtin("shopping", "Shopping", "cart", &[], &["amazon.*", "ebay.com", "aliexpress.com", "mercadolivre.com.br", "shopee.com.br", "etsy.com", "temu.com", "shein.com"]),
    ]
}

fn path() -> std::path::PathBuf {
    crate::data_dir().join("focus.json")
}

fn data() -> serde_json::Map<String, serde_json::Value> {
    match crate::store::read_json::<serde_json::Value>(&path()) {
        serde_json::Value::Object(map) => map,
        _ => serde_json::Map::new(),
    }
}

fn read<T: serde::de::DeserializeOwned>(key: &str) -> Option<T> {
    data().get(key).and_then(|v| serde_json::from_value(v.clone()).ok())
}

fn write(key: &str, value: impl Serialize) -> Result<(), String> {
    let mut map = data();
    map.insert(key.into(), serde_json::to_value(value).map_err(|e| e.to_string())?);
    crate::store::write_json(&path(), &serde_json::Value::Object(map)).map_err(|e| e.to_string())
}

pub fn categories() -> Vec<Category> {
    read("categories").unwrap_or_else(builtin_categories)
}

fn save_categories(list: &[Category]) -> Result<(), String> {
    write("categories", list)?;
    VERSION.fetch_add(1, Ordering::SeqCst);
    Ok(())
}

fn preferences() -> Preferences {
    read("preferences").unwrap_or_default()
}

pub fn focus_preferences() -> Preferences {
    preferences()
}

pub fn save_focus_preferences(preferences: &Preferences) -> Result<(), String> {
    write("preferences", preferences)
}

fn last_form() -> FocusForm {
    read("lastForm").unwrap_or_default()
}

pub fn normalize_host(text: &str) -> Option<String> {
    let value = text.trim().to_lowercase();
    if value.is_empty() || value.contains(char::is_whitespace) {
        return None;
    }
    let rest = value.split_once("://").map(|(_, r)| r).unwrap_or(&value);
    let host = rest.split(['/', '?', '#']).next()?.rsplit('@').next()?.split(':').next()?;
    let host = host.strip_prefix("www.").unwrap_or(host);
    let (head, tld) = host.rsplit_once('.')?;
    let first = head.chars().next()?;
    let valid = (first.is_ascii_alphanumeric() || first == '*')
        && head.chars().all(|c| c.is_ascii_alphanumeric() || ".*-".contains(c))
        && !tld.is_empty()
        && tld.chars().all(|c| c.is_ascii_alphanumeric() || "*-".contains(c));
    valid.then(|| host.to_owned())
}

pub fn slugify(title: &str, taken: &[String]) -> String {
    let lower = title.to_lowercase();
    let parts: Vec<&str> = lower.split(|c: char| !c.is_ascii_alphanumeric()).filter(|p| !p.is_empty()).collect();
    let base = if parts.is_empty() { "category".to_owned() } else { parts.join("-") };
    let mut id = base.clone();
    let mut n = 2;
    while taken.contains(&id) {
        id = format!("{base}-{n}");
        n += 1;
    }
    id
}

pub fn parse_import(text: &str) -> Result<Vec<Category>, String> {
    let invalid = || "Invalid focus category import format".to_owned();
    let data: serde_json::Value = serde_json::from_str(text).map_err(|_| invalid())?;
    let list = match &data {
        serde_json::Value::Array(list) => list.clone(),
        serde_json::Value::Object(map) => match map.get("focusCategories") {
            Some(serde_json::Value::Array(list)) => list.clone(),
            _ => vec![data.clone()],
        },
        _ => return Err(invalid()),
    };
    let strings = |v: Option<&serde_json::Value>| -> Vec<String> { v.and_then(|v| v.as_array()).map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default() };
    list.iter()
        .map(|entry| {
            let e = entry.as_object().ok_or_else(invalid)?;
            let title = e.get("title").and_then(|t| t.as_str()).map(str::trim).filter(|t| !t.is_empty()).ok_or_else(invalid)?;
            let icon = e.get("icon").or_else(|| e.get("iconName")).and_then(|i| i.as_str()).unwrap_or("target");
            Ok(Category {
                id: String::new(),
                title: title.to_owned(),
                icon: if CATEGORY_ICONS.iter().any(|(k, _, _)| *k == icon) { icon.to_owned() } else { "target".into() },
                apps: strings(e.get("applicationIds").or_else(|| e.get("apps"))),
                websites: strings(e.get("websiteHosts").or_else(|| e.get("websites"))).iter().filter_map(|w| normalize_host(w)).collect(),
            })
        })
        .collect()
}

fn unique(values: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for value in values {
        if !value.is_empty() && !out.contains(&value) {
            out.push(value);
        }
    }
    out
}

pub fn start_input(form: &FocusForm, categories: &[Category], preferences: &Preferences) -> focus::StartInput {
    let selected: Vec<&Category> = categories.iter().filter(|c| form.items.contains(&format!("category:{}", c.id))).collect();
    let of = |prefix: &str| form.items.iter().filter_map(|i| i.strip_prefix(prefix).map(String::from)).collect::<Vec<_>>();
    focus::StartInput {
        title: if form.goal.trim().is_empty() { DEFAULT_TITLE.into() } else { form.goal.trim().into() },
        mode: form.mode,
        duration_ms: form.duration_ms,
        apps: unique(selected.iter().flat_map(|c| c.apps.clone()).chain(of("app:"))),
        websites: unique(selected.iter().flat_map(|c| c.websites.clone()).chain(of("site:"))),
        allow_all_websites: preferences.allow_all_websites,
        snooze_ms: preferences.snooze_ms.filter(|ms| *ms > 0),
        play_sound: preferences.play_sound,
        restore_apps: preferences.restore_apps,
        form: serde_json::to_value(form).unwrap_or_default(),
    }
}

fn glyph(icon: &str) -> &'static str {
    CATEGORY_ICONS.iter().find(|(k, _, _)| *k == icon).map(|(_, g, _)| *g).unwrap_or("target")
}

fn category_icon(category: &Category) -> Icon {
    Icon::Tile(glyph(&category.icon), Tone::Green)
}

fn subtitle(session: &Session, now: u64) -> String {
    match (session.status, session.duration_ms) {
        (Status::Paused, _) => "Paused".into(),
        (_, None) => "Running".into(),
        _ => format!("{} left", focus_ui::format_timer(focus_ui::remaining_ms(session, now) as i64)),
    }
}

pub fn session_changed() {
    GENERATION.fetch_add(1, Ordering::SeqCst);
}

pub fn session_tick(session: &Session, now: u64) {
    if !crate::launcher::is_shown() {
        return;
    }
    let text = subtitle(session, now);
    let mut last = SUBTITLE.lock().unwrap_or_else(|e| e.into_inner());
    if *last != text {
        *last = text;
        GENERATION.fetch_add(1, Ordering::SeqCst);
    }
}

fn describe(value: &str) -> (String, Icon) {
    if let Some(host) = value.strip_prefix("site:") {
        return (host.to_owned(), Icon::Glyph("link"));
    }
    if let Some(id) = value.strip_prefix("category:") {
        return match categories().into_iter().find(|c| c.id == id) {
            Some(c) => (c.title.clone(), category_icon(&c)),
            None => (id.to_owned(), ICON),
        };
    }
    let path = value.strip_prefix("app:").unwrap_or(value);
    let apps = super::APPS.lock().unwrap_or_else(|e| e.into_inner());
    match apps.iter().find(|a| a.path == path) {
        Some(app) => (app.name.clone(), Icon::File(app.path.clone())),
        None => {
            let name = path.rsplit(['\\', '/']).next().unwrap_or(path);
            let lower = name.to_lowercase();
            let stem = if lower.ends_with(".exe") || lower.ends_with(".lnk") { &name[..name.len() - 4] } else { name };
            (stem.to_owned(), Icon::Glyph("app-window"))
        }
    }
}

fn site_value(query: &str) -> Option<String> {
    normalize_host(query).map(|h| format!("site:{h}"))
}

fn app_options() -> Vec<(String, String, Icon)> {
    let mut apps = super::APPS.lock().unwrap_or_else(|e| e.into_inner()).clone();
    apps.sort_by_key(|a| a.name.to_lowercase());
    apps.into_iter().map(|a| (format!("app:{}", a.path), a.name, Icon::File(a.path))).collect()
}

fn tokens(options: Vec<(String, String, Icon)>, create: &str) -> FieldKind {
    FieldKind::Tokens(Tokens { options, create: Some((create.to_owned(), site_value)), describe })
}

fn session_tokens(mode: Mode) -> (FieldKind, &'static str, &'static str) {
    let mut options: Vec<(String, String, Icon)> = categories().iter().map(|c| (format!("category:{}", c.id), c.title.clone(), category_icon(c))).collect();
    options.extend(app_options());
    match mode {
        Mode::Block => (tokens(options, "Block website"), "Block", "What apps or websites do you want to block?"),
        Mode::Allow => (tokens(options, "Allow website"), "Allow", "What apps or websites do you want to allow?"),
    }
}

fn duration_options(current: Option<u64>) -> Vec<(String, String, Icon)> {
    let mut list = vec![(String::new(), "No Limit".to_owned(), Icon::Glyph("clock"))];
    list.extend([5u64, 10, 15, 25, 30, 45].map(|m| ((m * 60_000).to_string(), format!("{m} minutes"), Icon::Glyph("clock"))));
    list.push(((60 * 60_000u64).to_string(), "1 hour".into(), Icon::Glyph("clock")));
    list.push(((90 * 60_000u64).to_string(), "1.5 hours".into(), Icon::Glyph("clock")));
    list.extend([2u64, 3, 4].map(|h| ((h * 3_600_000).to_string(), format!("{h} hours"), Icon::Glyph("clock"))));
    if let Some(ms) = current.filter(|ms| !list.iter().any(|(v, _, _)| *v == ms.to_string())) {
        list.push((ms.to_string(), focus_ui::format_timer(ms as i64), Icon::Glyph("clock")));
    }
    list
}

fn mode_value(mode: Mode) -> &'static str {
    match mode {
        Mode::Block => "block",
        Mode::Allow => "allow",
    }
}

fn joined(items: &[String]) -> String {
    items.join("\n")
}

fn split(value: &str) -> Vec<String> {
    value.lines().filter(|l| !l.is_empty()).map(String::from).collect()
}

struct SessionForm {
    edit: bool,
    form: Form,
}

impl SessionForm {
    fn factory(edit: bool) -> ViewFactory {
        Rc::new(move |_| Box::new(Self::new(edit)) as Box<dyn View>)
    }

    fn new(edit: bool) -> Self {
        let session = edit.then(focus::get).flatten();
        let initial = match &session {
            Some(s) => serde_json::from_value(s.form.clone()).unwrap_or(FocusForm { goal: s.title.clone(), ..Default::default() }),
            None if edit => FocusForm::default(),
            None => last_form(),
        };
        let (kind, label, placeholder) = session_tokens(initial.mode);
        let mut goal = Field::new("Goal", FieldKind::Text, initial.goal.clone()).placeholder("What are you focusing on?");
        if edit && session.is_none() {
            goal.error = Some(format!("{NO_SESSION}."));
        }
        let fields = vec![
            goal,
            Field::new("Duration", FieldKind::Choice(duration_options(initial.duration_ms)), initial.duration_ms.map(|ms| ms.to_string()).unwrap_or_default()),
            Field::new("Mode", FieldKind::Choice(vec![("block".into(), "Block Apps & Websites".into(), Icon::Glyph("shield")), ("allow".into(), "Allow Apps & Websites".into(), Icon::Glyph("circle-check"))]), mode_value(initial.mode)),
            Field::new(label, kind, joined(&initial.items)).placeholder(placeholder),
        ];
        let title = if edit { "Update Session" } else { "Start Focus Session" };
        Self { edit, form: Form { title: title.into(), submit: title, icon: ICON, fields, focus: Some(0) } }
    }

    fn current(&self) -> FocusForm {
        let value = |i: usize| self.form.fields[i].value.clone();
        FocusForm { goal: value(0), duration_ms: value(1).parse().ok(), mode: if value(2) == "allow" { Mode::Allow } else { Mode::Block }, items: split(&value(3)) }
    }
}

impl View for SessionForm {
    fn placeholder(&self) -> String {
        String::new()
    }

    fn footer(&self) -> (String, Icon) {
        ("Focus".into(), ICON)
    }

    fn sections(&mut self) -> Vec<Section> {
        Vec::new()
    }

    fn form(&mut self) -> Option<&mut Form> {
        Some(&mut self.form)
    }

    fn form_changed(&mut self, index: usize) {
        if index == 2 {
            let mode = self.current().mode;
            let (kind, label, placeholder) = session_tokens(mode);
            let field = &mut self.form.fields[3];
            field.kind = kind;
            field.label = label;
            field.placeholder = placeholder.into();
        }
    }

    fn submit(&mut self, host: &mut Host) -> Result<bool, String> {
        let form = self.current();
        let input = start_input(&form, &categories(), &preferences());
        let focus_host = focus_ui::host(&host.ctx);
        if self.edit {
            focus::update(&focus_host, input)?;
            host.hud(Ok("Focus session updated".into()));
        } else {
            focus::start(&focus_host, input);
            write("lastForm", &form)?;
            host.hud(Ok("Focus session started".into()));
        }
        Ok(true)
    }
}

struct CategoryForm {
    initial: Option<Category>,
    form: Form,
}

impl CategoryForm {
    fn factory(initial: Option<Category>) -> ViewFactory {
        Rc::new(move |_| Box::new(Self::new(initial.clone())) as Box<dyn View>)
    }

    fn new(initial: Option<Category>) -> Self {
        let items: Vec<String> = initial.as_ref().map(|c| c.apps.iter().map(|a| format!("app:{a}")).chain(c.websites.iter().map(|w| format!("site:{w}"))).collect()).unwrap_or_default();
        let icons = CATEGORY_ICONS.iter().map(|(key, glyph, label)| (key.to_string(), label.to_string(), Icon::Glyph(glyph))).collect();
        let fields = vec![
            Field::new("Title", FieldKind::Text, initial.as_ref().map(|c| c.title.clone()).unwrap_or_default()).placeholder("Category name"),
            Field::new("Icon", FieldKind::Choice(icons), initial.as_ref().map(|c| c.icon.clone()).unwrap_or_else(|| "target".into())),
            Field::new("Apps & Websites", tokens(app_options(), "Add website"), joined(&items)).placeholder("Add apps or type a website"),
        ];
        let (title, submit) = if initial.is_some() { ("Edit Focus Category", "Save Category") } else { ("Create Focus Category", "Create Category") };
        Self { initial, form: Form { title: title.into(), submit, icon: ICON, fields, focus: Some(0) } }
    }
}

impl View for CategoryForm {
    fn placeholder(&self) -> String {
        String::new()
    }

    fn footer(&self) -> (String, Icon) {
        ("Focus".into(), ICON)
    }

    fn sections(&mut self) -> Vec<Section> {
        Vec::new()
    }

    fn form(&mut self) -> Option<&mut Form> {
        Some(&mut self.form)
    }

    fn submit(&mut self, host: &mut Host) -> Result<bool, String> {
        let title = self.form.fields[0].value.trim().to_owned();
        if title.is_empty() {
            self.form.fields[0].error = Some("Title is required".into());
            return Ok(false);
        }
        let items = split(&self.form.fields[2].value);
        let mut list = categories();
        let category = Category {
            id: self.initial.as_ref().map(|c| c.id.clone()).unwrap_or_else(|| slugify(&title, &list.iter().map(|c| c.id.clone()).collect::<Vec<_>>())),
            title,
            icon: self.form.fields[1].value.clone(),
            apps: items.iter().filter_map(|i| i.strip_prefix("app:").map(String::from)).collect(),
            websites: items.iter().filter_map(|i| i.strip_prefix("site:").map(String::from)).collect(),
        };
        match &self.initial {
            Some(old) => list.iter_mut().filter(|c| c.id == old.id).for_each(|c| *c = category.clone()),
            None => list.push(category),
        }
        save_categories(&list)?;
        host.toast = Some(Toast::Success(if self.initial.is_some() { "Updated Focus Category".into() } else { "Created Focus Category".into() }));
        Ok(true)
    }
}

fn plural(count: usize, word: &str) -> String {
    if count == 1 {
        format!("1 {word}")
    } else {
        format!("{count} {word}s")
    }
}

fn create_category_action() -> Action {
    Action::new("create", "Create Focus Category", |host: &mut Host| {
        host.requests.push(Request::Push(CategoryForm::factory(None)));
        Ok(())
    })
    .icon("plus")
    .shortcut(Shortcut::key(Key::N).ctrl())
    .keep_open()
}

struct SearchCategories {
    version: u64,
    list: Vec<Category>,
}

impl View for SearchCategories {
    fn placeholder(&self) -> String {
        "Search focus categories…".into()
    }

    fn footer(&self) -> (String, Icon) {
        ("Search Focus Categories".into(), ICON)
    }

    fn empty(&self) -> Empty {
        if self.list.is_empty() {
            Empty { title: "No focus categories yet".into(), description: Some("Press Ctrl+N to create one.".into()) }
        } else {
            Empty { title: "No Results".into(), description: None }
        }
    }

    fn empty_actions(&self) -> Vec<Vec<Action>> {
        vec![vec![create_category_action()]]
    }

    fn sections(&mut self) -> Vec<Section> {
        let version = VERSION.load(Ordering::SeqCst);
        if version != self.version {
            self.version = version;
            self.list = categories();
        }
        let items = self
            .list
            .iter()
            .map(|category| {
                let counts: Vec<String> = [(category.apps.len(), "app"), (category.websites.len(), "website")].into_iter().filter(|(n, _)| *n > 0).map(|(n, w)| plural(n, w)).collect();
                let (edit, delete) = (category.clone(), category.id.clone());
                Item {
                    id: category.id.clone(),
                    title: category.title.clone(),
                    subtitle: None,
                    icon: category_icon(category),
                    accessories: if counts.is_empty() { Vec::new() } else { vec![Accessory::Text(counts.join(", "))] },
                    keywords: category.apps.iter().chain(&category.websites).cloned().collect(),
                    actions: vec![
                        vec![Action::new("edit", "Edit Category", move |host: &mut Host| {
                            host.requests.push(Request::Push(CategoryForm::factory(Some(edit.clone()))));
                            Ok(())
                        })
                        .icon("pencil")
                        .keep_open()],
                        vec![create_category_action()],
                        vec![Action::new("delete", "Delete Category", move |_| {
                            let list: Vec<Category> = categories().into_iter().filter(|c| c.id != delete).collect();
                            save_categories(&list)
                        })
                        .icon("trash-2")
                        .destructive()
                        .keep_open()
                        .shortcut(Shortcut::key(Key::D).ctrl())
                        .confirm("Delete Focus Category?", &format!("“{}” will be deleted. This can't be undone.", category.title), "Delete Category")],
                    ],
                }
            })
            .collect();
        vec![Section { title: "Categories".into(), items }]
    }
}

fn import(ctx: egui::Context) {
    std::thread::spawn(move || {
        let path = match crate::apps::pick_path(false, false, &["json".to_owned()]) {
            Ok(list) => match list.into_iter().next() {
                Some(path) => path,
                None => return,
            },
            Err(e) => return notify_toast(&ctx, Toast::Failure("Could not open the file picker".into(), e)),
        };
        let result = std::fs::read_to_string(&path).map_err(|e| e.to_string()).and_then(|text| parse_import(&text).map_err(|_| "JSON file not in the correct format".to_owned())).and_then(|imported| {
            if imported.is_empty() {
                return Err("No focus categories found to import".into());
            }
            let mut list = categories();
            let titles: Vec<String> = list.iter().map(|c| c.title.to_lowercase()).collect();
            let added: Vec<Category> = imported.iter().filter(|c| !titles.contains(&c.title.to_lowercase())).cloned().collect();
            for mut category in added.clone() {
                category.id = slugify(&category.title, &list.iter().map(|c| c.id.clone()).collect::<Vec<_>>());
                list.push(category);
            }
            save_categories(&list)?;
            Ok((added.len(), imported.len() - added.len()))
        });
        match result {
            Ok((added, skipped)) => {
                let title = if added == 1 { "Imported 1 category".to_owned() } else { format!("Imported {added} categories") };
                notify_toast(&ctx, Toast::Success(if skipped > 0 { format!("{title}, {skipped} already existed") } else { title }));
            }
            Err(e) => notify_toast(&ctx, Toast::Failure("Could not import focus categories".into(), e)),
        }
    });
}

fn pause_actions() -> Vec<Action> {
    PAUSE_CHOICES.iter().map(|(id, title, ms)| Action::new(id, *title, move |host: &mut Host| focus::pause(&focus_ui::host(&host.ctx), *ms))).collect()
}

fn session_command(session: &Session) -> Command {
    let mut cmd = command("focus", "session", if session.title.trim().is_empty() { "Focus Session" } else { &session.title }, ICON, "Focus Session", "Edit Session", |_| Ok(()));
    cmd.subtitle = Some(subtitle(session, focus::now_ms()));
    cmd.view = Some(SessionForm::factory(true));
    let run = |id: &'static str, title: &str, icon: &'static str, op: fn(&focus_ui::EguiHost) -> Result<(), String>| Action::new(id, title, move |host: &mut Host| op(&focus_ui::host(&host.ctx))).icon(icon);
    let middle = if session.status == Status::Paused {
        run("resume", "Resume Session", "play", |h| focus::resume(h))
    } else {
        Action::new("pause", "Pause Session", |_| Ok(())).icon("pause").submenu(pause_actions)
    };
    cmd.actions = vec![vec![
        run("restart", "Restart Session", "rotate-ccw", |h| focus::restart(h)),
        run("complete", "Complete Session", "circle-check", |h| focus::complete(h)),
        middle,
        run("cancel", "Cancel Session", "circle-x", |h| focus::cancel(h)).destructive(),
    ]];
    cmd
}

pub struct Focus;

impl Extension for Focus {
    fn id(&self) -> &'static str {
        "focus"
    }

    fn commands(&self) -> Vec<Command> {
        let id = self.id();
        let run = |key: &str, title: &str, glyph: &'static str, keywords: &[&str], op: fn(&focus_ui::EguiHost) -> Result<(), String>| {
            let mut cmd = command(id, key, title, Icon::Tile(glyph, Tone::Green), "Command", "Run Command", move |host: &mut Host| op(&focus_ui::host(&host.ctx)));
            cmd.keywords = keywords.iter().map(|k| k.to_string()).collect();
            cmd
        };
        let view = |key: &str, title: &str, glyph: &'static str, keywords: &[&str], factory: ViewFactory| {
            let mut cmd = command(id, key, title, Icon::Tile(glyph, Tone::Green), "Command", "Open Command", |_| Ok(()));
            cmd.view = Some(factory);
            cmd.keywords = keywords.iter().map(|k| k.to_string()).collect();
            cmd
        };
        let mut list = vec![
            view("start", "Start Focus Session", "target", &["focus", "block", "distraction", "foco", "bloquear", "distração"], SessionForm::factory(false)),
            run("toggle", "Toggle Focus Session", "target", &["focus", "foco"], |h| {
                if focus::get().is_some() {
                    return focus::complete(h);
                }
                focus::start(h, start_input(&last_form(), &categories(), &preferences()));
                Ok(())
            }),
            run("pause", "Pause Focus Session", "pause", &["foco", "pausar", "pausa"], |h| focus::pause(h, None)),
            run("resume", "Resume Focus Session", "play", &["foco", "retomar"], |h| focus::resume(h)),
            run("complete", "Complete Focus Session", "circle-check", &["foco", "concluir", "finalizar"], |h| focus::complete(h)),
            view("edit", "Edit Focus Session", "pencil", &["foco", "editar", "sessão"], SessionForm::factory(true)),
            view("search-categories", "Search Focus Categories", "search", &["foco", "categorias", "buscar"], Rc::new(|_| Box::new(SearchCategories { version: u64::MAX, list: Vec::new() }) as Box<dyn View>)),
            view("create-category", "Create Focus Category", "plus", &["new focus category", "nova categoria de foco", "criar categoria"], CategoryForm::factory(None)),
        ];
        let mut import_cmd = command(id, "import-categories", "Import Focus Categories", Icon::Tile("file-up", Tone::Green), "Command", "Run Command", |host: &mut Host| {
            import(host.ctx.clone());
            Ok(())
        });
        import_cmd.keywords = vec!["import focus categories".into(), "importar categorias de foco".into()];
        list.push(import_cmd);
        for cmd in &mut list {
            cmd.subtitle = Some("Focus".into());
        }
        if let Some(session) = focus::get() {
            list.push(session_command(&session));
        }
        list
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_website_hosts() {
        assert_eq!(normalize_host("https://www.YouTube.com/watch?v=1").as_deref(), Some("youtube.com"));
        assert_eq!(normalize_host("x.com").as_deref(), Some("x.com"));
        assert_eq!(normalize_host("amazon.*").as_deref(), Some("amazon.*"));
        assert_eq!(normalize_host("not a host"), None);
        assert_eq!(normalize_host("localhost"), None);
    }

    #[test]
    fn merges_categories_apps_and_websites_into_the_session_input() {
        let form = FocusForm { goal: " ".into(), duration_ms: Some(1_500_000), mode: Mode::Block, items: vec!["category:social".into(), r"app:C:\Apps\Code.exe".into(), "site:github.com".into()] };
        let input = start_input(&form, &builtin_categories(), &Preferences::default());
        assert_eq!(input.title, "Focus session");
        assert_eq!(input.apps, vec![r"C:\Apps\Code.exe".to_string()]);
        assert!(input.websites.contains(&"reddit.com".to_string()) && input.websites.contains(&"github.com".to_string()));
        assert_eq!(input.snooze_ms, Some(180_000));
        let none = start_input(&FocusForm { mode: Mode::Allow, ..Default::default() }, &[], &Preferences { snooze_ms: Some(0), ..Default::default() });
        assert_eq!(none.snooze_ms, None);
    }

    #[test]
    fn imports_categories_in_the_reference_json_shapes() {
        let list = parse_import(r#"[{"title":"Dev","websiteHosts":["https://github.com"],"applicationIds":["code.exe"]}]"#).unwrap();
        assert_eq!((list[0].title.as_str(), list[0].icon.as_str(), list[0].apps.clone(), list[0].websites.clone()), ("Dev", "target", vec!["code.exe".to_string()], vec!["github.com".to_string()]));
        assert_eq!(parse_import(r#"{"focusCategories":[{"title":"Fun","icon":"game","websites":["x.com"],"apps":[]}]}"#).unwrap()[0].icon, "game");
        assert_eq!(parse_import(r#"{"title":"One"}"#).unwrap().len(), 1);
        assert!(parse_import(r#"[{"websites":[]}]"#).is_err());
    }

    #[test]
    fn makes_unique_category_ids() {
        assert_eq!(slugify("Deep Work!", &[]), "deep-work");
        assert_eq!(slugify("Social", &["social".into(), "social-2".into()]), "social-3");
    }

    #[test]
    fn reads_forms_saved_by_the_tauri_app() {
        let form: FocusForm = serde_json::from_str(r#"{"goal":"Write","durationMs":1500000,"mode":"allow","items":["site:x.com"]}"#).unwrap();
        assert_eq!(form, FocusForm { goal: "Write".into(), duration_ms: Some(1_500_000), mode: Mode::Allow, items: vec!["site:x.com".into()] });
        let prefs: Preferences = serde_json::from_str(r#"{"playSound":false}"#).unwrap();
        assert_eq!(prefs, Preferences { play_sound: false, ..Default::default() });
    }
}
