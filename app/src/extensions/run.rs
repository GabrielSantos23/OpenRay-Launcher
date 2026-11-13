use super::placeholders::uuid;
use super::{command, GENERATION};
use crate::extension::{Accessory, Action, Argument, Command, Dropdown, Empty, Extension, Field, FieldKind, Form, Host, Icon, Item, Load, Request, Section, Shortcut, Toast, Tone, View, ViewFactory};
use crate::run::{join, split, Entry};
use egui::Key;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, LazyLock};

const ICON: Icon = Icon::Tile("square-terminal", Tone::Slate);
const KINDS: [(&str, &str, &str); 7] = [
    ("controlPanelApplet", "Control Panel Applet", "settings-2"),
    ("environmentVariable", "Environment Variable", "variable"),
    ("executable", "Executable", "terminal"),
    ("managementConsole", "Management Console", "wrench"),
    ("path", "Path", "folder"),
    ("command", "Run Command", "play"),
    ("uri", "URI", "globe"),
];
static VERSION: AtomicU64 = AtomicU64::new(0);
static ROOTED: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^([a-z]:[\\/]|\\\\)").unwrap());
static VARIABLE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^%[^%\s]+%$").unwrap());
static VARIABLE_PATH: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^%[^%]+%[\\/]").unwrap());
static SCHEME: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^[a-z][a-z0-9+.-]+:").unwrap());
static EXECUTABLE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)\.(exe|com|bat|cmd)$").unwrap());

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RunCommand {
    pub id: String,
    pub alias: String,
    pub command: String,
    #[serde(default)]
    pub arguments: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pinned: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_executed_at: Option<u64>,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Source {
    Input,
    History,
    Result,
}

#[derive(Clone, Debug)]
pub struct Row {
    pub id: String,
    pub source: Source,
    pub kind: &'static str,
    pub command: String,
    pub description: String,
    pub record: Option<RunCommand>,
}

fn path() -> std::path::PathBuf {
    crate::data_dir().join("run-commands.json")
}

pub fn load() -> Vec<RunCommand> {
    crate::store::read_json(&path())
}

fn save(list: &[RunCommand]) -> Result<(), String> {
    crate::store::write_json(&path(), &list).map_err(|e| e.to_string())?;
    VERSION.fetch_add(1, Ordering::SeqCst);
    GENERATION.fetch_add(1, Ordering::SeqCst);
    Ok(())
}

fn label(kind: &str) -> &'static str {
    KINDS.iter().find(|k| k.0 == kind).map_or("Executable", |k| k.1)
}

fn glyph(kind: &str) -> &'static str {
    KINDS.iter().find(|k| k.0 == kind).map_or("terminal", |k| k.2)
}

pub fn kind_of(command: &str) -> &'static str {
    let lower = command.to_lowercase();
    let uri = SCHEME.find(command).is_some_and(|m| !command[m.end()..].starts_with(['\\', '/']));
    if VARIABLE.is_match(command) {
        "environmentVariable"
    } else if uri && !ROOTED.is_match(command) {
        "uri"
    } else if lower.ends_with(".cpl") {
        "controlPanelApplet"
    } else if lower.ends_with(".msc") {
        "managementConsole"
    } else if EXECUTABLE.is_match(command) {
        "executable"
    } else if ROOTED.is_match(command) || VARIABLE_PATH.is_match(command) {
        "path"
    } else {
        "executable"
    }
}

pub fn is_path_like(text: &str) -> bool {
    let text = text.trim();
    ROOTED.is_match(text.strip_prefix('"').unwrap_or(text)) || VARIABLE_PATH.is_match(text)
}

pub fn run_rows(input: &str, history: &[String], catalog: &[Entry], commands: &[RunCommand], completions: &[Entry], kind: Option<&str>) -> Vec<(&'static str, Vec<Row>)> {
    let text = input.trim();
    let lower = text.to_lowercase();
    let allowed = |k: &str| kind.is_none_or(|only| only == k);
    let mut history_rows: Vec<Row> = history
        .iter()
        .filter(|h| h.to_lowercase().starts_with(&lower))
        .map(|h| Row { id: format!("history:{h}"), source: Source::History, kind: kind_of(&split(h).0), command: h.clone(), description: String::new(), record: None })
        .filter(|r| allowed(r.kind))
        .collect();
    if !text.is_empty() {
        history_rows.sort_by_key(|r| r.command.chars().count());
    }
    let mut results: Vec<Row> = Vec::new();
    if !text.is_empty() {
        if allowed("command") {
            for record in commands {
                if record.alias.to_lowercase().starts_with(&lower) || record.tags.iter().any(|t| t.to_lowercase().starts_with(&lower)) {
                    let description = if record.description.is_empty() { join(&record.command, &record.arguments) } else { record.description.clone() };
                    results.push(Row { id: format!("command:{}", record.id), source: Source::Result, kind: "command", command: record.alias.clone(), description, record: Some(record.clone()) });
                }
            }
        }
        let pool: Vec<&Entry> = if is_path_like(text) { completions.iter().collect() } else { catalog.iter().filter(|e| e.command.to_lowercase().starts_with(&lower)).collect() };
        for entry in pool {
            if results.len() >= 60 {
                break;
            }
            if allowed(entry.kind) {
                results.push(Row { id: format!("{}:{}", entry.kind, entry.command), source: Source::Result, kind: entry.kind, command: entry.command.clone(), description: entry.description.clone(), record: None });
            }
        }
        results.sort_by_key(|r| (r.kind != "command", r.command.chars().count()));
    }
    let same = |r: &Row| r.command.to_lowercase() == lower;
    let duplicate = history_rows.iter().any(same) || (history_rows.is_empty() && results.iter().any(|r| r.kind != "command" && same(r)));
    let input_kind = kind_of(&split(text).0);
    let input_row: Vec<Row> = if !text.is_empty() && !duplicate && allowed(input_kind) {
        vec![Row { id: "input".into(), source: Source::Input, kind: input_kind, command: text.to_owned(), description: String::new(), record: None }]
    } else {
        Vec::new()
    };
    results.retain(|r| !same(r) || r.kind == "command");
    [("Run", input_row), ("History", history_rows), ("Results", results)].into_iter().filter(|s| !s.1.is_empty()).collect()
}

pub fn run_command_sections(commands: &[RunCommand], now: u64, offset_ms: i64) -> Vec<(&'static str, Vec<RunCommand>)> {
    use super::calculator::history::{civil_from_days, days_from_civil};
    let local_day = |t: u64| (t as i64 + offset_ms).div_euclid(86_400_000);
    let today = local_day(now);
    let (year, month, _) = civil_from_days(today);
    let bounds = [("Today", today), ("Yesterday", today - 1), ("This Week", today - (today + 3).rem_euclid(7)), ("This Month", days_from_civil(year, month, 1)), ("This Year", days_from_civil(year, 1, 1)), ("Older", i64::MIN)];
    let pinned = |c: &RunCommand| c.pinned == Some(true);
    let mut sections = vec![("Pinned", commands.iter().filter(|c| pinned(c)).cloned().collect::<Vec<_>>()), ("Recent", commands.iter().filter(|c| !pinned(c) && c.last_executed_at.is_none()).cloned().collect())];
    sections.extend(bounds.iter().map(|b| (b.0, Vec::new())));
    let mut used: Vec<&RunCommand> = commands.iter().filter(|c| !pinned(c) && c.last_executed_at.is_some()).collect();
    used.sort_by(|a, b| b.last_executed_at.cmp(&a.last_executed_at));
    for c in used {
        let day = local_day(c.last_executed_at.unwrap_or(0));
        let slot = bounds.iter().position(|b| day >= b.1).unwrap_or(bounds.len() - 1);
        sections[slot + 2].1.push(c.clone());
    }
    sections.into_iter().filter(|s| !s.1.is_empty()).collect()
}

fn execute(text: &str, admin: bool, shell: bool) -> Result<(), String> {
    let (command, arguments) = split(text);
    let result = crate::run::execute(&command, &arguments, admin, shell);
    VERSION.fetch_add(1, Ordering::SeqCst);
    result
}

fn execute_record(record: &RunCommand, extra: &str, admin: bool) -> Result<(), String> {
    let arguments = [record.arguments.as_str(), extra].iter().filter(|a| !a.trim().is_empty()).copied().collect::<Vec<_>>().join(" ");
    crate::run::execute(&record.command, &arguments, admin, false)?;
    let now = crate::store::now_ms();
    save(&load().into_iter().map(|c| if c.id == record.id { RunCommand { last_executed_at: Some(now), ..c } } else { c }).collect::<Vec<_>>())
}

fn push(factory: ViewFactory) -> impl Fn(&mut Host) -> Result<(), String> {
    move |host| {
        host.requests.push(Request::Push(factory.clone()));
        Ok(())
    }
}

fn create_action(command: String) -> Action {
    Action::new("create", "Create Run Command", push(RunForm::factory(None, command))).icon("plus").shortcut(Shortcut::key(Key::N).ctrl()).keep_open()
}

struct RunForm {
    initial: Option<RunCommand>,
    form: Form,
}

impl RunForm {
    fn factory(initial: Option<RunCommand>, command: String) -> ViewFactory {
        Rc::new(move |_| {
            let editing = initial.as_ref().filter(|r| load().iter().any(|e| e.id == r.id)).cloned();
            let title = if editing.is_some() { "Edit Run Command" } else { "Create Run Command" };
            let value = |f: fn(&RunCommand) -> String| initial.as_ref().map_or(String::new(), f);
            let fields = vec![
                Field::new("Command", FieldKind::Text, initial.as_ref().map_or(command.clone(), |r| join(&r.command, &r.arguments)))
                    .placeholder("E.g. ping 1.1.1.1 -t")
                    .help("A program, file, folder, URI or %VARIABLE%, followed by its arguments."),
                Field::new("Alias", FieldKind::Text, value(|r| r.alias.clone())).placeholder("Command alias"),
                Field::new("Description", FieldKind::Text, value(|r| r.description.clone())).placeholder("Description (optional)"),
                Field::new("Tags", FieldKind::Text, value(|r| r.tags.join(", "))).placeholder("Tags (optional)").help("Separate tags with commas."),
            ];
            Box::new(RunForm { initial: editing, form: Form { title: title.into(), submit: "Save Run Command", icon: ICON, fields, focus: Some(0) } })
        })
    }
}

impl View for RunForm {
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
        let id = self.initial.as_ref().map(|r| r.id.clone());
        let command = self.form.fields[0].value.trim().to_owned();
        let alias = self.form.fields[1].value.trim().to_owned();
        self.form.fields[0].error = command.is_empty().then(|| "Command is required".into());
        self.form.fields[1].error = if alias.is_empty() {
            Some("Alias is required".into())
        } else {
            list.iter().any(|c| c.alias.eq_ignore_ascii_case(&alias) && Some(&c.id) != id.as_ref()).then(|| "Another run command uses this alias".into())
        };
        if let Some(i) = self.form.fields.iter().position(|f| f.error.is_some()) {
            self.form.focus = Some(i);
            return Ok(false);
        }
        let (program, arguments) = split(&command);
        let tags = self.form.fields[3].value.split(',').map(str::trim).filter(|t| !t.is_empty()).map(String::from).collect();
        let base = self.initial.clone().unwrap_or(RunCommand { id: uuid(), alias: String::new(), command: String::new(), arguments: String::new(), description: String::new(), tags: Vec::new(), pinned: None, last_executed_at: None });
        let record = RunCommand { alias, command: program, arguments, description: self.form.fields[2].value.trim().to_owned(), tags, ..base };
        let next: Vec<RunCommand> = if id.is_some() { list.into_iter().map(|c| if c.id == record.id { record.clone() } else { c }).collect() } else { [list, vec![record]].concat() };
        save(&next)?;
        host.toast = Some(Toast::Success(if id.is_some() { "Updated Run Command" } else { "Created Run Command" }.into()));
        Ok(true)
    }
}

struct OpenRun {
    query: String,
    filter: usize,
    catalog: Load<Arc<Vec<Entry>>>,
    version: u64,
    history: Vec<String>,
    commands: Vec<RunCommand>,
    completions: (String, Vec<Entry>),
}

fn row_icon(row: &Row) -> Icon {
    let file = matches!(row.kind, "path" | "executable" | "controlPanelApplet" | "managementConsole") && !row.description.is_empty();
    if file {
        Icon::File(row.description.clone())
    } else {
        Icon::Glyph(glyph(row.kind))
    }
}

fn row_actions(row: &Row) -> Vec<Vec<Action>> {
    let full = row.record.as_ref().map_or(row.command.clone(), |r| join(&r.command, &r.arguments));
    let mut primary = Vec::new();
    let (record, text) = (row.record.clone(), row.command.clone());
    primary.push(Action::new("run", "Run Command", move |_| record.as_ref().map_or_else(|| execute(&text, false, false), |r| execute_record(r, "", false))).icon("play"));
    if matches!(row.kind, "executable" | "managementConsole" | "command") {
        let (record, text) = (row.record.clone(), full.clone());
        primary.push(Action::new("run-admin", "Run Command as Administrator", move |_| record.as_ref().map_or_else(|| execute(&text, true, false), |r| execute_record(r, "", true))).icon("shield").shortcut(Shortcut::key(Key::Enter).ctrl().shift()));
    }
    if matches!(row.kind, "executable" | "command") {
        let text = full.clone();
        primary.push(Action::new("run-shell", "Run in Shell", move |_| execute(&text, false, true)).icon("terminal").shortcut(Shortcut::key(Key::Enter).ctrl()));
    }
    let copied = full.clone();
    primary.push(
        Action::new("copy", "Copy Command", move |host| {
            host.copy_and_close(&copied, "Copied to Clipboard");
            Ok(())
        })
        .icon("clipboard")
        .shortcut(Shortcut::key(Key::C).ctrl().shift()),
    );
    if row.kind == "environmentVariable" && row.source != Source::Input {
        let value = row.description.clone();
        primary.push(
            Action::new("copy-value", "Copy Value", move |host| {
                host.copy_and_close(&value, "Copied to Clipboard");
                Ok(())
            })
            .icon("clipboard")
            .shortcut(Shortcut::key(Key::C).ctrl().alt()),
        );
    }
    if row.source != Source::Input && (row.kind == "path" || (row.kind == "executable" && !row.description.is_empty())) {
        let target = row.description.trim_end_matches('\\').to_owned();
        primary.push(Action::new("reveal", "Show in File Explorer", move |_| crate::apps::show_in_explorer(&target)).icon("folder"));
    }
    let manage = match &row.record {
        Some(r) => vec![Action::new("edit", "Edit Run Command", push(RunForm::factory(Some(r.clone()), String::new()))).icon("pencil").shortcut(Shortcut::key(Key::E).ctrl()).keep_open()],
        None => vec![create_action(row.command.clone())],
    };
    let mut destructive = Vec::new();
    if row.source == Source::History {
        let text = row.command.clone();
        destructive.push(
            Action::new("forget", "Delete from History", move |host| {
                crate::run::forget(&text)?;
                VERSION.fetch_add(1, Ordering::SeqCst);
                host.toast = Some(Toast::Success("Deleted from history".into()));
                Ok(())
            })
            .icon("trash-2")
            .destructive()
            .keep_open()
            .shortcut(Shortcut::key(Key::D).ctrl()),
        );
    }
    if let Some(r) = &row.record {
        let id = r.id.clone();
        destructive.push(
            Action::new("delete", "Delete Run Command", move |host| {
                save(&load().into_iter().filter(|c| c.id != id).collect::<Vec<_>>())?;
                host.toast = Some(Toast::Success("Deleted Run Command".into()));
                Ok(())
            })
            .icon("trash-2")
            .destructive()
            .keep_open()
            .shortcut(Shortcut::key(Key::D).ctrl().shift()),
        );
    }
    vec![primary, manage, destructive]
}

impl View for OpenRun {
    fn placeholder(&self) -> String {
        "Type a command…".into()
    }

    fn footer(&self) -> (String, Icon) {
        ("Run".into(), ICON)
    }

    fn filters(&self) -> bool {
        false
    }

    fn set_query(&mut self, query: &str) {
        self.query = query.to_owned();
    }

    fn loading(&self) -> bool {
        self.catalog.get().is_none()
    }

    fn empty(&self) -> Empty {
        Empty { title: "No Commands".into(), description: Some("Type a command above and press Enter to run it".into()) }
    }

    fn dropdown(&self) -> Option<Dropdown> {
        let options = std::iter::once(("All Types".to_string(), Icon::Glyph("list"))).chain(KINDS.iter().map(|k| (k.1.to_string(), Icon::Glyph(k.2)))).collect();
        Some(Dropdown { label: "Filter Results", options, selected: self.filter })
    }

    fn select(&mut self, index: usize) {
        self.filter = index;
    }

    fn sections(&mut self) -> Vec<Section> {
        let version = VERSION.load(Ordering::SeqCst);
        if version != self.version {
            self.version = version;
            self.history = crate::run::history();
            self.commands = load();
        }
        if is_path_like(&self.query) && self.completions.0 != self.query {
            self.completions = (self.query.clone(), crate::run::complete(&self.query));
        }
        let catalog = self.catalog.get().and_then(Result::ok).unwrap_or_default();
        let completions = if is_path_like(&self.query) { self.completions.1.as_slice() } else { &[] };
        let kind = self.filter.checked_sub(1).map(|i| KINDS[i].0);
        run_rows(&self.query, &self.history, &catalog, &self.commands, completions, kind)
            .into_iter()
            .map(|(title, rows)| Section {
                title: title.into(),
                items: rows
                    .iter()
                    .map(|row| Item {
                        id: row.id.clone(),
                        title: row.command.clone(),
                        subtitle: (row.source != Source::Input && !row.description.is_empty()).then(|| row.description.clone()),
                        icon: row_icon(row),
                        accessories: vec![Accessory::Text(label(row.kind).into())],
                        keywords: Vec::new(),
                        actions: row_actions(row),
                    })
                    .collect(),
            })
            .collect()
    }
}

struct SearchRunCommands {
    list: Vec<RunCommand>,
    version: u64,
}

fn update(toast: &'static str, change: impl Fn(Vec<RunCommand>) -> Vec<RunCommand> + 'static) -> impl Fn(&mut Host) -> Result<(), String> {
    move |host| {
        save(&change(load()))?;
        host.toast = Some(Toast::Success(toast.into()));
        Ok(())
    }
}

fn record_actions(record: &RunCommand) -> Vec<Vec<Action>> {
    let full = join(&record.command, &record.arguments);
    let executed = record.clone();
    let pinned = record.pinned == Some(true);
    let (pin_id, delete_id) = (record.id.clone(), record.id.clone());
    let duplicate = RunCommand { id: uuid(), alias: format!("{} copy", record.alias), pinned: None, last_executed_at: None, ..record.clone() };
    vec![
        vec![
            Action::new("execute", "Execute Command", move |_| execute_record(&executed, "", false)).icon("play"),
            Action::new("copy", "Copy to Clipboard", move |host| {
                host.copy_and_close(&full, "Copied to Clipboard");
                Ok(())
            })
            .icon("clipboard")
            .shortcut(Shortcut::key(Key::C).ctrl().shift()),
        ],
        vec![
            Action::new(if pinned { "unpin" } else { "pin" }, if pinned { "Unpin" } else { "Pin" }, update(if pinned { "Unpinned" } else { "Pinned" }, move |l| l.into_iter().map(|c| if c.id == pin_id { RunCommand { pinned: Some(!pinned), ..c } } else { c }).collect()))
                .icon(if pinned { "pin-off" } else { "pin" })
                .shortcut(Shortcut::key(Key::P).ctrl().shift())
                .keep_open(),
            Action::new("edit", "Edit", push(RunForm::factory(Some(record.clone()), String::new()))).icon("pencil").shortcut(Shortcut::key(Key::E).ctrl()).keep_open(),
            Action::new("duplicate", "Duplicate", push(RunForm::factory(Some(duplicate), String::new()))).icon("copy-plus").keep_open(),
            create_action(String::new()),
        ],
        vec![
            Action::new("delete", "Delete", update("Deleted", move |l| l.into_iter().filter(|c| c.id != delete_id).collect()))
                .icon("trash-2")
                .destructive()
                .keep_open()
                .shortcut(Shortcut::key(Key::D).ctrl())
                .confirm("Delete Run Command?", "This action cannot be undone.", "Delete"),
            Action::new("delete-all", "Delete All", update("Deleted all", |_| Vec::new()))
                .icon("trash-2")
                .destructive()
                .keep_open()
                .shortcut(Shortcut::key(Key::D).ctrl().shift())
                .confirm("Delete All Run Commands?", "This action cannot be undone.", "Delete All"),
        ],
    ]
}

impl View for SearchRunCommands {
    fn placeholder(&self) -> String {
        "Search run commands…".into()
    }

    fn footer(&self) -> (String, Icon) {
        ("Search Run Commands".into(), ICON)
    }

    fn empty(&self) -> Empty {
        Empty { title: "No Run Commands".into(), description: Some("Press Ctrl+N to create one.".into()) }
    }

    fn empty_actions(&self) -> Vec<Vec<Action>> {
        vec![vec![create_action(String::new())]]
    }

    fn sections(&mut self) -> Vec<Section> {
        let version = VERSION.load(Ordering::SeqCst);
        if version != self.version {
            self.version = version;
            self.list = load();
        }
        let now = crate::store::now_ms();
        let offset = crate::icu::local_offset_minutes(now as f64) as i64 * 60_000;
        run_command_sections(&self.list, now, offset)
            .into_iter()
            .map(|(title, records)| Section {
                title: title.into(),
                items: records
                    .iter()
                    .map(|r| Item {
                        id: r.id.clone(),
                        title: r.alias.clone(),
                        subtitle: Some(if r.description.is_empty() { join(&r.command, &r.arguments) } else { r.description.clone() }),
                        icon: Icon::Glyph("play"),
                        accessories: if r.tags.is_empty() { Vec::new() } else { vec![Accessory::Text(r.tags.join(", "))] },
                        keywords: [vec![r.command.clone(), r.arguments.clone()], r.tags.clone()].concat(),
                        actions: record_actions(r),
                    })
                    .collect(),
            })
            .collect()
    }
}

fn capitalized(text: &str) -> String {
    let mut chars = text.chars();
    chars.next().map_or_else(String::new, |c| c.to_uppercase().chain(chars).collect())
}

pub struct Run;

impl Extension for Run {
    fn id(&self) -> &'static str {
        "windows-run"
    }

    fn commands(&self) -> Vec<Command> {
        let mut open = command(self.id(), "windows-run", "Open Run", ICON, "Command", "Open Command", |_| Ok(()));
        open.view = Some(Rc::new(|ctx| {
            Box::new(OpenRun { query: String::new(), filter: 0, catalog: Load::start(ctx, || Ok(Arc::new(crate::run::catalog()))), version: u64::MAX, history: Vec::new(), commands: Vec::new(), completions: (String::new(), Vec::new()) })
        }));
        open.keywords = ["run", "execute", "win+r", "command", "executar"].map(String::from).to_vec();
        let mut exec = command(self.id(), "execute", "Execute Run Command", Icon::Tile("terminal", Tone::Slate), "Command", "Run Command", |_| Ok(()));
        exec.argument = Some(Argument {
            placeholder: "Command".into(),
            optional: false,
            run: Rc::new(|text, host| {
                execute(text, false, false)?;
                host.hide(false);
                Ok(())
            }),
        });
        exec.keywords = ["run", "execute", "command", "executar"].map(String::from).to_vec();
        let mut search = command(self.id(), "search", "Search Run Commands", Icon::Tile("list", Tone::Slate), "Command", "Open Command", |_| Ok(()));
        search.view = Some(Rc::new(|_| Box::new(SearchRunCommands { list: Vec::new(), version: u64::MAX })));
        search.keywords = ["run", "alias", "commands", "comandos"].map(String::from).to_vec();
        let mut create = command(self.id(), "create", "Create Run Command", Icon::Tile("plus", Tone::Slate), "Command", "Open Command", |_| Ok(()));
        create.view = Some(RunForm::factory(None, String::new()));
        create.keywords = ["run", "alias", "new", "novo"].map(String::from).to_vec();
        let mut commands = vec![open, exec, search, create];
        for record in load() {
            let mut cmd = command(self.id(), &record.id, &capitalized(&record.alias), Icon::Tile("play", Tone::Slate), "Run Command", "Run Command", |_| Ok(()));
            cmd.keywords = [vec!["run".into(), "command".into(), record.command.clone()], (!record.arguments.is_empty()).then(|| record.arguments.clone()).into_iter().collect(), record.tags.clone()].concat();
            let executed = record.clone();
            cmd.argument = Some(Argument {
                placeholder: "Arguments".into(),
                optional: true,
                run: Rc::new(move |text, host| {
                    execute_record(&executed, text, false)?;
                    host.hide(false);
                    Ok(())
                }),
            });
            commands.push(cmd);
        }
        for cmd in &mut commands {
            cmd.subtitle = Some("Run".into());
        }
        commands
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extensions::calculator::history::days_from_civil;

    fn entry(kind: &'static str, command: &str, description: &str) -> Entry {
        Entry { kind, command: command.into(), description: description.into() }
    }

    fn record(id: &str) -> RunCommand {
        RunCommand { id: id.into(), alias: "notes".into(), command: "notepad.exe".into(), arguments: r"C:\notes.txt".into(), description: String::new(), tags: vec!["work".into()], pinned: None, last_executed_at: None }
    }

    fn catalog() -> Vec<Entry> {
        vec![entry("executable", "notepad.exe", r"C:\Windows\System32\notepad.exe"), entry("executable", "node.exe", r"C:\node\node.exe"), entry("managementConsole", "devmgmt.msc", r"C:\Windows\System32\devmgmt.msc"), entry("environmentVariable", "%TEMP%", r"C:\Temp")]
    }

    fn titles(sections: &[(&'static str, Vec<Row>)]) -> Vec<(&'static str, Vec<String>)> {
        sections.iter().map(|(t, rows)| (*t, rows.iter().map(|r| r.command.clone()).collect())).collect()
    }

    #[test]
    fn classifies_commands() {
        let cases = [("%TEMP%", "environmentVariable"), ("ms-settings:display", "uri"), (r"C:\Windows", "path"), ("ncpa.cpl", "controlPanelApplet"), ("devmgmt.msc", "managementConsole"), ("cmd", "executable"), (r"%USERPROFILE%\x", "path")];
        for (text, kind) in cases {
            assert_eq!(kind_of(text), kind, "{text}");
        }
        assert!(is_path_like(r#""C:\Program"#) && !is_path_like("cmd"));
    }

    #[test]
    fn builds_run_history_and_results_sections() {
        let history = ["notepad".to_string(), "cmd".to_string()];
        let records = [record("1")];
        let s = |owned: Vec<(&'static str, Vec<&str>)>| owned.into_iter().map(|(t, r)| (t, r.into_iter().map(String::from).collect())).collect::<Vec<_>>();
        assert_eq!(titles(&run_rows("no", &history, &catalog(), &records, &[], None)), s(vec![("Run", vec!["no"]), ("History", vec!["notepad"]), ("Results", vec!["notes", "node.exe", "notepad.exe"])]));
        assert_eq!(run_rows("", &["cmd".into()], &catalog(), &records, &[], None).iter().map(|s| s.0).collect::<Vec<_>>(), ["History"]);
        assert_eq!(run_rows("notepad", &["notepad".into()], &catalog(), &[], &[], None).iter().map(|s| s.0).collect::<Vec<_>>(), ["History", "Results"]);
        assert!(run_rows("no", &[], &catalog(), &records, &[], Some("managementConsole")).is_empty());
        let completions = [entry("path", r"C:\Windows\", r"C:\Windows")];
        assert_eq!(titles(&run_rows(r"C:\Win", &[], &catalog(), &[], &completions, None)).last().unwrap().1, [r"C:\Windows\"]);
    }

    #[test]
    fn groups_run_commands_by_pin_and_last_use() {
        let at = |y, m, d, h: i64| (days_from_civil(y, m, d) * 86_400_000 + h * 3_600_000) as u64;
        let with = |id: &str, pinned: Option<bool>, used: Option<u64>| RunCommand { pinned, last_executed_at: used, ..record(id) };
        let list = [with("a", Some(true), None), with("b", None, None), with("c", None, Some(at(2026, 10, 21, 9))), with("d", None, Some(at(2026, 10, 20, 9))), with("e", None, Some(at(2025, 2, 1, 0)))];
        let groups = run_command_sections(&list, at(2026, 10, 21, 12), 0);
        let ids: Vec<(&str, Vec<&str>)> = groups.iter().map(|(t, l)| (*t, l.iter().map(|c| c.id.as_str()).collect())).collect();
        assert_eq!(ids, [("Pinned", vec!["a"]), ("Recent", vec!["b"]), ("Today", vec!["c"]), ("Yesterday", vec!["d"]), ("Older", vec!["e"])]);
    }

    #[test]
    fn reads_tauri_run_command_files() {
        let list: Vec<RunCommand> = serde_json::from_str(r#"[{"id":"1","alias":"p","command":"ping","arguments":"1.1.1.1","description":"","tags":[],"lastExecutedAt":1761000000000}]"#).unwrap();
        assert_eq!(list[0].last_executed_at, Some(1_761_000_000_000));
        assert!(serde_json::to_string(&list).unwrap().contains("\"lastExecutedAt\""));
    }
}
