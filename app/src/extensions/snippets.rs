use super::command;
use super::placeholders::{attributes, date_text, uuid, PLACEHOLDER};
use crate::extension::{frontmost_app, notify, paste_title, Action, Command, Empty, Extension, Field, FieldKind, Form, Host, Icon, Item, Request, Section, Shortcut, Toast, Tone, View, ViewFactory};
use crate::icu::{self, Style};
use egui::{pos2, Key, Rect, Ui};
use serde::{Deserialize, Serialize};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

const ICON: Icon = Icon::Tile("text-quote", Tone::Blue);
const CURSOR: char = '\u{0}';
static VERSION: AtomicU64 = AtomicU64::new(0);

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Snippet {
    pub id: String,
    pub name: String,
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keyword: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pinned: Option<bool>,
    pub created_at: u64,
    pub modified_at: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copied_at: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copy_count: Option<u64>,
}

fn path() -> std::path::PathBuf {
    crate::data_dir().join("snippets.json")
}

pub fn load() -> Vec<Snippet> {
    crate::store::read_json(&path())
}

fn save(list: &[Snippet]) -> Result<(), String> {
    crate::store::write_json(&path(), &list).map_err(|e| e.to_string())?;
    crate::snippets::load(&serde_json::to_value(list).unwrap_or_default());
    VERSION.fetch_add(1, Ordering::SeqCst);
    Ok(())
}

pub struct Context<'a> {
    pub clipboard: &'a dyn Fn(usize) -> String,
    pub snippets: &'a [Snippet],
    pub now_ms: f64,
    pub locale: &'a str,
    pub uuid: &'a dyn Fn() -> String,
}

fn fill(text: &str, context: &Context, nested: bool) -> String {
    PLACEHOLDER
        .replace_all(text, |caps: &regex::Captures| {
            let kind = caps[1].to_lowercase();
            let attrs = attributes(&caps[2]);
            if let Some(date) = date_text(&kind, context.now_ms, context.locale) {
                return date;
            }
            match kind.as_str() {
                "clipboard" => (context.clipboard)(attrs.get("offset").and_then(|o| o.parse::<i64>().ok()).unwrap_or(0).max(0) as usize),
                "uuid" => (context.uuid)(),
                "cursor" => CURSOR.to_string(),
                "snippet" if !nested => {
                    let name = attrs.get("name").map(|n| n.to_lowercase()).unwrap_or_default();
                    context.snippets.iter().find(|s| s.name.to_lowercase() == name).map_or_else(|| caps[0].to_owned(), |s| fill(&s.text, context, true).replace(CURSOR, ""))
                }
                _ => caps[0].to_owned(),
            }
        })
        .into_owned()
}

pub fn resolve(text: &str, context: &Context) -> (String, usize) {
    let filled = fill(text, context, false).replace("\r\n", "\n");
    let at = filled.find(CURSOR);
    let clean = filled.replace(CURSOR, "");
    let back = at.map_or(0, |at| filled[at..].chars().filter(|c| *c != CURSOR).count());
    (clean, back)
}

fn clipboard_at(offset: usize) -> String {
    if offset == 0 {
        return crate::clipboard::read_clipboard_text().unwrap_or_default();
    }
    let entries = super::clipboard_history::HISTORY.get().map(|h| h.list()).unwrap_or_default();
    entries.into_iter().filter_map(|e| match e.content {
        crate::clipboard::Content::Text { text } => Some(text),
        _ => None,
    }).nth(offset).unwrap_or_default()
}

fn resolve_now(snippet: &Snippet, all: &[Snippet]) -> (String, usize) {
    resolve(&snippet.text, &Context { clipboard: &clipboard_at, snippets: all, now_ms: crate::store::now_ms() as f64, locale: super::calculator::time::locale(), uuid: &uuid })
}

pub fn keyword_in_use(keyword: &str, list: &[Snippet], except: Option<&str>) -> bool {
    let trimmed = keyword.trim();
    !trimmed.is_empty() && list.iter().any(|s| Some(s.id.as_str()) != except && s.keyword.as_deref().map(str::trim) == Some(trimmed))
}

pub fn parse_import(json: &str) -> Result<Vec<(String, String, Option<String>)>, String> {
    let invalid = || "Invalid snippet import format".to_string();
    let data: serde_json::Value = serde_json::from_str(json.trim_start_matches('\u{feff}')).map_err(|_| invalid())?;
    let list = match &data {
        serde_json::Value::Array(items) => items.clone(),
        _ => data.get("snippets").and_then(|s| s.as_array()).cloned().unwrap_or_else(|| vec![data.clone()]),
    };
    list.iter()
        .map(|raw| {
            let text_of = |a: &str, b: &str| raw.get(a).and_then(|v| v.as_str()).or_else(|| raw.get(b).and_then(|v| v.as_str())).unwrap_or("").to_owned();
            let (name, text) = (text_of("name", "title"), text_of("text", "snippet"));
            if name.trim().is_empty() || text.is_empty() {
                return Err(invalid());
            }
            let keyword = raw.get("keyword").and_then(|v| v.as_str()).map(str::trim).filter(|k| !k.is_empty()).map(str::to_owned);
            Ok((name.trim().to_owned(), text, keyword))
        })
        .collect()
}

pub fn merge(existing: Vec<Snippet>, incoming: Vec<(String, String, Option<String>)>, now: u64, new_id: &dyn Fn() -> String) -> (Vec<Snippet>, usize, usize) {
    let mut result = existing;
    let (mut added, mut skipped) = (0, 0);
    for (name, text, keyword) in incoming {
        let duplicate = result.iter().any(|s| s.name == name && s.text == text);
        if duplicate || keyword.as_deref().is_some_and(|k| keyword_in_use(k, &result, None)) {
            skipped += 1;
            continue;
        }
        result.push(Snippet { id: new_id(), name, text, keyword, created_at: now, modified_at: now, ..Default::default() });
        added += 1;
    }
    (result, added, skipped)
}

pub fn export(list: &[Snippet]) -> String {
    let items: Vec<serde_json::Value> = list
        .iter()
        .map(|s| match &s.keyword {
            Some(k) => serde_json::json!({ "name": s.name, "text": s.text, "keyword": k }),
            None => serde_json::json!({ "name": s.name, "text": s.text }),
        })
        .collect();
    serde_json::to_string_pretty(&items).unwrap_or_default()
}

pub fn start() {
    crate::snippets::init(
        |id| {
            let list = load();
            if let Some(snippet) = list.iter().find(|s| s.id == id) {
                let (text, back) = resolve_now(snippet, &list);
                crate::snippets::type_text(&text, back);
            }
        },
        &serde_json::to_value(load()).unwrap_or_default(),
    );
}

fn mark_used(id: &str) {
    let now = crate::store::now_ms();
    let list = load().into_iter().map(|s| if s.id == id { Snippet { copied_at: Some(now), copy_count: Some(s.copy_count.unwrap_or(0) + 1), ..s } } else { s }).collect::<Vec<_>>();
    if let Err(e) = save(&list) {
        eprintln!("openray: could not update snippet: {e}");
    }
}

struct SnippetForm {
    id: Option<String>,
    form: Form,
}

impl SnippetForm {
    fn factory(initial: Option<Snippet>, title: &'static str) -> ViewFactory {
        Rc::new(move |_| {
            let initial = initial.clone().unwrap_or_default();
            let fields = vec![
                Field::new("Snippet", FieldKind::Area(120.0), initial.text.clone()).mono().help("Include {clipboard}, {date}, {time}, {datetime}, {day}, {uuid}, {cursor} or {snippet name=\"…\"} to fill them in when it's inserted."),
                Field::new("Name", FieldKind::Text, initial.name.clone()).placeholder("Snippet name"),
                Field::new("Keyword", FieldKind::Text, initial.keyword.clone().unwrap_or_default()).placeholder("Optional keyword").mono().help("Type it in any app to replace it with the snippet."),
            ];
            let editing = !initial.id.is_empty();
            let focus = if editing && !initial.name.is_empty() { 1 } else { 0 };
            Box::new(SnippetForm { id: editing.then(|| initial.id.clone()), form: Form { title: title.into(), submit: "Save Snippet", icon: ICON, fields, focus: Some(focus) } })
        })
    }
}

impl View for SnippetForm {
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
        let list = load();
        let text = self.form.fields[0].value.clone();
        let name = self.form.fields[1].value.trim().to_owned();
        let keyword = self.form.fields[2].value.trim().to_owned();
        self.form.fields[0].error = text.is_empty().then(|| "Snippet cannot be empty".into());
        self.form.fields[1].error = name.is_empty().then(|| "Name cannot be empty".into());
        self.form.fields[2].error = keyword_in_use(&keyword, &list, self.id.as_deref()).then(|| "This keyword is already in use".into());
        if let Some(i) = self.form.fields.iter().position(|f| f.error.is_some()) {
            self.form.focus = Some(i);
            return Ok(false);
        }
        let now = crate::store::now_ms();
        let base = self.id.as_ref().and_then(|id| list.iter().find(|s| s.id == *id)).cloned();
        let snippet = Snippet { id: base.as_ref().map_or_else(uuid, |b| b.id.clone()), name, text, keyword: Some(keyword).filter(|k| !k.is_empty()), created_at: base.as_ref().map_or(now, |b| b.created_at), modified_at: now, ..base.clone().unwrap_or_default() };
        let next = if base.is_some() { list.into_iter().map(|s| if s.id == snippet.id { snippet.clone() } else { s }).collect() } else { [list, vec![snippet]].concat() };
        save(&next)?;
        host.toast = Some(Toast::Success(if base.is_some() { "Snippet updated" } else { "Created snippet" }.into()));
        Ok(true)
    }
}

fn push(factory: ViewFactory) -> impl Fn(&mut Host) -> Result<(), String> {
    move |host| {
        host.requests.push(Request::Push(factory.clone()));
        Ok(())
    }
}

fn create_action() -> Action {
    Action::new("create", "Create Snippet", push(SnippetForm::factory(None, "Create Snippet"))).icon("plus").shortcut(Shortcut::key(Key::N).ctrl()).keep_open()
}

fn update(toast: &'static str, change: impl Fn(Vec<Snippet>) -> Vec<Snippet> + 'static) -> impl Fn(&mut Host) -> Result<(), String> {
    move |host| {
        save(&change(load()))?;
        host.toast = Some(Toast::Success(toast.into()));
        Ok(())
    }
}

fn when(ms: Option<u64>) -> String {
    ms.map_or_else(|| "Never".into(), |ms| icu::format_date(ms as f64, "en-US", Style::Medium, Style::Short, None))
}

struct SearchSnippets {
    list: Vec<Snippet>,
    version: u64,
    app: Option<String>,
}

impl SearchSnippets {
    fn actions(&self, s: &Snippet) -> Vec<Vec<Action>> {
        let (paste, copy) = (s.clone(), s.clone());
        let (pin_id, delete_id) = (s.id.clone(), s.id.clone());
        let pinned = s.pinned.unwrap_or(false);
        let count = self.list.len();
        let mut duplicate = s.clone();
        duplicate.id = String::new();
        duplicate.name = format!("{} Copy", s.name);
        vec![
            vec![
                Action::new("paste", paste_title(&self.app), move |host| {
                    let all = load();
                    let (text, back) = resolve_now(&paste, &all);
                    host.paste(&text)?;
                    if back > 0 {
                        std::thread::spawn(move || {
                            std::thread::sleep(std::time::Duration::from_millis(250));
                            crate::snippets::type_text("", back);
                        });
                    }
                    mark_used(&paste.id);
                    Ok(())
                })
                .icon("clipboard-paste"),
                Action::new("copy", "Copy to Clipboard", move |host| {
                    let all = load();
                    let (text, _) = resolve_now(&copy, &all);
                    mark_used(&copy.id);
                    host.copy_and_close(&text, "Copied snippet to clipboard");
                    Ok(())
                })
                .icon("clipboard")
                .shortcut(Shortcut::key(Key::Enter).ctrl()),
            ],
            vec![
                Action::new(if pinned { "unpin" } else { "pin" }, if pinned { "Unpin Snippet" } else { "Pin Snippet" }, update(if pinned { "Unpinned Snippet" } else { "Pinned Snippet" }, move |l| l.into_iter().map(|x| if x.id == pin_id { Snippet { pinned: Some(!pinned), ..x } } else { x }).collect()))
                    .icon(if pinned { "pin-off" } else { "pin" })
                    .shortcut(Shortcut::key(Key::Period).ctrl())
                    .keep_open(),
                Action::new("edit", "Edit Snippet", push(SnippetForm::factory(Some(s.clone()), "Edit Snippet"))).icon("pencil").shortcut(Shortcut::key(Key::E).ctrl()).keep_open(),
                Action::new("duplicate", "Duplicate Snippet", push(SnippetForm::factory(Some(duplicate), "Create Snippet"))).icon("copy-plus").keep_open(),
                create_action(),
            ],
            vec![
                Action::new("delete", "Delete Snippet", update("Deleted Snippet", move |l| l.into_iter().filter(|x| x.id != delete_id).collect()))
                    .icon("trash-2")
                    .destructive()
                    .keep_open()
                    .shortcut(Shortcut::key(Key::D).ctrl())
                    .confirm("Delete Snippet?", &format!("“{}” will be deleted. This can't be undone.", s.name), "Delete"),
                Action::new("delete-all", "Delete All Snippets", update("Deleted All Snippets", |_| Vec::new()))
                    .icon("trash-2")
                    .destructive()
                    .keep_open()
                    .shortcut(Shortcut::key(Key::D).ctrl().shift())
                    .confirm("Delete All Snippets?", &format!("All {count} snippets will be deleted. This can't be undone."), "Delete All"),
            ],
        ]
    }
}

impl View for SearchSnippets {
    fn placeholder(&self) -> String {
        "Search snippets…".into()
    }

    fn footer(&self) -> (String, Icon) {
        ("Search Snippets".into(), ICON)
    }

    fn empty(&self) -> Empty {
        if self.list.is_empty() {
            Empty { title: "Write faster by storing and inserting frequently used text.".into(), description: Some("Press Ctrl+N to create your first snippet.".into()) }
        } else {
            Empty { title: "No Snippets Found".into(), description: None }
        }
    }

    fn empty_actions(&self) -> Vec<Vec<Action>> {
        vec![vec![create_action()]]
    }

    fn has_detail(&self) -> bool {
        true
    }

    fn detail(&mut self, ui: &mut Ui, rect: Rect, id: &str) {
        let Some(s) = self.list.iter().find(|s| s.id == id) else { return };
        let mut rows = vec![("Name".to_string(), s.name.clone())];
        if let Some(k) = &s.keyword {
            rows.push(("Keyword".into(), k.clone()));
        }
        rows.extend([("Content Type".to_string(), "Text".to_string()), ("Times Copied".into(), s.copy_count.unwrap_or(0).to_string()), ("Last Copied".into(), when(s.copied_at)), ("Modified".into(), when(Some(s.modified_at))), ("Created".into(), when(Some(s.created_at)))]);
        let info = crate::root::metadata(ui, rect, &rows);
        let area = Rect::from_min_max(rect.min, pos2(rect.right(), info.top())).shrink(16.0);
        let galley = crate::root::wrapped(ui, &s.text, egui::FontId::monospace(13.0), crate::theme::FG, area.width());
        ui.painter().with_clip_rect(area).galley(area.min, galley, crate::theme::FG);
    }

    fn sections(&mut self) -> Vec<Section> {
        let version = VERSION.load(Ordering::SeqCst);
        if version != self.version {
            self.version = version;
            self.list = load();
        }
        let item = |s: &Snippet| Item {
            id: s.id.clone(),
            title: s.name.clone(),
            subtitle: None,
            icon: Icon::Tile("file-text", Tone::Slate),
            accessories: s.keyword.iter().map(|k| crate::extension::Accessory::Text(k.clone())).collect(),
            keywords: vec![s.keyword.clone().unwrap_or_default(), s.text.clone()],
            actions: self.actions(s),
        };
        let pinned: Vec<Item> = self.list.iter().filter(|s| s.pinned == Some(true)).map(item).collect();
        let mut rest: Vec<&Snippet> = self.list.iter().filter(|s| s.pinned != Some(true)).collect();
        rest.sort_by_key(|s| std::cmp::Reverse(s.copied_at.unwrap_or(s.modified_at)));
        let rest: Vec<Item> = rest.into_iter().map(item).collect();
        [("Pinned", pinned), ("Snippets", rest)].into_iter().filter(|(_, items)| !items.is_empty()).map(|(title, items)| Section { title: title.into(), items }).collect()
    }
}

fn import(host: &mut Host) -> Result<(), String> {
    host.hide(false);
    let ctx = host.ctx.clone();
    std::thread::spawn(move || {
        let result = (|| -> Result<Option<Toast>, String> {
            let Some(path) = crate::apps::pick_path(false, false, &["json".to_string()])?.into_iter().next() else { return Ok(None) };
            let incoming = parse_import(&std::fs::read_to_string(&path).map_err(|e| e.to_string())?)?;
            let (list, added, skipped) = merge(load(), incoming, crate::store::now_ms(), &uuid);
            save(&list)?;
            let plural = |n: usize| if n == 1 { "snippet" } else { "snippets" };
            let title = format!("{added} {} added{}", plural(added), if skipped > 0 { format!(", {skipped} {} skipped", plural(skipped)) } else { String::new() });
            Ok(Some(if added > 0 { Toast::Success(title) } else { Toast::Failure(title, String::new()) }))
        })();
        match result {
            Ok(Some(toast)) => notify(&ctx, toast),
            Ok(None) => {}
            Err(e) => notify(&ctx, Toast::Failure(e, String::new())),
        }
    });
    Ok(())
}

fn export_all(host: &mut Host) -> Result<(), String> {
    let list = load();
    if list.is_empty() {
        return Err("There are no snippets to export".into());
    }
    host.hide(false);
    let ctx = host.ctx.clone();
    std::thread::spawn(move || {
        let result = (|| -> Result<(), String> {
            let Some(folder) = crate::apps::pick_path(true, false, &[])?.into_iter().next() else { return Ok(()) };
            let path = format!("{}\\snippets.json", folder.trim_end_matches('\\'));
            std::fs::write(&path, export(&list)).map_err(|e| e.to_string())?;
            crate::apps::show_in_explorer(&path)
        })();
        if let Err(e) = result {
            notify(&ctx, Toast::Failure(e, String::new()));
        }
    });
    Ok(())
}

pub struct Snippets;

impl Extension for Snippets {
    fn id(&self) -> &'static str {
        "snippets"
    }

    fn commands(&self) -> Vec<Command> {
        let mut create = command(self.id(), "create", "Create Snippet", ICON, "Command", "Open Command", |_| Ok(()));
        create.view = Some(SnippetForm::factory(None, "Create Snippet"));
        create.keywords = ["new snippet", "text", "expand", "novo snippet", "trecho"].map(String::from).to_vec();
        let mut search = command(self.id(), "search", "Search Snippets", ICON, "Command", "Open Command", |_| Ok(()));
        search.view = Some(Rc::new(|_| Box::new(SearchSnippets { list: Vec::new(), version: u64::MAX, app: frontmost_app() })));
        search.keywords = ["snippet", "text", "expand", "keyword", "snippets", "trechos"].map(String::from).to_vec();
        let mut import_cmd = command(self.id(), "import", "Import Snippets", Icon::Tile("download", Tone::Blue), "Command", "Run Command", import);
        import_cmd.keywords = ["snippet", "json", "importar"].map(String::from).to_vec();
        let mut export_cmd = command(self.id(), "export", "Export Snippets", Icon::Tile("upload", Tone::Blue), "Command", "Run Command", export_all);
        export_cmd.keywords = ["snippet", "json", "backup", "exportar"].map(String::from).to_vec();
        let mut commands = vec![create, search, import_cmd, export_cmd];
        for cmd in &mut commands {
            cmd.subtitle = Some("Snippets".into());
        }
        commands
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snippet(id: &str, name: &str, text: &str) -> Snippet {
        Snippet { id: id.into(), name: name.into(), text: text.into(), ..Default::default() }
    }

    fn with<T>(f: impl FnOnce(&Context) -> T) -> T {
        let snippets = [snippet("sig", "Signature", "Ana{cursor} — {clipboard}")];
        let now = crate::extensions::calculator::history::days_from_civil(2026, 10, 19) as f64 * 86_400_000.0 + 14.0 * 3_600_000.0 + 5.0 * 60_000.0;
        let local = now - icu::local_offset_minutes(now) as f64 * 60_000.0;
        let clipboard = |offset: usize| ["now", "before"].get(offset).map_or(String::new(), |s| s.to_string());
        let id = || "id-1".to_string();
        f(&Context { clipboard: &clipboard, snippets: &snippets, now_ms: local, locale: "en-US", uuid: &id })
    }

    #[test]
    fn resolves_dynamic_placeholders() {
        with(|c| {
            assert_eq!(resolve("{date} | {time} | {day}", c).0, "Oct 19, 2026 | 2:05 PM | Monday");
            assert_eq!(resolve(r#"{clipboard} {clipboard offset="1"} {uuid}"#, c).0, "now before id-1");
            assert_eq!(resolve(r#"{unknown} {argument name="a"}"#, c).0, r#"{unknown} {argument name="a"}"#);
            assert_eq!(resolve("Hi {cursor}!\r\nBye", c), ("Hi !\nBye".into(), 5));
            assert_eq!(resolve("plain", c).1, 0);
            assert_eq!(resolve("Best,\n{snippet name=\"signature\"}", c), ("Best,\nAna — now".into(), 0));
        });
    }

    #[test]
    fn detects_keywords_and_merges_imports() {
        let list = vec![Snippet { keyword: Some(";sig".into()), ..snippet("a", "A", "x") }];
        assert!(keyword_in_use(" ;sig ", &list, None));
        assert!(!keyword_in_use(";sig", &list, Some("a")));
        assert!(!keyword_in_use("", &list, None));
        let parsed = parse_import(r#"[{"name":"A","text":"a","keyword":";a"},{"name":"B","text":"b","keyword":";a"},{"title":"C","text":"c"}]"#).unwrap();
        let (merged, added, skipped) = merge(vec![snippet("c0", "C", "c")], parsed, 5, &|| "new".to_string());
        assert_eq!((added, skipped), (1, 2));
        let exported: serde_json::Value = serde_json::from_str(&export(&merged)).unwrap();
        assert_eq!(exported, serde_json::json!([{ "name": "C", "text": "c" }, { "name": "A", "text": "a", "keyword": ";a" }]));
        assert_eq!(parse_import(r#"[{"name":"x"}]"#).unwrap_err(), "Invalid snippet import format");
    }
}
