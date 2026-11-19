use super::{command, GENERATION};
use crate::extension::{notify, notify_toast, Accessory, Action, Argument, Command, Empty, Extension, Field, FieldKind, Form, Host, Icon, Item, Load, Page, Request, Section, Shortcut, Toast, Tone, View, ViewFactory};
use crate::scripts::{Output, Script};
use crate::theme;
use egui::Key;
use serde::{Deserialize, Serialize};
use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

const ICON: Icon = Icon::Tile("scroll-text", Tone::Slate);
const TEMPLATES: [(&str, &str, &str, &str); 4] = [("powershell", "PowerShell", "ps1", "#"), ("batch", "Batch", "bat", "REM"), ("python", "Python", "py", "#"), ("javascript", "JavaScript (Node.js)", "js", "//")];
const MODES: [(&str, &str); 4] = [("compact", "Compact"), ("silent", "Silent"), ("fullOutput", "Full Output"), ("inline", "Inline")];
static VERSION: AtomicU64 = AtomicU64::new(0);

#[derive(Serialize, Deserialize, Default)]
struct Data {
    #[serde(default)]
    directories: Vec<String>,
}

fn path() -> std::path::PathBuf {
    crate::data_dir().join("script-commands.json")
}

fn directories() -> Vec<String> {
    crate::store::read_json::<Data>(&path()).directories
}

fn add_directory(dir: &str) -> Result<(), String> {
    let mut dirs = directories();
    if !dirs.iter().any(|d| d.eq_ignore_ascii_case(dir)) {
        dirs.push(dir.to_owned());
        crate::store::write_json(&path(), &Data { directories: dirs }).map_err(|e| e.to_string())?;
    }
    changed();
    Ok(())
}

fn changed() {
    VERSION.fetch_add(1, Ordering::SeqCst);
    GENERATION.fetch_add(1, Ordering::SeqCst);
}

fn load() -> Vec<Script> {
    crate::scripts::list(&directories())
}

pub struct TemplateOptions<'a> {
    pub template: &'a str,
    pub title: &'a str,
    pub mode: &'a str,
    pub description: &'a str,
    pub package_name: &'a str,
    pub needs_confirmation: bool,
    pub with_argument: bool,
}

fn template_of(value: &str) -> (&'static str, &'static str, &'static str, &'static str) {
    TEMPLATES.iter().copied().find(|t| t.0 == value).unwrap_or(TEMPLATES[0])
}

pub fn script_file_name(title: &str, template: &str) -> String {
    let cleaned: String = title.trim().to_lowercase().chars().filter(|c| !r#"<>:"/\|?*"#.contains(*c)).collect();
    let base = cleaned.split_whitespace().collect::<Vec<_>>().join("-");
    format!("{}.{}", if base.is_empty() { "my-script-command" } else { &base }, template_of(template).2)
}

pub fn script_template(o: &TemplateOptions) -> String {
    let c = template_of(o.template).3;
    let title = if o.title.trim().is_empty() { "My Script Command" } else { o.title.trim() };
    let hello = if o.with_argument { "Hello World! Argument1 value:" } else { "Hello World!" };
    let (header, body) = match o.template {
        "batch" => ("@echo off", if o.with_argument { format!("echo {hello} %1") } else { format!("echo {hello}") }),
        "python" => ("", if o.with_argument { format!("import sys\nprint(\"{hello} \" + sys.argv[1])") } else { format!("print(\"{hello}\")") }),
        "javascript" => ("", if o.with_argument { format!("console.log(\"{hello} \" + process.argv[2])") } else { format!("console.log(\"{hello}\")") }),
        _ => ("", if o.with_argument { format!("param([string]$arg1)\nWrite-Output \"{hello} $arg1\"") } else { format!("Write-Output \"{hello}\"") }),
    };
    let mut lines: Vec<String> = Vec::new();
    if !header.is_empty() {
        lines.extend([header.to_owned(), String::new()]);
    }
    lines.extend([format!("{c} Required parameters:"), format!("{c} @raycast.schemaVersion 1"), format!("{c} @raycast.title {title}"), format!("{c} @raycast.mode {}", o.mode), String::new(), format!("{c} Optional parameters:"), format!("{c} @raycast.icon 🤖")]);
    if !o.package_name.trim().is_empty() {
        lines.push(format!("{c} @raycast.packageName {}", o.package_name.trim()));
    }
    if !o.description.trim().is_empty() {
        lines.push(format!("{c} @raycast.description {}", o.description.trim()));
    }
    if o.needs_confirmation && o.mode != "inline" {
        lines.push(format!("{c} @raycast.needsConfirmation true"));
    }
    if o.with_argument && o.mode != "inline" {
        lines.push(format!("{c} @raycast.argument1 {{ \"type\": \"text\", \"placeholder\": \"Placeholder\" }}"));
    }
    lines.extend([String::new(), body, String::new()]);
    lines.join(if o.template == "batch" { "\r\n" } else { "\n" })
}

pub fn last_line(text: &str) -> String {
    text.trim_end().lines().next_back().unwrap_or("").trim().to_owned()
}

pub fn is_emoji_icon(icon: &str) -> bool {
    !icon.is_empty() && !icon.contains(['\\', '/', '.']) && icon.chars().count() <= 3
}

fn icon_of(script: &Script, fallback: Icon) -> Icon {
    if is_emoji_icon(&script.icon) {
        Icon::Emoji(script.icon.clone())
    } else {
        fallback
    }
}

fn argument_of(script: &Script) -> Option<(String, bool)> {
    let arg = script.arguments.first()?;
    Some((arg.get("placeholder").and_then(|p| p.as_str()).unwrap_or("Argument").to_owned(), arg.get("optional").and_then(|o| o.as_bool()).unwrap_or(false)))
}

fn failure(script: &Script, output: &Output) -> String {
    [last_line(&output.stderr), last_line(&output.stdout)].into_iter().find(|l| !l.is_empty()).unwrap_or_else(|| format!("{} exited with code {}", script.title, output.code.map_or("?".into(), |c| c.to_string())))
}

fn run_quietly(script: &Script, args: Vec<String>, host: &mut Host, in_view: bool) {
    let (script, ctx, silent) = (script.clone(), host.ctx.clone(), script.mode == "silent" || !in_view);
    if silent {
        host.hide(true);
    }
    std::thread::spawn(move || {
        let toast = match crate::scripts::run(&script.path, &args) {
            Ok(output) if output.code == Some(0) => {
                let line = last_line(&output.stdout);
                Toast::Success(if line.is_empty() { format!("Ran {}", script.title) } else { line })
            }
            Ok(output) => Toast::Failure(format!("{} failed", script.title), failure(&script, &output)),
            Err(e) => Toast::Failure(format!("{} failed", script.title), e),
        };
        if silent {
            notify(&ctx, toast);
        } else {
            notify_toast(&ctx, toast);
        }
    });
}

fn confirm_if_needed(action: Action, script: &Script) -> Action {
    if script.needs_confirmation {
        action.confirm(&format!("Run {}?", script.title), "This script asks for confirmation before it runs.", "Run Script")
    } else {
        action
    }
}

struct OutputView {
    script: Script,
    args: Vec<String>,
    ctx: egui::Context,
    load: Load<Output>,
    again: Rc<Cell<bool>>,
}

impl OutputView {
    fn factory(script: Script, args: Vec<String>) -> ViewFactory {
        Rc::new(move |ctx| {
            let (path, owned) = (script.path.clone(), args.clone());
            Box::new(OutputView { script: script.clone(), args: args.clone(), ctx: ctx.clone(), load: Load::start(ctx, move || crate::scripts::run(&path, &owned)), again: Rc::default() })
        })
    }

    fn text(&self) -> Option<(String, Option<i32>)> {
        Some(match self.load.get()? {
            Ok(output) => ([output.stdout.trim_end(), output.stderr.trim_end()].iter().filter(|s| !s.is_empty()).copied().collect::<Vec<_>>().join("\n"), output.code),
            Err(e) => (e, None),
        })
    }
}

impl View for OutputView {
    fn placeholder(&self) -> String {
        String::new()
    }

    fn footer(&self) -> (String, Icon) {
        (if self.script.package_name.is_empty() { "Script Commands".into() } else { self.script.package_name.clone() }, ICON)
    }

    fn sections(&mut self) -> Vec<Section> {
        Vec::new()
    }

    fn page(&mut self) -> Option<Page> {
        if self.again.replace(false) {
            let (path, args) = (self.script.path.clone(), self.args.clone());
            self.load = Load::start(&self.ctx, move || crate::scripts::run(&path, &args));
        }
        let text = self.text().map(|t| t.0).unwrap_or_default();
        let again = self.again.clone();
        let actions = vec![vec![
            Action::new("copy", "Copy Output", move |host| {
                host.copy_and_close(&text, "Copied Output");
                Ok(())
            })
            .icon("clipboard")
            .shortcut(Shortcut::key(Key::C).ctrl().shift()),
            Action::new("again", "Run Again", move |_| {
                again.set(true);
                Ok(())
            })
            .icon("refresh-cw")
            .shortcut(Shortcut::key(Key::R).ctrl())
            .keep_open(),
        ]];
        Some(Page { title: self.script.title.clone(), dropdowns: Vec::new(), actions, enter_runs_primary: false, backspace_pops: true })
    }

    fn page_ui(&mut self, ui: &mut egui::Ui, _rect: egui::Rect) {
        let Some((text, code)) = self.text() else { return };
        egui::ScrollArea::both().auto_shrink(false).show(ui, |ui| {
            egui::Frame::NONE.inner_margin(egui::Margin::same(16)).show(ui, |ui| {
                let font = egui::FontId::monospace(13.0);
                if let Some(code) = code.filter(|c| *c != 0) {
                    ui.label(egui::RichText::new(format!("Exited with code {code}")).font(font.clone()).color(theme::RED()));
                }
                let shown = if text.is_empty() { "No output" } else { &text };
                ui.add(egui::Label::new(egui::RichText::new(shown).font(font).color(theme::FG())).wrap().selectable(true));
            });
        });
    }
}

fn pick_directory(ctx: egui::Context) {
    std::thread::spawn(move || match crate::apps::pick_path(true, false, &[]) {
        Ok(list) => {
            if let Some(dir) = list.into_iter().next() {
                match add_directory(&dir) {
                    Ok(()) => notify_toast(&ctx, Toast::Success("Added Script Directory".into())),
                    Err(e) => notify_toast(&ctx, Toast::Failure("Could not add the folder".into(), e)),
                }
            }
        }
        Err(e) => notify_toast(&ctx, Toast::Failure("Could not open the folder picker".into(), e)),
    });
}

fn add_folder_action() -> Action {
    Action::new("add-directory", "Add Script Directory", |host: &mut Host| {
        pick_directory(host.ctx.clone());
        Ok(())
    })
    .icon("folder-plus")
    .keep_open()
}

fn create_action() -> Action {
    Action::new("create", "Create Script Command", |host: &mut Host| {
        host.requests.push(Request::Push(CreateScript::factory()));
        Ok(())
    })
    .icon("plus")
    .shortcut(Shortcut::key(Key::N).ctrl())
    .keep_open()
}

fn mode_label(mode: &str) -> String {
    MODES.iter().find(|m| m.0 == mode).map_or_else(|| mode.to_owned(), |m| m.1.to_owned())
}

struct SearchScripts {
    scripts: Vec<Script>,
    version: u64,
    values: HashMap<String, String>,
    select: Option<String>,
}

impl SearchScripts {
    fn actions(&self, script: &Script) -> Vec<Vec<Action>> {
        let value = self.values.get(&script.path).map(|v| v.trim().to_owned()).unwrap_or_default();
        let args: Vec<String> = if value.is_empty() { Vec::new() } else { vec![value] };
        let missing = argument_of(script).filter(|(_, optional)| !optional && args.is_empty()).map(|(p, _)| p);
        let ran = script.clone();
        let run = Action::new("run", "Run Script", move |host| {
            if let Some(placeholder) = &missing {
                return Err(format!("Enter a value for {placeholder}"));
            }
            if ran.mode == "fullOutput" {
                host.requests.push(Request::Push(OutputView::factory(ran.clone(), args.clone())));
            } else {
                run_quietly(&ran, args.clone(), host, true);
            }
            Ok(())
        })
        .icon("play")
        .keep_open();
        let (reveal, open, copy) = (script.path.clone(), script.path.clone(), script.path.clone());
        vec![
            vec![confirm_if_needed(run, script)],
            vec![
                Action::new("reveal", "Show in File Explorer", move |_| crate::apps::show_in_explorer(&reveal)).icon("folder").shortcut(Shortcut::key(Key::Enter).ctrl()),
                Action::new("open-with", "Open With…", move |_| crate::apps::open_with(&open)).icon("square-pen").shortcut(Shortcut::key(Key::O).ctrl()),
                Action::new("copy-contents", "Copy Script Contents", move |host| {
                    let text = std::fs::read_to_string(&copy).map_err(|e| e.to_string())?;
                    host.copy_and_close(&text, "Copied Script Contents");
                    Ok(())
                })
                .icon("clipboard")
                .shortcut(Shortcut::key(Key::C).ctrl().alt()),
            ],
            vec![create_action(), add_folder_action()],
        ]
    }
}

impl View for SearchScripts {
    fn placeholder(&self) -> String {
        "Search script commands…".into()
    }

    fn footer(&self) -> (String, Icon) {
        ("Search Script Commands".into(), ICON)
    }

    fn empty(&self) -> Empty {
        Empty { title: "No Script Commands".into(), description: Some("Press Ctrl+N to create one, or add a folder with scripts from the actions.".into()) }
    }

    fn empty_actions(&self) -> Vec<Vec<Action>> {
        vec![vec![create_action(), add_folder_action()]]
    }

    fn initial_selection(&self) -> Option<String> {
        self.select.clone()
    }

    fn item_argument(&self, id: &str) -> Option<(String, String)> {
        let script = self.scripts.iter().find(|s| s.path == id)?;
        Some((argument_of(script)?.0, self.values.get(id).cloned().unwrap_or_default()))
    }

    fn set_argument(&mut self, id: &str, value: &str) {
        self.values.insert(id.to_owned(), value.to_owned());
    }

    fn sections(&mut self) -> Vec<Section> {
        let version = VERSION.load(Ordering::SeqCst);
        if version != self.version {
            self.version = version;
            self.scripts = load();
        }
        let items: Vec<Item> = self
            .scripts
            .iter()
            .map(|s| Item {
                id: s.path.clone(),
                title: s.title.clone(),
                subtitle: Some(s.package_name.clone()).filter(|p| !p.is_empty()),
                icon: icon_of(s, Icon::Glyph("file-code-2")),
                accessories: vec![Accessory::Text(mode_label(&s.mode))],
                keywords: vec!["script".into(), s.package_name.clone(), s.description.clone(), s.path.rsplit(['\\', '/']).next().unwrap_or("").to_owned()],
                actions: self.actions(s),
            })
            .collect();
        if items.is_empty() {
            Vec::new()
        } else {
            vec![Section { title: "Script Commands".into(), items }]
        }
    }
}

fn search_view(select: Option<String>) -> ViewFactory {
    Rc::new(move |_| Box::new(SearchScripts { scripts: Vec::new(), version: u64::MAX, values: HashMap::new(), select: select.clone() }))
}

struct CreateScript {
    form: Form,
}

impl CreateScript {
    fn factory() -> ViewFactory {
        Rc::new(|_| {
            let templates = TEMPLATES.iter().map(|t| (t.0.to_owned(), t.1.to_owned(), Icon::Glyph("file-code-2"))).collect();
            let modes = MODES.iter().map(|m| (m.0.to_owned(), m.1.to_owned(), Icon::Glyph("play"))).collect();
            let fields = vec![
                Field::new("Template", FieldKind::Choice(templates), "powershell"),
                Field::new("Mode", FieldKind::Choice(modes), "silent"),
                Field::new("Title", FieldKind::Text, "").placeholder("Command Title"),
                Field::new("Description", FieldKind::Text, "").placeholder("Descriptive summary"),
                Field::new("Package Name", FieldKind::Text, "").placeholder("E.g., Developer Utils"),
                Field::new("Needs Confirmation", FieldKind::Toggle, "false"),
                Field::new("With Argument", FieldKind::Toggle, "false"),
            ];
            Box::new(CreateScript { form: Form { title: "Create Script Command".into(), submit: "Create Script", icon: ICON, fields, focus: Some(2) } })
        })
    }

    fn value(&self, label: &str) -> String {
        self.form.fields.iter().find(|f| f.label == label).map(|f| f.value.clone()).unwrap_or_default()
    }
}

impl View for CreateScript {
    fn placeholder(&self) -> String {
        String::new()
    }

    fn footer(&self) -> (String, Icon) {
        ("Create Script Command".into(), ICON)
    }

    fn sections(&mut self) -> Vec<Section> {
        Vec::new()
    }

    fn form(&mut self) -> Option<&mut Form> {
        Some(&mut self.form)
    }

    fn form_changed(&mut self, index: usize) {
        if self.form.fields.get(index).map(|f| f.label) != Some("Mode") {
            return;
        }
        let inline = self.value("Mode") == "inline";
        let has = self.form.fields.iter().any(|f| f.label == "Needs Confirmation");
        if inline && has {
            self.form.fields.retain(|f| !matches!(f.kind, FieldKind::Toggle));
        } else if !inline && !has {
            self.form.fields.push(Field::new("Needs Confirmation", FieldKind::Toggle, "false"));
            self.form.fields.push(Field::new("With Argument", FieldKind::Toggle, "false"));
        }
    }

    fn submit(&mut self, host: &mut Host) -> Result<bool, String> {
        let dir = match directories().into_iter().next() {
            Some(dir) => dir,
            None => match crate::apps::pick_path(true, false, &[])?.into_iter().next() {
                Some(dir) => dir,
                None => return Ok(false),
            },
        };
        let (template, mode, title) = (self.value("Template"), self.value("Mode"), self.value("Title"));
        let path = format!("{}\\{}", dir.trim_end_matches(['\\', '/']), script_file_name(&title, &template));
        if std::path::Path::new(&path).exists() {
            return Err(format!("{path} already exists"));
        }
        let inline = mode == "inline";
        let content = script_template(&TemplateOptions {
            template: &template,
            title: &title,
            mode: &mode,
            description: &self.value("Description"),
            package_name: &self.value("Package Name"),
            needs_confirmation: self.value("Needs Confirmation") == "true" && !inline,
            with_argument: self.value("With Argument") == "true" && !inline,
        });
        std::fs::write(&path, content).map_err(|e| format!("Failed to create script: {e}"))?;
        add_directory(&dir)?;
        crate::apps::show_in_explorer(&path)?;
        host.hud(Ok("Script command created".into()));
        Ok(true)
    }
}

pub struct ScriptCommands;

impl Extension for ScriptCommands {
    fn id(&self) -> &'static str {
        "script-commands"
    }

    fn commands(&self) -> Vec<Command> {
        let mut search = command(self.id(), "search", "Search Script Commands", ICON, "Command", "Open Command", |_| Ok(()));
        search.view = Some(search_view(None));
        search.keywords = ["find", "browse", "scripts", "comandos"].map(String::from).to_vec();
        let mut create = command(self.id(), "create", "Create Script Command", Icon::Tile("plus", Tone::Slate), "Command", "Open Command", |_| Ok(()));
        create.view = Some(CreateScript::factory());
        create.keywords = ["new", "create", "script", "template", "novo", "criar"].map(String::from).to_vec();
        let reload = command(self.id(), "reload", "Reload Script Commands", Icon::Tile("refresh-cw", Tone::Slate), "Command", "Reload", |host| {
            let count = load().len();
            changed();
            host.hud(Ok(format!("Reloaded {count} script command{}", if count == 1 { "" } else { "s" })));
            Ok(())
        });
        let mut commands = vec![search, create, reload];
        for cmd in &mut commands {
            cmd.subtitle = Some("Script Commands".into());
        }
        for script in load() {
            let mut cmd = command(self.id(), &script.path, &script.title, icon_of(&script, ICON), "Script Command", "Run Script", |_| Ok(()));
            cmd.subtitle = Some(if script.package_name.is_empty() { "Script Commands".into() } else { script.package_name.clone() });
            cmd.keywords = vec!["script".into(), script.package_name.clone()];
            let argument = argument_of(&script);
            if script.mode == "fullOutput" {
                cmd.view = Some(if argument.is_some() { search_view(Some(script.path.clone())) } else { OutputView::factory(script.clone(), Vec::new()) });
            } else if let Some((placeholder, optional)) = argument {
                let ran = script.clone();
                cmd.argument = Some(Argument {
                    placeholder,
                    optional,
                    run: Rc::new(move |value, host| {
                        let args = if value.trim().is_empty() { Vec::new() } else { vec![value.trim().to_owned()] };
                        run_quietly(&ran, args, host, false);
                        Ok(())
                    }),
                });
            } else {
                let ran = script.clone();
                cmd.run = Rc::new(move |host| {
                    run_quietly(&ran, Vec::new(), host, false);
                    Ok(())
                });
            }
            if script.needs_confirmation {
                cmd.confirm = Some(crate::extension::Confirmation { title: format!("Run {}?", script.title), message: "This script asks for confirmation before it runs.".into(), confirm_title: "Run Script".into() });
            }
            commands.push(cmd);
        }
        commands
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options<'a>(template: &'a str, title: &'a str, mode: &'a str, package: &'a str, confirm: bool, argument: bool) -> TemplateOptions<'a> {
        TemplateOptions { template, title, mode, description: if title == "Say Hi" { "Greets" } else { "" }, package_name: package, needs_confirmation: confirm, with_argument: argument }
    }

    #[test]
    fn writes_the_metadata_header_for_a_template() {
        let text = script_template(&options("powershell", "Say Hi", "compact", "Dev", true, true));
        assert!(text.contains("# @raycast.schemaVersion 1\n# @raycast.title Say Hi\n# @raycast.mode compact"));
        assert!(text.contains("# @raycast.packageName Dev"));
        assert!(text.contains("# @raycast.needsConfirmation true"));
        assert!(text.contains("# @raycast.argument1 { \"type\": \"text\", \"placeholder\": \"Placeholder\" }"));
        assert!(text.contains("param([string]$arg1)\nWrite-Output \"Hello World! Argument1 value: $arg1\""));
        assert!(crate::scripts::parse("x.ps1", &text).is_some_and(|s| s.title == "Say Hi" && s.arguments.len() == 1));
    }

    #[test]
    fn uses_rem_comments_and_crlf_for_batch_and_drops_inline_extras() {
        let text = script_template(&options("batch", "", "silent", "", false, false));
        assert!(text.starts_with("@echo off\r\n\r\nREM Required parameters:"));
        assert!(text.contains("REM @raycast.title My Script Command") && !text.contains("packageName"));
        let inline = script_template(&options("python", "T", "inline", "", true, true));
        assert!(!inline.contains("needsConfirmation") && !inline.contains("argument1"));
    }

    #[test]
    fn derives_file_names_and_reads_output() {
        assert_eq!(script_file_name("Open My: Repo", "javascript"), "open-my-repo.js");
        assert_eq!(script_file_name("  ", "batch"), "my-script-command.bat");
        assert_eq!(last_line("one\r\ntwo\r\n\r\n"), "two");
        assert!(is_emoji_icon("🤖") && !is_emoji_icon("icon.png"));
    }
}
