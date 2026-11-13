use crate::extension::{Accessory, Action, Async, Command, Dropdown, Empty, Extension, Field, FieldKind, Form, Host, Icon, Item, Page, Request, Section, Shortcut, Toast, View};
use crate::flowkey_gate::Grants;
use crate::flowkey_host::{self as host, Installed, Shared};
use crate::theme;
use egui::{pos2, vec2, Key, Rect, RichText, Ui, UiBuilder};
use serde_json::{json, Map, Value};
use std::cell::Cell;
use std::collections::HashSet;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

const ACTION_ICONS: &[(&str, &str)] = &[
    (r"^(play|resume)\b", "play"),
    (r"^pause\b", "pause"),
    (r"^dislike\b|^unlike\b|remove from liked", "heart-off"),
    (r"^like\b", "heart"),
    (r"^next\b|^skip\b", "forward"),
    (r"^previous\b", "rewind"),
    (r"radio", "radio"),
    (r"^copy (url|link)", "link"),
    (r"embed", "code"),
    (r"^copy\b", "copy"),
    (r"^paste\b", "clipboard-paste"),
    (r"search (the )?web|in browser|on the web", "globe"),
    (r"^open\b", "external-link"),
    (r"refresh|reload", "refresh-cw"),
    (r"search again", "search"),
    (r"restart|retry|again\b", "rotate-ccw"),
    (r"remove from library", "minus"),
    (r"^(remove|delete|uninstall|clear)\b", "trash-2"),
    (r"add to queue", "list-end"),
    (r"playlist", "list-plus"),
    (r"^(add|create|new|save)\b", "plus"),
    (r"album|grid", "layout-grid"),
    (r"popular", "list-music"),
    (r"^show\b|details|list\b", "list"),
    (r"connect|device", "smartphone"),
    (r"library", "library"),
    (r"search", "search"),
    (r"lyrics", "mic-vocal"),
    (r"settings|preferences|configure", "settings"),
    (r"download", "download"),
    (r"share", "share-2"),
    (r"^(go|back)\b", "arrow-left"),
];

fn intern(text: &str) -> &'static str {
    static STRINGS: Mutex<Option<HashSet<&'static str>>> = Mutex::new(None);
    let mut strings = STRINGS.lock().unwrap_or_else(|e| e.into_inner());
    let set = strings.get_or_insert_with(HashSet::new);
    if let Some(existing) = set.get(text) {
        return existing;
    }
    let leaked: &'static str = Box::leak(text.to_owned().into_boxed_str());
    set.insert(leaked);
    leaked
}

pub fn icon_key(name: &str) -> String {
    let mut out = String::new();
    let chars: Vec<char> = name.trim().chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        let prev = i.checked_sub(1).map(|j| chars[j]);
        if c == '_' || c.is_whitespace() {
            if !out.ends_with('-') {
                out.push('-');
            }
            continue;
        }
        let boundary = prev.is_some_and(|p| (c.is_ascii_uppercase() && (p.is_ascii_lowercase() || p.is_ascii_digit())) || (c.is_ascii_digit() && p.is_ascii_alphabetic()));
        if boundary {
            out.push('-');
        }
        out.extend(c.to_lowercase());
    }
    out
}

fn glyph(name: &str) -> Option<&'static str> {
    let key = icon_key(name);
    crate::icons::has_glyph(&key).then(|| intern(&key))
}

fn action_glyph(title: &str) -> Option<&'static str> {
    let title = title.trim().to_lowercase();
    ACTION_ICONS.iter().find(|(pattern, _)| regex::Regex::new(pattern).is_ok_and(|r| r.is_match(&title))).and_then(|(_, name)| glyph(name))
}

fn is_image(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    [".png", ".jpg", ".jpeg", ".gif", ".webp", ".ico", ".bmp", ".svg"].iter().any(|e| lower.ends_with(e))
}

fn icon_for(ext: &Installed, fields: &Value, title: &str) -> Icon {
    let file = fields["iconUri"].as_str().or(fields["icon"].as_str().filter(|i| is_image(i)));
    if let Some(uri) = file.and_then(|f| ext.image(f)) {
        return Icon::Url(uri);
    }
    let name = fields["iconName"].as_str().or(fields["icon"].as_str());
    if let Some(glyph) = name.and_then(glyph) {
        return Icon::Glyph(glyph);
    }
    match fields["icon"].as_str().filter(|i| !i.is_empty() && !is_image(i)) {
        Some(emoji) if emoji.chars().count() <= 4 => Icon::Emoji(emoji.to_owned()),
        _ => Icon::Initials(title.trim().chars().next().map(|c| c.to_uppercase().collect()).unwrap_or_default()),
    }
}

fn shortcut_of(value: &Value) -> Option<Shortcut> {
    let name = value["key"].as_str()?.to_lowercase();
    let mapped = match name.as_str() {
        "enter" | "return" => "Enter",
        "arrowright" | "rightarrow" => "ArrowRight",
        "arrowleft" | "leftarrow" => "ArrowLeft",
        "arrowup" | "uparrow" => "ArrowUp",
        "arrowdown" | "downarrow" => "ArrowDown",
        "backspace" => "Backspace",
        "delete" => "Delete",
        "escape" => "Escape",
        "tab" => "Tab",
        "space" => "Space",
        "pageup" => "PageUp",
        "pagedown" => "PageDown",
        "home" => "Home",
        "end" => "End",
        other => other,
    };
    let key = Key::from_name(mapped).or_else(|| Key::from_name(&mapped.to_uppercase()))?;
    let modifiers: Vec<String> = value["modifiers"].as_array().into_iter().flatten().filter_map(|m| m.as_str().map(str::to_lowercase)).collect();
    let has = |names: &[&str]| modifiers.iter().any(|m| names.contains(&m.as_str()));
    Some(Shortcut { key: Some(key), ctrl: has(&["cmd", "command", "ctrl", "control", "meta", "win"]), alt: has(&["alt", "opt", "option"]), shift: has(&["shift"]) })
}

fn ordered(actions: &Value) -> Vec<Value> {
    let mut list: Vec<Value> = actions.as_array().cloned().unwrap_or_default();
    if let Some(i) = list.iter().position(|a| a["primary"] == true) {
        let primary = list.remove(i);
        list.insert(0, primary);
    }
    list
}

#[derive(Clone)]
struct Target {
    ext: Arc<Installed>,
    view: String,
    state: Shared,
}

impl Target {
    fn run(&self, action: &Value, item: Option<Value>, form: Option<Value>) {
        let Some(id) = action["id"].as_str() else { return };
        let title = action["title"].as_str().unwrap_or("Action").trim_end_matches('…').to_owned();
        let push = action["push"].as_str().map(String::from);
        let (state, ext) = (self.state.clone(), self.ext.clone());
        state.lock().unwrap_or_else(|e| e.into_inner()).busy += 1;
        host::action(&self.ext, &self.view, id, item, form, move |result| {
            state.lock().unwrap_or_else(|e| e.into_inner()).busy -= 1;
            let Some(ctx) = crate::flowkey_host::context() else { return };
            match result {
                Err(error) => crate::extension::post(&ctx, Async::Toast(Toast::Failure(format!("{title} failed"), error))),
                Ok(()) => {
                    if let Some(command) = push {
                        crate::extension::post(&ctx, Async::Push(Box::new(move |ctx| Box::new(TreeView::new(ext, command, ctx, None)))));
                    }
                }
            }
        });
    }

    fn actions(&self, actions: &Value, item: Option<&Value>) -> Vec<Vec<Action>> {
        let mapped: Vec<Action> = ordered(actions)
            .into_iter()
            .enumerate()
            .map(|(i, action)| {
                let title = action["title"].as_str().unwrap_or_default().to_owned();
                let (target, item, spec) = (self.clone(), item.cloned(), action.clone());
                let mut built = Action::new("flowkey-action", title.clone(), move |_| {
                    target.run(&spec, item.clone(), None);
                    Ok(())
                })
                .keep_open();
                built.icon = action_glyph(&title);
                built.destructive = action["style"] == "destructive";
                built.shortcut = match i {
                    0 => Some(Shortcut::key(Key::Enter)),
                    _ => shortcut_of(&action["shortcut"]).or((i == 1).then(|| Shortcut::key(Key::Enter).ctrl())),
                };
                built
            })
            .collect();
        let mut first = mapped;
        let rest = first.split_off(first.len().min(1));
        [first, rest].into_iter().filter(|g| !g.is_empty()).collect()
    }
}

enum Stage {
    Consent(Grants, Rc<Cell<bool>>),
    Setup(Form, Vec<Value>),
    Arguments(Form, Vec<Value>),
    Running(Target),
}

pub struct TreeView {
    ext: Arc<Installed>,
    command: String,
    title: String,
    icon: Icon,
    stage: Stage,
    query: String,
    filter: Option<String>,
    arguments: Option<Value>,
    form: Option<(String, Form)>,
}

fn preference_form(ext: &Installed) -> (Form, Vec<Value>) {
    let schema: Vec<Value> = ext.manifest["preferences"].as_array().cloned().unwrap_or_default();
    let values = host::preferences(ext);
    let fields = schema.iter().map(|p| field_for(p, values.get(p["name"].as_str().unwrap_or_default()), "title")).collect();
    (Form { title: format!("Set Up {}", ext.name()), submit: "Save Preferences", icon: Icon::Glyph("settings"), fields, focus: Some(0) }, schema)
}

fn argument_form(ext: &Installed, command: &Value) -> (Form, Vec<Value>) {
    let schema: Vec<Value> = command["arguments"].as_array().cloned().unwrap_or_default();
    let fields = schema.iter().map(|a| field_for(a, None, "placeholder")).collect();
    let title = command["title"].as_str().unwrap_or_default();
    let submit = if command["mode"] == "background" { "Run Command" } else { "Open Command" };
    (Form { title: format!("{title} — {}", ext.name()), submit, icon: Icon::Glyph("command"), fields, focus: Some(0) }, schema)
}

fn options_of(value: &Value) -> Vec<(String, String, Icon)> {
    value.as_array().into_iter().flatten().map(|o| (o["value"].as_str().unwrap_or_default().to_owned(), o["title"].as_str().unwrap_or_default().to_owned(), Icon::Glyph("check"))).collect()
}

fn field_for(spec: &Value, value: Option<&Value>, label_key: &str) -> Field {
    let label = intern(spec[label_key].as_str().or(spec["name"].as_str()).unwrap_or_default());
    let kind = spec["type"].as_str().unwrap_or("text");
    let current = value.or(spec.get("default")).filter(|v| !v.is_null());
    let (field_kind, initial) = match kind {
        "checkbox" => (FieldKind::Toggle, (current == Some(&Value::Bool(true))).to_string()),
        "dropdown" => {
            let options = options_of(spec.get("options").or(spec.get("data")).unwrap_or(&Value::Null));
            let first = options.first().map(|o| o.0.clone()).unwrap_or_default();
            (FieldKind::Choice(options), current.and_then(Value::as_str).map_or(first, String::from))
        }
        _ => (FieldKind::Text, current.and_then(Value::as_str).unwrap_or_default().to_owned()),
    };
    let mut field = Field::new(label, field_kind, initial).placeholder(spec["placeholder"].as_str().unwrap_or_default());
    field.secret = kind == "password";
    if let Some(help) = spec["description"].as_str().or(spec["label"].as_str()) {
        field = field.help(help);
    }
    field
}

fn values_of(form: &Form, schema: &[Value]) -> Map<String, Value> {
    schema
        .iter()
        .zip(&form.fields)
        .filter_map(|(spec, field)| {
            let name = spec["name"].as_str()?.to_owned();
            let value = if matches!(field.kind, FieldKind::Toggle) { Value::Bool(field.value == "true") } else { Value::String(field.value.clone()) };
            Some((name, value))
        })
        .collect()
}

impl TreeView {
    pub fn new(ext: Arc<Installed>, command: String, ctx: &egui::Context, initial: Option<String>) -> Self {
        host::init(ctx);
        let spec = ext.command(&command).unwrap_or(Value::Null);
        let title = spec["title"].as_str().unwrap_or(&command).to_owned();
        let icon = icon_for(&ext, &json!({ "icon": spec["icon"].as_str().or(ext.manifest["icon"].as_str()) }), &title);
        let mut view = Self { ext, command, title, icon, stage: Stage::Consent(Grants::default(), Rc::default()), query: initial.unwrap_or_default(), filter: None, arguments: None, form: None };
        view.advance();
        view
    }

    fn advance(&mut self) {
        if let Some(missing) = host::pending_consent(&self.ext) {
            self.stage = Stage::Consent(missing, Rc::default());
            return;
        }
        if !host::missing_preferences(&self.ext).is_empty() {
            let (form, schema) = preference_form(&self.ext);
            self.stage = Stage::Setup(form, schema);
            return;
        }
        let spec = self.ext.command(&self.command).unwrap_or(Value::Null);
        if self.arguments.is_none() && spec["arguments"].as_array().is_some_and(|a| !a.is_empty()) {
            let (form, schema) = argument_form(&self.ext, &spec);
            self.stage = Stage::Arguments(form, schema);
            return;
        }
        let (view, state) = host::open_view(&self.ext, &self.command, &self.query, self.arguments.clone());
        self.stage = Stage::Running(Target { ext: self.ext.clone(), view, state });
    }

    fn target(&self) -> Option<&Target> {
        match &self.stage {
            Stage::Running(target) => Some(target),
            _ => None,
        }
    }

    fn tree(&self) -> Option<Value> {
        self.target().and_then(|t| t.state.lock().unwrap_or_else(|e| e.into_inner()).tree.clone())
    }

    fn error(&self) -> Option<String> {
        self.target().and_then(|t| t.state.lock().unwrap_or_else(|e| e.into_inner()).error.clone())
    }

    fn rendered(&self) {
        if let Some(target) = self.target() {
            host::render(&self.ext, &target.view, &self.command, &self.query, self.filter.as_deref(), self.arguments.clone());
        }
    }

    fn raw_sections(tree: &Value) -> Vec<Value> {
        if tree["type"] == "grid" && tree["sections"].as_array().is_none_or(|s| s.is_empty()) {
            return vec![json!({ "title": "", "items": tree["items"] })];
        }
        tree["sections"].as_array().cloned().unwrap_or_default()
    }

    fn find_item(&self, id: &str) -> Option<Value> {
        let tree = self.tree()?;
        let (section, item) = id.split_once(':')?;
        let section: usize = section.parse().ok()?;
        Self::raw_sections(&tree).get(section)?["items"].as_array()?.iter().find(|i| i["id"].as_str() == Some(item)).cloned()
    }
}

impl Drop for TreeView {
    fn drop(&mut self) {
        if let Some(target) = self.target() {
            host::close(&self.ext, &target.view);
        }
    }
}

fn markdown(ui: &mut Ui, source: &str) {
    use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};
    let mut job = egui::text::LayoutJob::default();
    let (mut size, mut strong, mut code, mut list) = (13.0, false, false, 0usize);
    let flush = |ui: &mut Ui, job: &mut egui::text::LayoutJob| {
        if !job.text.is_empty() {
            ui.add(egui::Label::new(std::mem::take(job)).wrap().selectable(true));
            ui.add_space(8.0);
        }
    };
    for event in Parser::new(source) {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                flush(ui, &mut job);
                size = match level {
                    HeadingLevel::H1 => 22.0,
                    HeadingLevel::H2 => 18.0,
                    _ => 15.0,
                };
                strong = true;
            }
            Event::End(TagEnd::Heading(_)) => {
                flush(ui, &mut job);
                size = 13.0;
                strong = false;
            }
            Event::Start(Tag::Strong) => strong = true,
            Event::End(TagEnd::Strong) => strong = false,
            Event::Start(Tag::List(_)) => list += 1,
            Event::End(TagEnd::List(_)) => list = list.saturating_sub(1),
            Event::Start(Tag::Item) => job.append(&format!("{}• ", "   ".repeat(list.saturating_sub(1))), 0.0, egui::TextFormat { font_id: theme::font(13.0), color: theme::FG_60, ..Default::default() }),
            Event::End(TagEnd::CodeBlock) => {
                code = false;
                flush(ui, &mut job);
            }
            Event::End(TagEnd::Item | TagEnd::Paragraph | TagEnd::BlockQuote(_)) => flush(ui, &mut job),
            Event::Start(Tag::CodeBlock(_)) => code = true,
            Event::Code(text) => job.append(&text, 0.0, egui::TextFormat { font_id: theme::mono(12.0), color: theme::FG, background: theme::KEY_BG, ..Default::default() }),
            Event::Text(text) => {
                let font = if code { theme::mono(12.0) } else if strong { theme::weighted(size, theme::SEMIBOLD) } else { theme::font(size) };
                job.append(&text, 0.0, egui::TextFormat { font_id: font, color: theme::FG, ..Default::default() });
            }
            Event::SoftBreak => job.append(" ", 0.0, egui::TextFormat { font_id: theme::font(size), color: theme::FG, ..Default::default() }),
            Event::HardBreak => job.append("\n", 0.0, egui::TextFormat { font_id: theme::font(size), color: theme::FG, ..Default::default() }),
            Event::Rule => {
                flush(ui, &mut job);
                ui.separator();
            }
            _ => {}
        }
    }
    flush(ui, &mut job);
}

fn field_rows(fields: &Value) -> Vec<(String, String)> {
    fields
        .as_array()
        .into_iter()
        .flatten()
        .filter(|f| f["kind"] != "separator")
        .map(|f| {
            let value = match f["tags"].as_array() {
                Some(tags) => tags.iter().filter_map(Value::as_str).collect::<Vec<_>>().join(", "),
                None => f["value"].as_str().unwrap_or_default().to_owned(),
            };
            (f["label"].as_str().unwrap_or_default().to_owned(), value)
        })
        .collect()
}

fn image(ui: &mut Ui, ext: &Installed, uri: Option<&str>, max_height: f32) {
    if let Some(uri) = uri.and_then(|u| ext.image(u)) {
        ui.add(egui::Image::new(uri).max_height(max_height).max_width(ui.available_width()).corner_radius(6.0));
        ui.add_space(12.0);
    }
}

fn centered_message(ui: &Ui, rect: Rect, title: &str, description: &str) {
    let center = rect.center();
    let title_rect = ui.painter().text(center - vec2(0.0, 10.0), egui::Align2::CENTER_CENTER, title, theme::weighted(13.0, theme::MEDIUM), theme::FG);
    let galley = crate::root::wrapped(ui, description, theme::font(13.0), theme::FG_60, 460.0);
    ui.painter().galley(pos2(center.x - galley.size().x / 2.0, title_rect.bottom() + 4.0), galley, theme::FG_60);
}

impl View for TreeView {
    fn placeholder(&self) -> String {
        if let Some(text) = self.tree().and_then(|t| t["searchBarPlaceholder"].as_str().map(String::from)) {
            return text;
        }
        let grid_title = self.tree().filter(|t| t["type"] == "grid").and_then(|t| t["title"].as_str().map(String::from));
        let title = grid_title.unwrap_or_else(|| self.title.clone());
        if title.to_lowercase().starts_with("search") {
            format!("{title}...")
        } else {
            format!("Search {title}...")
        }
    }

    fn footer(&self) -> (String, Icon) {
        (self.title.clone(), self.icon.clone())
    }

    fn filters(&self) -> bool {
        false
    }

    fn set_query(&mut self, query: &str) {
        if query != self.query {
            self.query = query.to_owned();
            self.rendered();
        }
    }

    fn loading(&self) -> bool {
        let Some(target) = self.target() else { return false };
        let state = target.state.lock().unwrap_or_else(|e| e.into_inner());
        state.busy > 0 || (state.tree.is_none() && state.error.is_none()) || state.tree.as_ref().is_some_and(|t| t["isLoading"] == true)
    }

    fn dropdown(&self) -> Option<Dropdown> {
        let tree = self.tree()?;
        let options = tree["filter"]["options"].as_array().filter(|o| !o.is_empty())?;
        let selected = self.filter.as_ref().and_then(|f| options.iter().position(|o| o["value"].as_str() == Some(f))).unwrap_or(0);
        Some(Dropdown { label: "Filter", options: options.iter().map(|o| (o["label"].as_str().unwrap_or_default().to_owned(), Icon::Glyph("list"))).collect(), selected })
    }

    fn select(&mut self, index: usize) {
        let value = self.tree().and_then(|t| t["filter"]["options"][index]["value"].as_str().map(String::from));
        if value.is_some() && value != self.filter {
            self.filter = value;
            self.rendered();
        }
    }

    fn grid(&self) -> Option<usize> {
        self.tree().filter(|t| t["type"] == "grid").map(|t| t["columns"].as_u64().unwrap_or(5).clamp(1, 8) as usize)
    }

    fn has_detail(&self) -> bool {
        self.tree().is_some_and(|t| t["type"] == "list" && (t["layout"] == "side-pane" || Self::raw_sections(&t).iter().any(|s| s["items"].as_array().is_some_and(|i| i.iter().any(|i| i["pane"].is_object())))))
    }

    fn detail(&mut self, ui: &mut Ui, rect: Rect, id: &str) {
        let Some(pane) = self.find_item(id).map(|i| i["pane"].clone()).filter(Value::is_object) else { return };
        let rows: Vec<(String, String)> = field_rows(&pane["fields"]);
        let bottom = if rows.is_empty() { rect.bottom() } else { crate::root::metadata(ui, rect, &rows).top() };
        let area = Rect::from_min_max(rect.min, pos2(rect.right(), bottom)).shrink2(vec2(16.0, 12.0));
        let mut child = ui.new_child(UiBuilder::new().max_rect(area));
        egui::ScrollArea::vertical().id_salt(("flowkey-pane", id)).auto_shrink(false).show(&mut child, |ui| {
            image(ui, &self.ext, pane["previewImageUri"].as_str(), 220.0);
            if let Some(preview) = pane["preview"].as_str() {
                markdown(ui, preview);
            }
        });
    }

    fn page(&mut self) -> Option<Page> {
        if let Stage::Consent(_, allowed) = &self.stage {
            if allowed.replace(false) {
                if let Err(error) = host::grant_consent(&self.ext) {
                    eprintln!("openray: could not save consent for {}: {error}", self.ext.id());
                }
                self.advance();
                return self.page();
            }
            let allow = allowed.clone();
            let actions = vec![vec![Action::new("allow", "Allow", move |_| {
                allow.set(true);
                Ok(())
            })
            .icon("check")
            .keep_open()]];
            return Some(Page { title: format!("Allow “{}”?", self.ext.name()), dropdowns: Vec::new(), actions, enter_runs_primary: true, backspace_pops: true });
        }
        let target = self.target()?.clone();
        let tree = self.tree();
        match tree {
            Some(tree) if tree["type"] == "detail" => {
                let actions = target.actions(&tree["actions"], None);
                Some(Page { title: tree["title"].as_str().unwrap_or_default().to_owned(), dropdowns: Vec::new(), actions, enter_runs_primary: true, backspace_pops: true })
            }
            None if self.error().is_some() => Some(Page { title: self.title.clone(), dropdowns: Vec::new(), actions: Vec::new(), enter_runs_primary: false, backspace_pops: true }),
            _ => None,
        }
    }

    fn page_ui(&mut self, ui: &mut Ui, rect: Rect) {
        if let Stage::Consent(grants, _) = &self.stage {
            let mut child = ui.new_child(UiBuilder::new().max_rect(rect.shrink2(vec2(24.0, 16.0))));
            child.label(RichText::new("This extension was installed outside Openray and needs your permission to use:").font(theme::font(13.0)).color(theme::FG_60));
            child.add_space(8.0);
            for line in grants.describe() {
                child.label(RichText::new(format!("•  {line}")).font(theme::font(13.0)).color(theme::FG));
            }
            child.add_space(12.0);
            child.label(RichText::new("Press Enter to allow, or Esc to cancel.").font(theme::font(12.0)).color(theme::FG_40));
            return;
        }
        let Some(tree) = self.tree() else {
            centered_message(ui, rect, &format!("{} failed to load", self.title), &self.error().unwrap_or_default());
            return;
        };
        let rows = field_rows(&tree["fields"]);
        let (body, side) = if rows.is_empty() { (rect, None) } else { (Rect::from_min_max(rect.min, pos2(rect.right() - 260.0, rect.bottom())), Some(Rect::from_min_max(pos2(rect.right() - 260.0, rect.top()), rect.max))) };
        let mut child = ui.new_child(UiBuilder::new().max_rect(body.shrink2(vec2(24.0, 16.0))));
        egui::ScrollArea::vertical().id_salt("flowkey-detail").auto_shrink(false).show(&mut child, |ui| {
            if let Some(subtitle) = tree["subtitle"].as_str() {
                ui.label(RichText::new(subtitle).font(theme::font(13.0)).color(theme::FG_60));
                ui.add_space(8.0);
            }
            image(ui, &self.ext, tree["imageUri"].as_str(), 200.0);
            if let Some(description) = tree["description"].as_str() {
                markdown(ui, description);
            }
        });
        if let Some(side) = side {
            ui.painter().vline(side.left(), side.y_range(), egui::Stroke::new(1.0, theme::SEPARATOR));
            let mut column = ui.new_child(UiBuilder::new().max_rect(side.shrink2(vec2(16.0, 12.0))));
            egui::ScrollArea::vertical().id_salt("flowkey-fields").auto_shrink(false).show(&mut column, |ui| {
                for (label, value) in rows {
                    ui.label(RichText::new(label).font(theme::font(12.0)).color(theme::FG_60));
                    ui.add(egui::Label::new(RichText::new(value).font(theme::weighted(13.0, theme::MEDIUM)).color(theme::FG)).wrap());
                    ui.add_space(8.0);
                }
            });
        }
    }

    fn form(&mut self) -> Option<&mut Form> {
        if !matches!(self.stage, Stage::Running(_)) {
            return match &mut self.stage {
                Stage::Setup(form, _) | Stage::Arguments(form, _) => Some(form),
                _ => None,
            };
        }
        let tree = self.tree().filter(|t| t["type"] == "form");
        let Some(tree) = tree else {
            self.form = None;
            return None;
        };
        let key = tree["fields"].to_string();
        if self.form.as_ref().is_none_or(|(k, _)| *k != key) {
            let previous: Vec<(String, String)> = self.form.take().map(|(_, f)| f.fields.into_iter().map(|f| (f.label.to_owned(), f.value)).collect()).unwrap_or_default();
            let fields = tree["fields"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|f| !matches!(f["kind"].as_str(), Some("separator" | "description")))
                .map(|f| {
                    let kind = match f["kind"].as_str() {
                        Some("checkbox") => "checkbox",
                        Some("dropdown") => "dropdown",
                        Some("password") => "password",
                        _ => "text",
                    };
                    let spec = json!({ "name": f["id"], "title": f["label"].as_str().or(f["id"].as_str()), "type": kind, "default": f["default"], "options": f["options"], "placeholder": f["placeholder"] });
                    let mut field = field_for(&spec, None, "title");
                    if f["kind"] == "textarea" {
                        field.kind = FieldKind::Area(80.0);
                    }
                    if let Some((_, value)) = previous.iter().find(|(label, _)| label == field.label) {
                        field.value = value.clone();
                    }
                    field
                })
                .collect();
            let submit = tree["actions"].as_array().and_then(|a| a.iter().find(|a| a["primary"] == true).or(a.first())).and_then(|a| a["title"].as_str()).unwrap_or("Submit");
            self.form = Some((key, Form { title: tree["title"].as_str().unwrap_or(&self.title).to_owned(), submit: intern(submit), icon: self.icon.clone(), fields, focus: Some(0) }));
        }
        self.form.as_mut().map(|(_, form)| form)
    }

    fn submit(&mut self, host: &mut Host) -> Result<bool, String> {
        match &mut self.stage {
            Stage::Setup(form, schema) => {
                let mut values = host::preferences(&self.ext);
                values.extend(values_of(form, schema));
                host::set_preferences(&self.ext, values)?;
                let missing = host::missing_preferences(&self.ext);
                if !missing.is_empty() {
                    return Err(format!("Still required: {}", missing.join(", ")));
                }
                self.advance();
                Ok(false)
            }
            Stage::Arguments(form, schema) => {
                let values = values_of(form, schema);
                if let Some((spec, _)) = schema.iter().zip(&form.fields).find(|(spec, field)| spec["required"] == true && field.value.trim().is_empty()) {
                    return Err(format!("{} is required", spec["placeholder"].as_str().or(spec["name"].as_str()).unwrap_or("A value")));
                }
                let values: Map<String, Value> = values.into_iter().filter(|(_, v)| v != "").collect();
                let spec = self.ext.command(&self.command).unwrap_or(Value::Null);
                if spec["mode"] == "background" {
                    run_background(&self.ext, &self.command, Some(Value::Object(values)), host.ctx.clone());
                    host.hide(true);
                    return Ok(true);
                }
                self.arguments = Some(Value::Object(values));
                self.advance();
                Ok(false)
            }
            Stage::Consent(..) => Ok(false),
            Stage::Running(target) => {
                let target = target.clone();
                let Some(tree) = self.tree() else { return Ok(false) };
                let (Some((_, form)), Some(fields)) = (&self.form, tree["fields"].as_array()) else { return Ok(false) };
                let editable: Vec<&Value> = fields.iter().filter(|f| !matches!(f["kind"].as_str(), Some("separator" | "description"))).collect();
                let mut values = Map::new();
                for (spec, field) in editable.iter().zip(&form.fields) {
                    let Some(id) = spec["id"].as_str() else { continue };
                    if spec["required"] == true && field.value.trim().is_empty() {
                        return Err(format!("{} is required", field.label));
                    }
                    let value = match spec["kind"].as_str() {
                        Some("checkbox") => Value::Bool(field.value == "true"),
                        Some("tagpicker" | "filepicker") => json!(field.value.split(',').map(str::trim).filter(|s| !s.is_empty()).collect::<Vec<_>>()),
                        _ => Value::String(field.value.clone()),
                    };
                    values.insert(id.to_owned(), value);
                }
                let declared = ordered(&tree["actions"]);
                let action = match tree["submitActionId"].as_str() {
                    Some(id) => declared.iter().find(|a| a["id"] == id).cloned().unwrap_or_else(|| json!({ "id": id, "title": "Submit" })),
                    None => match declared.first() {
                        Some(action) => action.clone(),
                        None => return Ok(false),
                    },
                };
                target.run(&action, None, Some(Value::Object(values)));
                Ok(false)
            }
        }
    }

    fn empty(&self) -> Empty {
        if let Some(error) = self.error() {
            return Empty { title: "Something went wrong".into(), description: Some(error) };
        }
        let tree = self.tree().unwrap_or(Value::Null);
        Empty { title: tree["emptyView"]["title"].as_str().unwrap_or("No Results").to_owned(), description: tree["emptyView"]["description"].as_str().map(String::from) }
    }

    fn sections(&mut self) -> Vec<Section> {
        let (Some(target), Some(tree)) = (self.target().cloned(), self.tree()) else { return Vec::new() };
        if !matches!(tree["type"].as_str(), Some("list" | "grid")) {
            return Vec::new();
        }
        let grid = tree["type"] == "grid";
        let mut sections: Vec<Section> = Self::raw_sections(&tree)
            .iter()
            .enumerate()
            .map(|(s, section)| Section {
                title: section["title"].as_str().unwrap_or_default().to_owned(),
                items: section["items"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|item| {
                        let title = item["title"].as_str().unwrap_or_default().to_owned();
                        let accessories = if grid { Vec::new() } else { item["accessories"].as_array().into_iter().flatten().filter_map(|a| a["text"].as_str().map(|t| Accessory::Text(t.to_owned()))).collect() };
                        Item {
                            id: format!("{s}:{}", item["id"].as_str().unwrap_or_default()),
                            icon: icon_for(&self.ext, item, &title),
                            subtitle: item["subtitle"].as_str().map(String::from),
                            keywords: item["keywords"].as_array().into_iter().flatten().filter_map(|k| k.as_str().map(String::from)).collect(),
                            actions: target.actions(&item["actions"], Some(item)),
                            accessories,
                            title,
                        }
                    })
                    .collect(),
            })
            .collect();
        let more = &tree["pagination"];
        if tree["type"] == "list" && more["hasNextPage"] == true {
            if let Some(id) = more["moreActionId"].as_str() {
                let load = target.actions(&json!([{ "id": id, "title": "Load More" }]), None);
                sections.push(Section { title: String::new(), items: vec![Item { id: "more:".into(), title: "Load More".into(), subtitle: None, icon: Icon::Glyph("plus"), accessories: Vec::new(), keywords: Vec::new(), actions: load }] });
            }
        }
        sections
    }
}

fn run_background(ext: &Arc<Installed>, command: &str, arguments: Option<Value>, ctx: egui::Context) {
    host::init(&ctx);
    let title = ext.command(command).and_then(|c| c["title"].as_str().map(String::from)).unwrap_or_else(|| command.to_owned());
    host::background(ext, command, arguments, move |result| {
        if let Err(error) = result {
            crate::extension::post(&ctx, Async::Toast(Toast::Failure(format!("{title} failed"), error)));
        }
    });
}

pub struct FlowKey {
    id: &'static str,
    ext: Arc<Installed>,
}

pub fn extensions() -> Vec<Box<dyn Extension>> {
    let (extensions, failures) = host::list();
    for failure in failures {
        eprintln!("openray: FlowKey extension skipped: {failure}");
    }
    extensions.into_iter().map(|ext| Box::new(FlowKey { id: intern(&format!("flowkey.{}", ext.id())), ext }) as Box<dyn Extension>).collect()
}

impl Extension for FlowKey {
    fn id(&self) -> &'static str {
        self.id
    }

    fn commands(&self) -> Vec<Command> {
        self.ext
            .commands()
            .into_iter()
            .filter(|spec| spec["ui"] != "web")
            .map(|spec| {
                let id = spec["id"].as_str().unwrap_or_default().to_owned();
                let title = spec["title"].as_str().unwrap_or(&id).to_owned();
                let icon = icon_for(&self.ext, &json!({ "icon": spec["icon"].as_str().or(self.ext.manifest["icon"].as_str()) }), &title);
                let has_arguments = spec["arguments"].as_array().is_some_and(|a| !a.is_empty());
                let background = spec["mode"] == "background" && !has_arguments;
                let ext = self.ext.clone();
                let command_id = id.clone();
                let mut command = super::command(self.id, &id, &title, icon, "Command", if background { "Run Command" } else { "Open Command" }, move |host| {
                    if !background {
                        return Ok(());
                    }
                    if !host::missing_preferences(&ext).is_empty() {
                        return Err(format!("{} needs setup. Open one of its view commands to set its preferences.", ext.name()));
                    }
                    match host::pending_consent(&ext) {
                        Some(missing) => {
                            let (ext, command_id, ctx) = (ext.clone(), command_id.clone(), host.ctx.clone());
                            let message = format!("This extension was installed outside Openray and needs your permission to use:\n{}", missing.describe().iter().map(|l| format!("•  {l}")).collect::<Vec<_>>().join("\n"));
                            let confirmation = crate::extension::Confirmation { title: format!("Allow “{}”?", ext.name()), message, confirm_title: "Allow".into() };
                            crate::extension::post(
                                &host.ctx.clone(),
                                Async::Confirm(
                                    confirmation,
                                    Box::new(move |yes| {
                                        if yes && host::grant_consent(&ext).is_ok() {
                                            run_background(&ext, &command_id, None, ctx);
                                        }
                                    }),
                                ),
                            );
                            host.requests.push(Request::SetQuery(String::new()));
                        }
                        None => {
                            run_background(&ext, &command_id, None, host.ctx.clone());
                            host.hide(true);
                        }
                    }
                    Ok(())
                });
                command.subtitle = Some(spec["subtitle"].as_str().unwrap_or(self.ext.name()).to_owned());
                command.keywords = spec["keywords"].as_array().into_iter().flatten().filter_map(|k| k.as_str().map(String::from)).collect();
                if !background {
                    let (ext, id) = (self.ext.clone(), id.clone());
                    command.view = Some(Rc::new(move |ctx| Box::new(TreeView::new(ext.clone(), id.clone(), ctx, None)) as Box<dyn View>));
                }
                command
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_icon_names_to_lucide_keys() {
        assert_eq!(icon_key("ListMusic"), "list-music");
        assert_eq!(icon_key("Volume2"), "volume-2");
        assert_eq!(icon_key("disc_3"), "disc-3");
        assert_eq!(icon_key("mic vocal"), "mic-vocal");
    }

    #[test]
    fn maps_shortcuts_and_orders_primary_first() {
        let s = shortcut_of(&json!({ "key": "k", "modifiers": ["cmd", "shift"] })).unwrap();
        assert_eq!((s.key, s.ctrl, s.shift, s.alt), (Some(Key::K), true, true, false));
        assert_eq!(shortcut_of(&json!({ "key": "return" })).unwrap().key, Some(Key::Enter));
        let actions = ordered(&json!([{ "id": "a" }, { "id": "b", "primary": true }, { "id": "c" }]));
        assert_eq!(actions.iter().map(|a| a["id"].as_str().unwrap()).collect::<Vec<_>>(), ["b", "a", "c"]);
    }

    #[test]
    fn picks_action_icons_from_titles() {
        assert_eq!(action_glyph("Play Song"), Some("play"));
        assert_eq!(action_glyph("Copy URL"), Some("link"));
        assert_eq!(action_glyph("Copy Song URL"), Some("copy"));
        assert_eq!(action_glyph("Add to Queue"), Some("list-end"));
        assert_eq!(action_glyph("Frobnicate"), None);
    }
}
