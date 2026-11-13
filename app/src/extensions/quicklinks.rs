use super::placeholders::{attributes, date_text, uuid, PLACEHOLDER};
use super::web::encode_query;
use super::{command, GENERATION};
use crate::extension::{notify, Accessory, Action, Command, Empty, Extension, Field, FieldKind, Form, Host, Icon, Item, Request, Section, Shortcut, Toast, Tone, View, ViewFactory};
use egui::{pos2, Key, Rect, Ui};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::LazyLock;

const ICON: Icon = Icon::Tile("link-2", Tone::Purple);
static VERSION: AtomicU64 = AtomicU64::new(0);
static BROWSERS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)\b(chrome|chromium|edge|firefox|brave|opera|vivaldi|helium|arc|zen|librewolf|waterfox|thorium|floorp|tor browser|zen browser)\b").unwrap());
static URL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^[a-z][a-z0-9+.-]*://").unwrap());

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Quicklink {
    pub id: String,
    pub name: String,
    pub link: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app_name: Option<String>,
}

#[derive(PartialEq, Debug)]
pub struct LinkArgument {
    pub name: String,
    pub default: Option<String>,
}

fn path() -> std::path::PathBuf {
    crate::data_dir().join("quicklinks.json")
}

pub fn load() -> Vec<Quicklink> {
    crate::store::read_json(&path())
}

fn save(list: &[Quicklink]) -> Result<(), String> {
    crate::store::write_json(&path(), &list).map_err(|e| e.to_string())?;
    VERSION.fetch_add(1, Ordering::SeqCst);
    GENERATION.fetch_add(1, Ordering::SeqCst);
    Ok(())
}

pub fn link_arguments(link: &str) -> Vec<LinkArgument> {
    let mut found: Vec<LinkArgument> = Vec::new();
    for caps in PLACEHOLDER.captures_iter(link) {
        if !caps[1].eq_ignore_ascii_case("argument") {
            continue;
        }
        let attrs = attributes(&caps[2]);
        let name = attrs.get("name").cloned().unwrap_or_else(|| "Query".into());
        if !found.iter().any(|a| a.name == name) {
            found.push(LinkArgument { name, default: attrs.get("default").cloned() });
        }
    }
    found
}

pub fn fill_link(link: &str, values: &HashMap<String, String>, clipboard: Option<&str>, now_ms: f64, make_uuid: &dyn Fn() -> String) -> String {
    let is_url = URL.is_match(link.trim());
    let encode = |value: &str| if is_url { encode_query(value) } else { value.to_owned() };
    PLACEHOLDER
        .replace_all(link, |caps: &regex::Captures| {
            let attrs = attributes(&caps[2]);
            let kind = caps[1].to_lowercase();
            match kind.as_str() {
                "argument" => {
                    let name = attrs.get("name").cloned().unwrap_or_else(|| "Query".into());
                    let value = values.get(&name).filter(|v| !v.is_empty()).or(attrs.get("default")).cloned().unwrap_or_default();
                    encode(&value)
                }
                "clipboard" => encode(clipboard.unwrap_or("")),
                "uuid" => make_uuid(),
                other => date_text(other, now_ms, "en-US").map_or_else(|| caps[0].to_owned(), |d| encode(&d)),
            }
        })
        .into_owned()
}

pub fn display_link(link: &str, values: &HashMap<String, String>) -> String {
    PLACEHOLDER
        .replace_all(link, |caps: &regex::Captures| {
            if !caps[1].eq_ignore_ascii_case("argument") {
                return caps[0].to_owned();
            }
            let name = attributes(&caps[2]).get("name").cloned().unwrap_or_else(|| "Query".into());
            values.get(&name).filter(|v| !v.is_empty()).cloned().unwrap_or_else(|| "_".into())
        })
        .into_owned()
}

pub fn favicon_url(link: &str) -> Option<String> {
    let shown = display_link(link, &HashMap::new());
    let rest = shown.trim().strip_prefix("https://").or_else(|| shown.trim().strip_prefix("http://"))?;
    let host = rest.split(['/', '?', '#']).next().filter(|h| !h.is_empty())?;
    let host = host.rsplit('@').next().unwrap_or(host).split(':').next().unwrap_or(host).to_lowercase();
    Some(format!("https://www.google.com/s2/favicons?domain={}&sz=64", encode_query(&host)))
}

#[derive(PartialEq, Debug)]
pub struct Import {
    pub name: String,
    pub link: String,
    pub open_with: Option<String>,
}

pub fn parse_import(json: &str) -> Result<Vec<Import>, String> {
    let data: serde_json::Value = serde_json::from_str(json.trim_start_matches('\u{feff}')).map_err(|e| e.to_string())?;
    let list = match &data {
        serde_json::Value::Array(items) => items.clone(),
        _ => data.get("quicklinks").and_then(|q| q.as_array()).cloned().unwrap_or_else(|| vec![data.clone()]),
    };
    list.iter()
        .map(|raw| {
            let text = |key: &str| raw.get(key).and_then(|v| v.as_str()).map(str::to_owned);
            let name = text("name").ok_or("Invalid quicklink import format")?;
            Ok(Import { name, link: text("link").or_else(|| text("url")).unwrap_or_default(), open_with: text("openWith").or_else(|| text("applicationID")) })
        })
        .collect()
}

pub fn merge(existing: Vec<Quicklink>, incoming: Vec<Import>, apps: &[(String, String)], new_id: &dyn Fn() -> String) -> (Vec<Quicklink>, usize, usize) {
    let mut list = existing;
    let (mut added, mut skipped) = (0, 0);
    for item in incoming {
        if list.iter().any(|q| q.link == item.link) {
            skipped += 1;
            continue;
        }
        let app = item.open_with.as_ref().and_then(|w| apps.iter().find(|(name, path)| name.eq_ignore_ascii_case(w) || path == w));
        list.push(Quicklink { id: new_id(), name: item.name, link: item.link.trim().to_owned(), app_path: app.map(|a| a.1.clone()), app_name: app.map(|a| a.0.clone()) });
        added += 1;
    }
    (list, added, skipped)
}

pub fn export(list: &[Quicklink]) -> String {
    let items: Vec<serde_json::Value> = list
        .iter()
        .map(|q| match &q.app_name {
            Some(app) => serde_json::json!({ "name": q.name, "link": q.link, "openWith": app }),
            None => serde_json::json!({ "name": q.name, "link": q.link }),
        })
        .collect();
    serde_json::to_string_pretty(&items).unwrap_or_default()
}

fn open(quicklink: &Quicklink, values: &HashMap<String, String>) -> Result<(), String> {
    if let Some(missing) = link_arguments(&quicklink.link).into_iter().find(|a| a.default.is_none() && values.get(&a.name).is_none_or(|v| v.trim().is_empty())) {
        return Err(format!("Enter a value for {}", missing.name));
    }
    let clipboard = quicklink.link.to_lowercase().contains("{clipboard").then(crate::clipboard::read_clipboard_text).flatten();
    let url = fill_link(&quicklink.link, values, clipboard.as_deref(), crate::store::now_ms() as f64, &uuid);
    crate::apps::open_url(&url, quicklink.app_path.as_deref())
}

fn icon_of(link: &str) -> Icon {
    favicon_url(link).map_or(Icon::Boxed("link"), Icon::Url)
}

fn browsers() -> Vec<(String, String)> {
    super::APPS.lock().unwrap_or_else(|e| e.into_inner()).iter().filter(|a| BROWSERS.is_match(&a.name) && !a.path.starts_with("shell:")).map(|a| (a.name.clone(), a.path.clone())).collect()
}

struct QuicklinkForm {
    id: Option<String>,
    form: Form,
}

impl QuicklinkForm {
    fn factory(initial: Option<Quicklink>, link: String) -> ViewFactory {
        Rc::new(move |_| {
            let editing = initial.as_ref().is_some_and(|q| load().iter().any(|e| e.id == q.id));
            let title = if editing { "Edit Quicklink" } else { "Create Quicklink" };
            let mut options = vec![(String::new(), "Default Browser".to_string(), Icon::Glyph("external-link"))];
            options.extend(browsers().into_iter().map(|(name, path)| (path.clone(), name, Icon::File(path))));
            if let Some(q) = &initial {
                if let Some(path) = q.app_path.as_ref().filter(|p| !options.iter().any(|o| o.0 == **p)) {
                    options.push((path.clone(), q.app_name.clone().unwrap_or_else(|| path.clone()), Icon::File(path.clone())));
                }
            }
            let fields = vec![
                Field::new("Link", FieldKind::Area(100.0), initial.as_ref().map_or(link.clone(), |q| q.link.clone()))
                    .placeholder("https://www.google.com/search?q={argument}")
                    .help("Use {argument} to ask for text when you open it, or {clipboard}, {date}, {time} and {uuid} to insert them."),
                Field::new("Name", FieldKind::Text, initial.as_ref().map_or(String::new(), |q| q.name.clone())).placeholder("Quicklink name"),
                Field::new("Open With", FieldKind::Choice(options), initial.as_ref().and_then(|q| q.app_path.clone()).unwrap_or_default()),
            ];
            Box::new(QuicklinkForm { id: initial.as_ref().map(|q| q.id.clone()), form: Form { title: title.into(), submit: "Save Quicklink", icon: ICON, fields, focus: Some(0) } })
        })
    }
}

impl View for QuicklinkForm {
    fn placeholder(&self) -> String {
        String::new()
    }

    fn footer(&self) -> (String, Icon) {
        (self.form.title.clone(), ICON)
    }

    fn sections(&mut self) -> Vec<Section> {
        Vec::new()
    }

    fn form(&mut self) -> Option<&mut Form> {
        Some(&mut self.form)
    }

    fn submit(&mut self, host: &mut Host) -> Result<bool, String> {
        let link = self.form.fields[0].value.trim().to_owned();
        let name = self.form.fields[1].value.trim().to_owned();
        self.form.fields[0].error = link.is_empty().then(|| "Link is required".into());
        self.form.fields[1].error = name.is_empty().then(|| "Name is required".into());
        if let Some(i) = self.form.fields.iter().position(|f| f.error.is_some()) {
            self.form.focus = Some(i);
            return Ok(false);
        }
        let app_path = Some(self.form.fields[2].value.clone()).filter(|p| !p.is_empty());
        let app_name = match &self.form.fields[2].kind {
            FieldKind::Choice(options) => app_path.as_ref().and_then(|p| options.iter().find(|o| o.0 == *p)).map(|o| o.1.clone()),
            _ => None,
        };
        let list = load();
        let id = self.id.clone().unwrap_or_else(uuid);
        let existed = list.iter().any(|q| q.id == id);
        let quicklink = Quicklink { id: id.clone(), name, link, app_path, app_name };
        let next = if existed { list.into_iter().map(|q| if q.id == id { quicklink.clone() } else { q }).collect() } else { [list, vec![quicklink]].concat() };
        save(&next)?;
        host.toast = Some(Toast::Success(if existed { "Updated Quicklink" } else { "Created Quicklink" }.into()));
        Ok(true)
    }
}

struct SearchQuicklinks {
    list: Vec<Quicklink>,
    version: u64,
    values: HashMap<String, HashMap<String, String>>,
    select: Option<String>,
}

fn push(factory: ViewFactory) -> impl Fn(&mut Host) -> Result<(), String> {
    move |host| {
        host.requests.push(Request::Push(factory.clone()));
        Ok(())
    }
}

fn create_action() -> Action {
    Action::new("create", "Create Quicklink", push(QuicklinkForm::factory(None, String::new()))).icon("plus").shortcut(Shortcut::key(Key::N).ctrl()).keep_open()
}

impl SearchQuicklinks {
    fn new(select: Option<String>) -> Self {
        Self { list: Vec::new(), version: u64::MAX, values: HashMap::new(), select }
    }

    fn actions(&self, q: &Quicklink) -> Vec<Vec<Action>> {
        let values = self.values.get(&q.id).cloned().unwrap_or_default();
        let (opened, copied) = (q.clone(), q.clone());
        let copy_values = values.clone();
        let delete_id = q.id.clone();
        let mut duplicate = q.clone();
        duplicate.id = uuid();
        duplicate.name = format!("{} Copy", q.name);
        vec![
            vec![
                Action::new("open", "Open Quicklink", move |_| open(&opened, &values)).icon("external-link"),
                Action::new("copy-link", "Copy Link", move |host| {
                    let clipboard = crate::clipboard::read_clipboard_text();
                    host.copy_and_close(&fill_link(&copied.link, &copy_values, clipboard.as_deref(), crate::store::now_ms() as f64, &uuid), "Copied Link to Clipboard");
                    Ok(())
                })
                .icon("clipboard")
                .shortcut(Shortcut::key(Key::C).ctrl().shift()),
            ],
            vec![
                Action::new("edit", "Edit Quicklink", push(QuicklinkForm::factory(Some(q.clone()), String::new()))).icon("pencil").shortcut(Shortcut::key(Key::E).ctrl()).keep_open(),
                Action::new("duplicate", "Duplicate Quicklink", push(QuicklinkForm::factory(Some(duplicate), String::new()))).icon("copy-plus").keep_open(),
                create_action(),
            ],
            vec![Action::new("delete", "Delete Quicklink", move |_| save(&load().into_iter().filter(|e| e.id != delete_id).collect::<Vec<_>>()))
                .icon("trash-2")
                .destructive()
                .keep_open()
                .shortcut(Shortcut::key(Key::D).ctrl())
                .confirm("Delete Quicklink?", &format!("“{}” will be deleted. This can't be undone.", q.name), "Delete Quicklink")],
        ]
    }
}

impl View for SearchQuicklinks {
    fn placeholder(&self) -> String {
        "Search for Quicklinks…".into()
    }

    fn footer(&self) -> (String, Icon) {
        ("Search Quicklinks".into(), ICON)
    }

    fn empty(&self) -> Empty {
        Empty { title: "No Quicklinks yet. Press Ctrl+N to create one.".into(), description: None }
    }

    fn empty_actions(&self) -> Vec<Vec<Action>> {
        vec![vec![create_action()]]
    }

    fn initial_selection(&self) -> Option<String> {
        self.select.clone()
    }

    fn item_argument(&self, id: &str) -> Option<(String, String)> {
        let q = self.list.iter().find(|q| q.id == id)?;
        let first = link_arguments(&q.link).into_iter().next()?;
        let value = self.values.get(id).and_then(|v| v.get(&first.name)).cloned().unwrap_or_default();
        Some((first.name, value))
    }

    fn set_argument(&mut self, id: &str, value: &str) {
        if let Some((name, _)) = self.item_argument(id) {
            self.values.entry(id.to_owned()).or_default().insert(name, value.to_owned());
        }
    }

    fn has_detail(&self) -> bool {
        true
    }

    fn detail(&mut self, ui: &mut Ui, rect: Rect, id: &str) {
        let Some(q) = self.list.iter().find(|q| q.id == id) else { return };
        let rows = vec![("Name".to_string(), q.name.clone()), ("Link".to_string(), q.link.clone()), ("Open With".to_string(), q.app_name.clone().unwrap_or_else(|| "Default Browser".into()))];
        let info = crate::root::metadata(ui, rect, &rows);
        let area = Rect::from_min_max(rect.min, pos2(rect.right(), info.top())).shrink(16.0);
        let preview = display_link(&q.link, self.values.get(id).unwrap_or(&HashMap::new()));
        let galley = crate::root::wrapped(ui, &preview, egui::FontId::monospace(13.0), crate::theme::FG, area.width());
        ui.painter().with_clip_rect(area).galley(area.min, galley, crate::theme::FG);
    }

    fn sections(&mut self) -> Vec<Section> {
        let version = VERSION.load(Ordering::SeqCst);
        if version != self.version {
            self.version = version;
            self.list = load();
        }
        let items = self.list.iter().map(|q| Item { id: q.id.clone(), title: q.name.clone(), subtitle: None, icon: icon_of(&q.link), accessories: Vec::<Accessory>::new(), keywords: vec![q.link.clone()], actions: self.actions(q) }).collect();
        vec![Section { title: "Quicklinks".into(), items }]
    }
}

fn import(host: &mut Host) -> Result<(), String> {
    host.hide(false);
    let ctx = host.ctx.clone();
    std::thread::spawn(move || {
        let result = (|| -> Result<Option<String>, String> {
            let Some(path) = crate::apps::pick_path(false, false, &["json".to_string()])?.into_iter().next() else { return Ok(None) };
            let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
            let incoming = parse_import(&text).map_err(|_| "JSON file not in the correct format".to_string())?;
            let apps: Vec<(String, String)> = super::APPS.lock().unwrap_or_else(|e| e.into_inner()).iter().map(|a| (a.name.clone(), a.path.clone())).collect();
            let (list, added, skipped) = merge(load(), incoming, &apps, &uuid);
            save(&list)?;
            let plural = |n: usize, word: &str| format!("{n} {word}{}", if n == 1 { "" } else { "s" });
            let parts: Vec<String> = [(added > 0).then(|| format!("{} added", plural(added, "quicklink"))), (skipped > 0).then(|| format!("{} skipped", plural(skipped, "duplicate")))].into_iter().flatten().collect();
            Ok(Some(if parts.is_empty() { "Import successful".into() } else { format!("Import successful: {}", parts.join(", ")) }))
        })();
        match result {
            Ok(Some(title)) => notify(&ctx, Toast::Success(title)),
            Ok(None) => {}
            Err(e) => notify(&ctx, Toast::Failure(e, String::new())),
        }
    });
    Ok(())
}

fn export_all(host: &mut Host) -> Result<(), String> {
    let list = load();
    if list.is_empty() {
        return Err("There are no quicklinks to export".into());
    }
    host.hide(false);
    let ctx = host.ctx.clone();
    std::thread::spawn(move || {
        let result = (|| -> Result<bool, String> {
            let Some(folder) = crate::apps::pick_path(true, false, &[])?.into_iter().next() else { return Ok(false) };
            let date = crate::icu::format_pattern(crate::store::now_ms() as f64, "en-US", "yyyy-MM-dd", None);
            let path = format!("{}\\Quicklinks {date}.json", folder.trim_end_matches('\\'));
            std::fs::write(&path, export(&list)).map_err(|e| e.to_string())?;
            crate::apps::show_in_explorer(&path)?;
            Ok(true)
        })();
        match result {
            Ok(true) => notify(&ctx, Toast::Success("Exported quicklinks".into())),
            Ok(false) => {}
            Err(e) => notify(&ctx, Toast::Failure(e, String::new())),
        }
    });
    Ok(())
}

pub struct Quicklinks;

impl Extension for Quicklinks {
    fn id(&self) -> &'static str {
        "quicklinks"
    }

    fn commands(&self) -> Vec<Command> {
        let keywords = ["link", "url", "bookmark", "links", "atalhos"].map(String::from).to_vec();
        let mut create = command(self.id(), "create", "Create Quicklink", ICON, "Command", "Open Command", |_| Ok(()));
        create.view = Some(QuicklinkForm::factory(None, String::new()));
        create.fallback = Some(Rc::new(|text, host| {
            host.requests.push(Request::Push(QuicklinkForm::factory(None, text.to_owned())));
            Ok(())
        }));
        create.keywords = keywords.clone();
        let mut search = command(self.id(), "search", "Search Quicklinks", ICON, "Command", "Open Command", |_| Ok(()));
        search.view = Some(Rc::new(|_| Box::new(SearchQuicklinks::new(None))));
        search.keywords = keywords;
        let mut import_cmd = command(self.id(), "import", "Import Quicklinks", Icon::Tile("download", Tone::Purple), "Command", "Run Command", import);
        import_cmd.keywords = ["link", "json", "import", "importar"].map(String::from).to_vec();
        let mut export_cmd = command(self.id(), "export", "Export Quicklinks", Icon::Tile("upload", Tone::Purple), "Command", "Run Command", export_all);
        export_cmd.keywords = ["link", "json", "export", "backup", "exportar"].map(String::from).to_vec();
        let mut commands = vec![create, search, import_cmd, export_cmd];
        for cmd in &mut commands {
            cmd.subtitle = Some("Quicklinks".into());
        }
        for q in load() {
            let mut cmd = command(self.id(), &q.id, &q.name, icon_of(&q.link), "Quicklink", "Open Quicklink", |_| Ok(()));
            cmd.subtitle = Some("Quicklink".into());
            cmd.keywords = vec![q.link.clone()];
            match link_arguments(&q.link).into_iter().next() {
                None => {
                    let opened = q.clone();
                    cmd.run = Rc::new(move |_| open(&opened, &HashMap::new()));
                }
                Some(first) => {
                    let select = q.id.clone();
                    cmd.view = Some(Rc::new(move |_| Box::new(SearchQuicklinks::new(Some(select.clone())))));
                    let (opened, name) = (q.clone(), first.name);
                    cmd.fallback = Some(Rc::new(move |text, _| open(&opened, &HashMap::from([(name.clone(), text.to_owned())]))));
                }
            }
            commands.push(cmd);
        }
        commands
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn arg(name: &str, default: Option<&str>) -> LinkArgument {
        LinkArgument { name: name.into(), default: default.map(String::from) }
    }

    #[test]
    fn lists_named_arguments_once_with_defaults() {
        assert_eq!(link_arguments(r#"https://google.com/search?q={argument name="Query"}"#), [arg("Query", None)]);
        assert_eq!(link_arguments(r#"x/{argument name="A" default="1"}/{argument name="A"}/{argument}"#), [arg("A", Some("1")), arg("Query", None)]);
        assert!(link_arguments("https://example.com").is_empty());
    }

    #[test]
    fn fills_arguments_and_context_encoding_values_in_urls_only() {
        let values = HashMap::from([("Query".to_string(), "a b&c".to_string())]);
        let now = crate::extensions::calculator::history::days_from_civil(2026, 10, 18) as f64 * 86_400_000.0 + 14.0 * 3_600_000.0;
        let local = now - crate::icu::local_offset_minutes(now) as f64 * 60_000.0;
        let none = HashMap::new();
        let fixed = || "u1".to_string();
        assert_eq!(fill_link(r#"https://google.com/search?q={argument name="Query"}"#, &values, None, local, &fixed), "https://google.com/search?q=a%20b%26c");
        assert_eq!(fill_link(r#"C:\notes\{argument default="today"}.txt"#, &none, None, local, &fixed), r"C:\notes\today.txt");
        assert_eq!(fill_link("https://x.com/{clipboard}?d={date}", &none, Some("hi there"), local, &fixed), "https://x.com/hi%20there?d=Oct%2018%2C%202026");
        assert_eq!(fill_link("id={uuid}&{unknown}", &none, None, local, &fixed), "id=u1&{unknown}");
    }

    #[test]
    fn previews_links_and_finds_favicons_for_web_links_only() {
        assert_eq!(display_link(r#"https://google.com/search?q={argument name="Query"}"#, &HashMap::new()), "https://google.com/search?q=_");
        assert_eq!(display_link("https://x.com/{argument}?d={date}", &HashMap::from([("Query".to_string(), "a b".to_string())])), "https://x.com/a b?d={date}");
        assert_eq!(favicon_url("https://www.github.com/x").as_deref(), Some("https://www.google.com/s2/favicons?domain=www.github.com&sz=64"));
        assert_eq!(favicon_url(r"C:\Users"), None);
    }

    #[test]
    fn imports_reference_shapes_skips_duplicates_and_round_trips_exports() {
        assert_eq!(parse_import(r#"{"name":"A","url":"https://a.com","applicationID":"Firefox"}"#).unwrap(), [Import { name: "A".into(), link: "https://a.com".into(), open_with: Some("Firefox".into()) }]);
        assert_eq!(parse_import(r#"{"quicklinks":[{"name":"B","link":"https://b.com"}]}"#).unwrap(), [Import { name: "B".into(), link: "https://b.com".into(), open_with: None }]);
        assert!(parse_import(r#"[{"link":"x"}]"#).is_err());
        let apps = [("Firefox".to_string(), r"C:\ff.exe".to_string())];
        let existing = vec![Quicklink { id: "1".into(), name: "Old".into(), link: "https://a.com".into(), app_path: None, app_name: None }];
        let incoming = parse_import(r#"[{"name":"A","link":"https://a.com"},{"name":"C","link":" https://c.com ","openWith":"firefox"}]"#).unwrap();
        let (list, added, skipped) = merge(existing, incoming, &apps, &|| "new".to_string());
        assert_eq!((added, skipped), (1, 1));
        assert_eq!(list[1], Quicklink { id: "new".into(), name: "C".into(), link: "https://c.com".into(), app_path: Some(r"C:\ff.exe".into()), app_name: Some("Firefox".into()) });
        assert_eq!(parse_import(&export(&list)).unwrap().iter().map(|i| (i.name.as_str(), i.open_with.as_deref())).collect::<Vec<_>>(), [("Old", None), ("C", Some("Firefox"))]);
    }
}
