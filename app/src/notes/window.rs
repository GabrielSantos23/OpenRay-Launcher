use super::doc::{Kind, BOLD, CODE, ITALIC, STRIKE, UNDERLINE};
use super::editor::{Format, NoteEditor};
use super::Note;
use crate::extension::{notify, Shortcut, Toast};
use crate::root::{consume, key_presses, keycaps, paint_text, plain};
use crate::search::match_score;
use crate::{icons, launcher, theme};
use egui::{pos2, vec2, Align2, Color32, Key, Rect, Sense, Stroke, Ui, UiBuilder};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const SAVE_DELAY: f64 = 0.3;
const HEADER: f32 = 40.0;
const FOOTER: f32 = 32.0;
const TOP_PANEL_Y: f32 = 118.0;
const TOP_PANEL_WIDTH: f32 = 320.0;
const FORMAT_PANEL_WIDTH: f32 = 260.0;
const FORMAT_PANEL_BOTTOM: f32 = 40.0;
const SEARCH: f32 = 38.0;
const MENU_LIST_MAX: f32 = 239.0;
const BROWSE_LIST_MAX: f32 = 320.0;
pub const TITLE: &str = "Notes";

#[derive(Clone, Copy, PartialEq, Debug)]
enum CopyKind {
    Markdown,
    Text,
    Html,
}

#[derive(Clone, PartialEq, Debug)]
enum Cmd {
    New,
    Duplicate,
    TogglePin(String),
    Browse,
    Back,
    Forward,
    CopyMenu,
    Copy(CopyKind),
    FormatMenu,
    HeadingMenu,
    ListMenu,
    ToggleFormatBar,
    Delete(String),
    DeleteAll,
    Format(Format),
}

#[derive(Clone)]
struct Item {
    title: String,
    icon: &'static str,
    shortcut: Option<Shortcut>,
    destructive: bool,
    disabled: bool,
    cmd: Cmd,
}

fn item(title: &str, icon: &'static str, cmd: Cmd) -> Item {
    Item { title: title.into(), icon, shortcut: None, destructive: false, disabled: false, cmd }
}

impl Item {
    fn key(mut self, shortcut: Shortcut) -> Self {
        self.shortcut = Some(shortcut);
        self
    }

    fn danger(mut self) -> Self {
        self.destructive = true;
        self
    }

    fn off(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Placement {
    Top,
    Format,
}

enum Overlay {
    Menu { title: String, groups: Vec<Vec<Item>>, placement: Placement, query: String, selected: usize },
    Browse { query: String, selected: usize },
    Link { url: String },
    Confirm { title: String, message: String, confirm: String, cmd: Cmd, ok: bool },
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct Prefs {
    format_bar: bool,
    words: bool,
}

impl Default for Prefs {
    fn default() -> Self {
        Self { format_bar: true, words: false }
    }
}

const ACTIONS: Shortcut = Shortcut::key(Key::K).ctrl();
const BROWSE: Shortcut = Shortcut::key(Key::P).ctrl();
const CREATE: Shortcut = Shortcut::key(Key::N).ctrl();
const PIN: Shortcut = Shortcut::key(Key::P).ctrl().shift();
const BACK: Shortcut = Shortcut::key(Key::OpenBracket).ctrl();
const FORWARD: Shortcut = Shortcut::key(Key::CloseBracket).ctrl();
const COPY: Shortcut = Shortcut::key(Key::C).ctrl().shift();
const FORMAT_BAR: Shortcut = Shortcut::key(Key::Comma).ctrl().alt();
const REMOVE: Shortcut = Shortcut::key(Key::D).ctrl();
const REMOVE_ALL: Shortcut = Shortcut::key(Key::D).ctrl().alt();

pub fn note_title(text: &str) -> String {
    text.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("Untitled").to_owned()
}

pub fn count_label(text: &str, words: bool) -> String {
    let n = if words { text.split_whitespace().count() } else { text.trim().chars().count() };
    let digits = n.to_string();
    let grouped: String = digits.chars().enumerate().flat_map(|(i, c)| (i > 0 && (digits.len() - i) % 3 == 0).then_some(',').into_iter().chain([c])).collect();
    let unit = if words { "word" } else { "character" };
    format!("{grouped} {unit}{}", if n == 1 { "" } else { "s" })
}

fn with_protocol(url: &str) -> String {
    let scheme = url.split_once(':').is_some_and(|(s, _)| !s.is_empty() && s.chars().next().is_some_and(|c| c.is_ascii_alphabetic()) && s.chars().all(|c| c.is_ascii_alphanumeric() || "+.-".contains(c)));
    if scheme {
        url.to_owned()
    } else {
        format!("https://{url}")
    }
}

fn markdown_html(markdown: &str) -> String {
    let mut html = String::new();
    pulldown_cmark::html::push_html(&mut html, pulldown_cmark::Parser::new_ext(markdown, pulldown_cmark::Options::ENABLE_STRIKETHROUGH | pulldown_cmark::Options::ENABLE_TASKLISTS));
    html
}

fn now_ms() -> f64 {
    crate::store::now_ms() as f64
}

fn tip(label: &str, shortcut: Option<Shortcut>) -> String {
    let label = crate::i18n::tr(label).into_owned();
    match shortcut {
        Some(s) => format!("{label}  {}", s.labels().join(" ")),
        None => label,
    }
}

fn icon_button(ui: &Ui, rect: Rect, id: &str, glyph: &str, label: &str, shortcut: Option<Shortcut>) -> bool {
    let response = ui.interact(rect, egui::Id::unique(("notes-button", id)), Sense::click());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, label));
    if response.hovered() {
        ui.painter().rect_filled(rect, 5.0, theme::CONTROL());
    }
    icons::paint_glyph(ui, glyph, Rect::from_center_size(rect.center(), vec2(16.0, 16.0)), theme::FG());
    response.on_hover_text(tip(label, shortcut)).clicked()
}

fn format_button(ui: &Ui, rect: Rect, id: &str, glyph: &str, menu: bool, active: bool, label: &str, shortcut: Option<Shortcut>) -> bool {
    let response = ui.interact(rect, egui::Id::unique(("notes-format", id)), Sense::click());
    response.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, true, active, label));
    if response.hovered() {
        ui.painter().rect_filled(rect, 5.0, theme::CONTROL());
    }
    let color = if active || response.hovered() { theme::FG() } else { theme::FG_60() };
    if menu {
        icons::paint_glyph(ui, glyph, Rect::from_min_size(pos2(rect.left() + 4.0, rect.center().y - 8.0), vec2(16.0, 16.0)), color);
        icons::paint_glyph(ui, "chevron-down", Rect::from_min_size(pos2(rect.left() + 22.0, rect.center().y - 6.0), vec2(12.0, 12.0)), color);
    } else {
        icons::paint_glyph(ui, glyph, Rect::from_center_size(rect.center(), vec2(16.0, 16.0)), color);
    }
    response.on_hover_text(tip(label, shortcut)).clicked()
}

fn panel_frame(ui: &Ui, frame: Rect) {
    let painter = ui.painter();
    painter.add(egui::epaint::RectShape::filled(frame.translate(vec2(0.0, 8.0)).expand(4.0), 10.0, Color32::from_black_alpha(40)));
    painter.rect(frame, 6.0, theme::PANEL(), Stroke::new(1.0, theme::PANEL_BORDER()), egui::StrokeKind::Inside);
}

fn search_field(ui: &mut Ui, frame: Rect, id: &str, text: &mut String, hint: &str) -> bool {
    let search = Rect::from_min_max(frame.min + vec2(1.0, 1.0), pos2(frame.right() - 1.0, frame.top() + 1.0 + SEARCH));
    ui.painter().hline(search.x_range(), search.bottom() - 0.5, Stroke::new(1.0, theme::PANEL_LINE()));
    let field = search.shrink2(vec2(15.0, 0.0));
    let id = egui::Id::unique(("notes-search", id));
    let before = text.clone();
    let edit = egui::TextEdit::singleline(text).id(id).frame(egui::Frame::NONE).margin(egui::Margin::ZERO).font(theme::font(13.0)).text_color(theme::FG()).hint_text(egui::RichText::new(crate::i18n::tr(hint)).color(theme::FG_40())).desired_width(field.width());
    ui.put(Rect::from_center_size(field.center(), vec2(field.width(), 18.0)), edit);
    ui.memory_mut(|m| m.request_focus(id));
    *text != before
}

pub struct NotesWindow {
    data_dir: PathBuf,
    notes: Vec<Note>,
    current: String,
    editor: NoteEditor,
    dirty_since: Option<f64>,
    edited: bool,
    focus: bool,
    styled: bool,
    scrolled: bool,
    history: Vec<String>,
    history_index: usize,
    overlay: Option<Overlay>,
    prefs: Prefs,
    drag: Option<launcher::DragOrigin>,
}

impl NotesWindow {
    pub fn open(data_dir: PathBuf) -> Self {
        let mut notes = super::load(&data_dir);
        if notes.is_empty() {
            notes.push(super::new_note(now_ms()));
        }
        let index = super::current(&notes).unwrap_or(0);
        notes[index].last_accessed_at = now_ms();
        let current = notes[index].id.clone();
        let editor = NoteEditor::new(egui::Id::unique("note-editor"), &notes[index].markdown);
        let prefs = crate::store::read_json(&data_dir.join("notes-ui.json"));
        Self { data_dir, notes, current: current.clone(), editor, dirty_since: Some(f64::NEG_INFINITY), edited: false, focus: true, styled: false, scrolled: false, history: vec![current], history_index: 0, overlay: None, prefs, drag: None }
    }

    pub fn request_focus(&mut self) {
        self.focus = true;
    }

    fn index(&self) -> usize {
        self.notes.iter().position(|n| n.id == self.current).unwrap_or(0)
    }

    fn touch(&mut self) {
        self.dirty_since = Some(f64::NEG_INFINITY);
    }

    fn store_editor(&mut self) {
        if !std::mem::take(&mut self.edited) {
            return;
        }
        let i = self.index();
        let note = &mut self.notes[i];
        note.markdown = self.editor.markdown();
        note.text = self.editor.doc.plain_text();
        note.updated_at = now_ms();
    }

    fn persist(&mut self) {
        self.store_editor();
        if let Err(e) = super::save(&self.data_dir, &self.notes) {
            eprintln!("openray: could not save notes ({e})");
        }
        self.dirty_since = None;
    }

    fn open_note(&mut self, ctx: &egui::Context, id: &str, record: bool) {
        self.store_editor();
        let Some(i) = self.notes.iter().position(|n| n.id == id) else { return };
        self.notes[i].last_accessed_at = now_ms();
        self.current = id.to_owned();
        self.editor.load(ctx, &self.notes[i].markdown);
        self.editor.focus_end(ctx);
        self.touch();
        if record {
            self.history.truncate(self.history_index + 1);
            self.history.retain(|x| x != id);
            self.history.push(id.to_owned());
            self.history_index = self.history.len() - 1;
        }
    }

    pub fn create(&mut self, ctx: &egui::Context) {
        self.store_editor();
        match self.notes.iter().find(|n| n.text.trim().is_empty()).map(|n| n.id.clone()) {
            Some(id) if id == self.current => notify(ctx, Toast::Failure("You already have an empty note".into(), String::new())),
            Some(id) => self.open_note(ctx, &id, true),
            None => {
                let note = super::new_note(now_ms());
                let id = note.id.clone();
                self.notes.push(note);
                self.open_note(ctx, &id, true);
            }
        }
    }

    fn remove(&mut self, ctx: &egui::Context, id: &str) {
        self.store_editor();
        self.notes.retain(|n| n.id != id);
        let before = self.history.len();
        self.history.retain(|x| x != id);
        if self.history.len() < before {
            self.history_index = self.history_index.min(self.history.len().saturating_sub(1));
        }
        if self.notes.is_empty() {
            self.notes.push(super::new_note(now_ms()));
        }
        if id == self.current {
            let next = self.notes[super::current(&self.notes).unwrap_or(0)].id.clone();
            self.current.clear();
            self.open_note(ctx, &next, true);
        }
        self.touch();
    }

    fn toggle_pin(&mut self, ctx: &egui::Context, id: &str) {
        if let Some(note) = self.notes.iter_mut().find(|n| n.id == id) {
            note.pinned = !note.pinned;
            notify(ctx, Toast::Success(if note.pinned { "Pinned Note" } else { "Unpinned Note" }.into()));
            self.touch();
        }
    }

    fn step(&mut self, ctx: &egui::Context, delta: isize) {
        let Some(index) = self.history_index.checked_add_signed(delta).filter(|&i| i < self.history.len()) else { return };
        self.history_index = index;
        let id = self.history[index].clone();
        self.open_note(ctx, &id, false);
    }

    fn save_prefs(&self) {
        let _ = crate::store::write_json(&self.data_dir.join("notes-ui.json"), &self.prefs);
    }

    fn heading_items() -> Vec<Item> {
        vec![
            item("Text", "pilcrow", Cmd::Format(Format::Block(Kind::Paragraph))).key(Shortcut::key(Key::Num0).ctrl().alt()),
            item("Heading 1", "heading-1", Cmd::Format(Format::Block(Kind::Heading(1)))).key(Shortcut::key(Key::Num1).ctrl().alt()),
            item("Heading 2", "heading-2", Cmd::Format(Format::Block(Kind::Heading(2)))).key(Shortcut::key(Key::Num2).ctrl().alt()),
            item("Heading 3", "heading-3", Cmd::Format(Format::Block(Kind::Heading(3)))).key(Shortcut::key(Key::Num3).ctrl().alt()),
        ]
    }

    fn inline_items() -> Vec<Item> {
        vec![
            item("Bold", "bold", Cmd::Format(Format::Mark(BOLD))).key(Shortcut::key(Key::B).ctrl()),
            item("Italic", "italic", Cmd::Format(Format::Mark(ITALIC))).key(Shortcut::key(Key::I).ctrl()),
            item("Underline", "underline", Cmd::Format(Format::Mark(UNDERLINE))).key(Shortcut::key(Key::U).ctrl()),
            item("Strikethrough", "strikethrough", Cmd::Format(Format::Mark(STRIKE))).key(Shortcut::key(Key::S).ctrl().shift()),
        ]
    }

    fn list_items() -> Vec<Item> {
        vec![
            item("Bulleted List", "list", Cmd::Format(Format::Block(Kind::Bullet))).key(Shortcut::key(Key::Num8).ctrl().shift()),
            item("Numbered List", "list-ordered", Cmd::Format(Format::Block(Kind::Ordered))).key(Shortcut::key(Key::Num7).ctrl().shift()),
            item("Task List", "list-checks", Cmd::Format(Format::Block(Kind::Task(false)))).key(Shortcut::key(Key::Num9).ctrl().shift()),
        ]
    }

    fn groups(&self, ctx: &egui::Context) -> Vec<Vec<Item>> {
        let note = &self.notes[self.index()];
        let empty = self.editor.doc.plain_text().trim().is_empty();
        let selection = self.editor.selection_doc(ctx).is_some();
        vec![
            vec![
                item("New Note", "plus", Cmd::New).key(CREATE).off(empty),
                item("Duplicate Note", "copy-plus", Cmd::Duplicate).off(empty),
                item(if note.pinned { "Unpin Note" } else { "Pin Note" }, if note.pinned { "pin-off" } else { "pin" }, Cmd::TogglePin(note.id.clone())).key(PIN),
            ],
            vec![
                item("Browse Notes", "files", Cmd::Browse).key(BROWSE),
                item("Go Back", "circle-arrow-left", Cmd::Back).key(BACK).off(self.history_index == 0),
                item("Go Forward", "circle-arrow-right", Cmd::Forward).key(FORWARD).off(self.history_index + 1 >= self.history.len()),
            ],
            vec![item(if selection { "Copy Selection As…" } else { "Copy Note As…" }, "copy", Cmd::CopyMenu).key(COPY)],
            vec![
                item("Format…", "type", Cmd::FormatMenu),
                item("Heading…", "heading", Cmd::HeadingMenu),
                item("List…", "list", Cmd::ListMenu),
                item(if self.prefs.format_bar { "Hide Format Bar" } else { "Show Format Bar" }, "type", Cmd::ToggleFormatBar).key(FORMAT_BAR),
            ],
            vec![item("Delete Note", "trash-2", Cmd::Delete(note.id.clone())).key(REMOVE).danger(), item("Delete All Notes", "trash-2", Cmd::DeleteAll).key(REMOVE_ALL).danger()],
        ]
    }

    fn menu(&mut self, title: &str, groups: Vec<Vec<Item>>, placement: Placement) {
        self.overlay = Some(Overlay::Menu { title: title.into(), groups, placement, query: String::new(), selected: 0 });
    }

    fn execute(&mut self, ctx: &egui::Context, cmd: Cmd) {
        let placement = match &self.overlay {
            Some(Overlay::Menu { placement, .. }) => *placement,
            _ => Placement::Top,
        };
        self.overlay = None;
        match cmd {
            Cmd::New => self.create(ctx),
            Cmd::Duplicate => {
                self.store_editor();
                let source = &self.notes[self.index()];
                let mut note = super::new_note(now_ms());
                (note.markdown, note.text) = (source.markdown.clone(), source.text.clone());
                let id = note.id.clone();
                self.notes.push(note);
                self.open_note(ctx, &id, true);
                notify(ctx, Toast::Success("Note duplicated".into()));
            }
            Cmd::TogglePin(id) => self.toggle_pin(ctx, &id),
            Cmd::Browse => self.overlay = Some(Overlay::Browse { query: String::new(), selected: 0 }),
            Cmd::Back => self.step(ctx, -1),
            Cmd::Forward => self.step(ctx, 1),
            Cmd::CopyMenu => {
                let title = if self.editor.selection_doc(ctx).is_some() { "Copy Selection As" } else { "Copy Note As" };
                self.menu(title, vec![vec![item("Markdown", "file-code", Cmd::Copy(CopyKind::Markdown)), item("Plain Text", "file-text", Cmd::Copy(CopyKind::Text)), item("HTML", "code", Cmd::Copy(CopyKind::Html))]], placement);
            }
            Cmd::Copy(kind) => {
                let doc = self.editor.selection_doc(ctx).unwrap_or_else(|| self.editor.doc.clone());
                let (title, text) = match kind {
                    CopyKind::Markdown => ("Markdown", doc.to_markdown()),
                    CopyKind::Text => ("Plain Text", doc.plain_text()),
                    CopyKind::Html => ("HTML", markdown_html(&doc.to_markdown())),
                };
                match crate::clipboard::write_text(&text) {
                    Ok(()) => notify(ctx, Toast::Success(format!("Copied as {title}"))),
                    Err(e) => notify(ctx, Toast::Failure("Could not copy note".into(), e)),
                }
                self.editor.focus(ctx);
            }
            Cmd::FormatMenu => self.menu("Format", vec![Self::inline_items()], placement),
            Cmd::HeadingMenu => self.menu("Heading", vec![Self::heading_items()], placement),
            Cmd::ListMenu => self.menu("List", vec![Self::list_items()], placement),
            Cmd::ToggleFormatBar => {
                self.prefs.format_bar = !self.prefs.format_bar;
                self.save_prefs();
                self.editor.focus(ctx);
            }
            Cmd::Delete(id) => {
                self.store_editor();
                let title = self.notes.iter().find(|n| n.id == id).map(|n| note_title(&n.text)).unwrap_or_default();
                self.overlay = Some(Overlay::Confirm { title: "Delete Note?".into(), message: format!("“{title}” will be deleted. This can't be undone."), confirm: "Delete Note".into(), cmd: Cmd::Delete(id), ok: true });
            }
            Cmd::DeleteAll => self.overlay = Some(Overlay::Confirm { title: "Delete All Notes?".into(), message: "All notes will be deleted. This can't be undone.".into(), confirm: "Delete All Notes".into(), cmd: Cmd::DeleteAll, ok: true }),
            Cmd::Format(format) => self.editor.apply(ctx, format),
        }
    }

    fn confirmed(&mut self, ctx: &egui::Context, cmd: Cmd) {
        self.overlay = None;
        match cmd {
            Cmd::Delete(id) => {
                self.remove(ctx, &id);
                notify(ctx, Toast::Success("Note deleted".into()));
            }
            Cmd::DeleteAll => {
                self.edited = false;
                let note = super::new_note(now_ms());
                let id = note.id.clone();
                self.notes = vec![note];
                self.history.clear();
                self.history_index = 0;
                self.current.clear();
                self.open_note(ctx, &id, true);
                notify(ctx, Toast::Success("All notes deleted".into()));
            }
            _ => {}
        }
    }

    fn browse_sections(&self, query: &str) -> Vec<(&'static str, Vec<usize>)> {
        let mut order: Vec<usize> = (0..self.notes.len()).filter(|&i| match_score(query, &note_title(&self.notes[i].text), std::slice::from_ref(&self.notes[i].text)) > 0).collect();
        order.sort_by(|&a, &b| self.notes[b].last_accessed_at.total_cmp(&self.notes[a].last_accessed_at));
        [("Pinned", true), ("Notes", false)].into_iter().map(|(title, pinned)| (title, order.iter().copied().filter(|&i| self.notes[i].pinned == pinned).collect::<Vec<_>>())).filter(|(_, list)| !list.is_empty()).collect()
    }

    fn filtered(groups: &[Vec<Item>], query: &str) -> Vec<Vec<Item>> {
        groups.iter().map(|g| g.iter().filter(|i| query.trim().is_empty() || match_score(query, &i.title, &[]) > 0).cloned().collect::<Vec<_>>()).filter(|g| !g.is_empty()).collect()
    }

    fn overlay_keys(&mut self, ui: &Ui) -> Option<(Cmd, bool)> {
        let ctx = ui.ctx().clone();
        let mut result = None;
        let browse_flat: Vec<usize> = match &self.overlay {
            Some(Overlay::Browse { query, .. }) => self.browse_sections(query).into_iter().flat_map(|(_, l)| l).collect(),
            _ => Vec::new(),
        };
        for (key, modifiers) in key_presses(ui) {
            let mut handled = true;
            let mut close = false;
            match &mut self.overlay {
                Some(Overlay::Menu { groups, query, selected, .. }) => {
                    let items: Vec<Item> = Self::filtered(groups, query).into_iter().flatten().collect();
                    if (key == Key::Escape && plain(modifiers)) || ACTIONS.matches(key, modifiers) {
                        close = true;
                    } else if key == Key::ArrowDown && plain(modifiers) {
                        *selected = (*selected + 1).min(items.len().saturating_sub(1));
                    } else if key == Key::ArrowUp && plain(modifiers) {
                        *selected = selected.saturating_sub(1);
                    } else if key == Key::Enter && plain(modifiers) {
                        result = items.get(*selected).filter(|i| !i.disabled).map(|i| (i.cmd.clone(), false));
                    } else if let Some(found) = items.iter().find(|i| i.shortcut.is_some_and(|s| s.matches(key, modifiers))) {
                        result = (!found.disabled).then(|| (found.cmd.clone(), false));
                    } else {
                        handled = false;
                    }
                }
                Some(Overlay::Browse { selected, .. }) => {
                    let current = browse_flat.get((*selected).min(browse_flat.len().saturating_sub(1))).map(|&i| self.notes[i].id.clone());
                    if (key == Key::Escape && plain(modifiers)) || BROWSE.matches(key, modifiers) {
                        close = true;
                    } else if key == Key::ArrowDown && plain(modifiers) {
                        *selected = (*selected + 1).min(browse_flat.len().saturating_sub(1));
                    } else if key == Key::ArrowUp && plain(modifiers) {
                        *selected = selected.saturating_sub(1);
                    } else if key == Key::Enter && plain(modifiers) {
                        if let Some(id) = current {
                            self.overlay = None;
                            self.open_note(&ctx, &id, true);
                        }
                    } else if PIN.matches(key, modifiers) {
                        if let Some(id) = current {
                            self.toggle_pin(&ctx, &id);
                        }
                    } else if REMOVE.matches(key, modifiers) {
                        result = current.map(|id| (Cmd::Delete(id), false));
                    } else {
                        handled = false;
                    }
                }
                Some(Overlay::Link { url }) => {
                    if key == Key::Escape {
                        close = true;
                    } else if key == Key::Enter && plain(modifiers) {
                        let url = url.trim().to_owned();
                        self.overlay = None;
                        if url.is_empty() {
                            self.editor.set_link(&ctx, None);
                        } else {
                            self.editor.set_link(&ctx, Some(&with_protocol(&url)));
                        }
                    } else {
                        handled = false;
                    }
                }
                Some(Overlay::Confirm { cmd, ok, .. }) => {
                    if key == Key::Escape {
                        close = true;
                    } else if key == Key::Enter && plain(modifiers) {
                        if *ok {
                            result = Some((cmd.clone(), true));
                        } else {
                            close = true;
                        }
                    } else if key == Key::Tab {
                        *ok = !*ok;
                    }
                }
                None => handled = false,
            }
            if close {
                self.overlay = None;
                self.editor.focus(&ctx);
            }
            if handled {
                consume(ui, key, modifiers);
            }
            if result.is_some() || self.overlay.is_none() {
                break;
            }
        }
        result
    }

    fn window_keys(&mut self, ui: &Ui, escape_closes: bool) -> (Option<Cmd>, bool) {
        let groups = self.groups(ui.ctx());
        for (key, modifiers) in key_presses(ui) {
            if ACTIONS.matches(key, modifiers) {
                consume(ui, key, modifiers);
                self.menu("Actions", groups, Placement::Top);
                return (None, false);
            }
            if key == Key::Escape && plain(modifiers) && escape_closes {
                consume(ui, key, modifiers);
                return (None, true);
            }
            if let Some(found) = groups.iter().flatten().find(|i| i.shortcut.is_some_and(|s| s.matches(key, modifiers))) {
                consume(ui, key, modifiers);
                return ((!found.disabled).then(|| found.cmd.clone()), false);
            }
        }
        (None, false)
    }

    fn style_window(&mut self) {
        if self.styled {
            return;
        }
        if let Some(hwnd) = crate::focus_ui::own_window(TITLE) {
            use windows::Win32::Graphics::Dwm::{DWMWA_BORDER_COLOR, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND};
            launcher::set_dwm(hwnd, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND);
            launcher::set_dwm(hwnd, DWMWA_BORDER_COLOR, theme::WINDOW_BORDER());
            self.styled = true;
        }
    }

    pub fn ui(&mut self, ui: &mut Ui, escape_closes: bool) -> bool {
        let ctx = ui.ctx().clone();
        let time = ui.input(|i| i.time);
        self.style_window();
        if std::mem::take(&mut self.focus) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            if self.overlay.is_none() {
                self.editor.focus(&ctx);
            }
        }
        let full = ui.max_rect();
        ui.painter().rect_filled(full, 0.0, theme::BG());
        let (mut run, mut close) = (None, false);
        if self.overlay.is_some() {
            if let Some((cmd, confirmed)) = self.overlay_keys(ui) {
                if confirmed {
                    self.confirmed(&ctx, cmd);
                } else {
                    run = Some(cmd);
                }
            }
        } else {
            (run, close) = self.window_keys(ui, escape_closes);
        }
        let header = Rect::from_min_size(full.min, vec2(full.width(), HEADER));
        let footer = Rect::from_min_max(pos2(full.left(), full.bottom() - FOOTER), full.max);
        close |= self.header(ui, header, &mut run);
        self.footer(ui, footer, &mut run);
        self.body(ui, Rect::from_min_max(pos2(full.left(), header.bottom() + 1.0), pos2(full.right(), footer.top())), time);
        if self.overlay.is_some() {
            self.overlay_ui(ui, full, &mut run);
        }
        if let Some(cmd) = run {
            self.execute(&ctx, cmd);
        }
        if let Some(since) = self.dirty_since {
            if time - since >= SAVE_DELAY {
                self.persist();
            } else {
                ctx.request_repaint_after(std::time::Duration::from_secs_f64(SAVE_DELAY));
            }
        }
        close |= ui.input(|i| i.viewport().close_requested());
        if close {
            self.persist();
        }
        close
    }

    fn header(&mut self, ui: &mut Ui, rect: Rect, run: &mut Option<Cmd>) -> bool {
        let drag = ui.interact(rect, egui::Id::unique("notes-drag"), Sense::click_and_drag());
        launcher::follow_drag(&drag, crate::focus_ui::own_window(TITLE), &mut self.drag);
        let button = |i: f32| Rect::from_min_size(pos2(rect.left() + 8.0 + i * 32.0, rect.top() + 8.0), vec2(24.0, 24.0));
        if icon_button(ui, button(0.0), "actions", "terminal", "Action Panel", Some(ACTIONS)) {
            self.menu("Actions", self.groups(ui.ctx()), Placement::Top);
        }
        if icon_button(ui, button(1.0), "browse", "files", "Browse Notes", Some(BROWSE)) {
            *run = Some(Cmd::Browse);
        }
        if icon_button(ui, button(2.0), "create", "plus", "Create Note", Some(CREATE)) {
            *run = Some(Cmd::New);
        }
        let title = note_title(&self.editor.doc.plain_text());
        let middle = Rect::from_min_max(pos2(rect.left() + 108.0, rect.top()), pos2(rect.right() - 108.0, rect.bottom()));
        paint_text(ui, middle.center(), Align2::CENTER_CENTER, &title, theme::font(13.0), theme::FG_60(), middle.width());
        icon_button(ui, Rect::from_min_size(pos2(rect.right() - 32.0, rect.top() + 8.0), vec2(24.0, 24.0)), "close", "x", "Close", None)
    }

    fn footer(&mut self, ui: &mut Ui, rect: Rect, run: &mut Option<Cmd>) {
        let ctx = ui.ctx().clone();
        let y = rect.center().y - 12.0;
        let right = Rect::from_min_size(pos2(rect.right() - 32.0, y), vec2(24.0, 24.0));
        if !self.prefs.format_bar {
            let label = count_label(&self.editor.doc.plain_text(), self.prefs.words);
            let width = paint_text(ui, pos2(-1000.0, -1000.0), Align2::LEFT_TOP, &label, theme::font(12.0), Color32::TRANSPARENT, 400.0).width() + 16.0;
            let counter = Rect::from_center_size(pos2(rect.center().x, rect.center().y), vec2(width, 24.0));
            let response = ui.interact(counter, egui::Id::unique("notes-counter"), Sense::click());
            response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &label));
            paint_text(ui, counter.center(), Align2::CENTER_CENTER, &label, theme::font(12.0), if response.hovered() { theme::FG_60() } else { theme::FG_40() }, width);
            if response.on_hover_text(tip("Toggle between character count and word count", None)).clicked() {
                self.prefs.words = !self.prefs.words;
                self.save_prefs();
            }
            if format_button(ui, right, "show", "type", false, false, "Show Format Buttons", Some(FORMAT_BAR)) {
                *run = Some(Cmd::ToggleFormatBar);
            }
            return;
        }
        let (kind, style) = self.editor.state_at_cursor(&ctx);
        let inline = style.marks & (BOLD | ITALIC | UNDERLINE | STRIKE) != 0;
        enum Part {
            Button(&'static str, &'static str, &'static str, bool, bool, Option<Shortcut>),
            Divider,
        }
        let parts = [
            Part::Button("heading", "heading", "Heading", true, matches!(kind, Kind::Heading(_)), None),
            Part::Button("format", "type", "Format", true, inline, None),
            Part::Button("link", "link", "Link", false, style.link.is_some(), None),
            Part::Button("code", "code", "Code", false, style.marks & CODE != 0, Some(Shortcut::key(Key::E).ctrl())),
            Part::Divider,
            Part::Button("quote", "text-quote", "Quote", false, kind == Kind::Quote, Some(Shortcut::key(Key::B).ctrl().shift())),
            Part::Button("code-block", "square-code", "Code Block", false, kind == Kind::Code, Some(Shortcut::key(Key::C).ctrl().alt())),
            Part::Divider,
            Part::Button("list", "list", "List", true, kind.is_list(), None),
        ];
        let width_of = |p: &Part| match p {
            Part::Button(_, _, _, true, _, _) => 38.0,
            Part::Button(..) => 24.0,
            Part::Divider => 9.0,
        };
        let total: f32 = parts.iter().map(width_of).sum::<f32>() + 4.0 * (parts.len() - 1) as f32;
        let mut x = rect.center().x - total / 2.0;
        let mut clicked = None;
        for part in &parts {
            let w = width_of(part);
            match part {
                Part::Button(id, glyph, label, menu, active, shortcut) => {
                    if format_button(ui, Rect::from_min_size(pos2(x, y), vec2(w, 24.0)), id, glyph, *menu, *active, label, *shortcut) {
                        clicked = Some(*id);
                    }
                }
                Part::Divider => {
                    ui.painter().vline(x + 4.5, (rect.center().y - 8.0)..=(rect.center().y + 8.0), Stroke::new(1.0, theme::SEPARATOR()));
                }
            }
            x += w + 4.0;
        }
        match clicked {
            Some("heading") => self.menu("Heading", vec![Self::heading_items()], Placement::Format),
            Some("format") => self.menu("Format", vec![Self::inline_items()], Placement::Format),
            Some("list") => self.menu("List", vec![Self::list_items()], Placement::Format),
            Some("link") if style.link.is_some() => self.editor.set_link(&ctx, None),
            Some("link") => self.overlay = Some(Overlay::Link { url: String::new() }),
            Some("code") => self.editor.apply(&ctx, Format::Mark(CODE)),
            Some("quote") => self.editor.apply(&ctx, Format::Block(Kind::Quote)),
            Some("code-block") => self.editor.apply(&ctx, Format::Block(Kind::Code)),
            _ => {}
        }
        if format_button(ui, right, "hide", "circle-x", false, false, "Hide Format Buttons", Some(FORMAT_BAR)) {
            *run = Some(Cmd::ToggleFormatBar);
        }
    }

    fn body(&mut self, ui: &mut Ui, rect: Rect, time: f64) {
        if self.scrolled {
            ui.painter().hline(rect.x_range(), rect.top() - 0.5, Stroke::new(1.0, theme::SEPARATOR()));
        }
        let mut child = ui.new_child(UiBuilder::new().max_rect(rect));
        let output = egui::ScrollArea::vertical().id_salt("notes-scroll").auto_shrink(false).show(&mut child, |ui| {
            let changed = egui::Frame::NONE.inner_margin(egui::Margin { left: 16, right: 16, top: 4, bottom: 16 }).show(ui, |ui| self.editor.show(ui)).inner;
            let rest = ui.available_rect_before_wrap();
            if rest.height() > 0.0 {
                let response = ui.allocate_rect(rest, Sense::click());
                if response.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Text);
                }
                if response.clicked() {
                    self.editor.focus_end(ui.ctx());
                }
            }
            changed
        });
        self.scrolled = output.state.clamped_offset().y > 0.0;
        if output.inner {
            self.edited = true;
            self.dirty_since.get_or_insert(time);
        }
    }

    fn overlay_ui(&mut self, ui: &mut Ui, full: Rect, run: &mut Option<Cmd>) {
        let ctx = ui.ctx().clone();
        let mut frame_rect = None;
        let mut confirm_answer = None;
        let mut open = None;
        let mut pin = None;
        let browse_sections = match &self.overlay {
            Some(Overlay::Browse { query, .. }) => self.browse_sections(query),
            _ => Vec::new(),
        };
        let now = now_ms();
        let current = self.current.clone();
        let editor_text = self.editor.doc.plain_text();
        let notes: Vec<(String, String, bool, f64)> = self.notes.iter().map(|n| (n.id.clone(), if n.id == current { editor_text.clone() } else { n.text.clone() }, n.pinned, n.last_accessed_at)).collect();
        egui::Area::new(egui::Id::unique("notes-overlay")).order(egui::Order::Foreground).fixed_pos(full.min).show(&ctx, |ui| match &mut self.overlay {
            Some(Overlay::Menu { title, groups, placement, query, selected }) => {
                let groups = Self::filtered(groups, query);
                let rows: usize = groups.iter().map(Vec::len).sum();
                let content = 7.0 + 24.0 + rows.max(1) as f32 * 32.0 + groups.len().saturating_sub(1) as f32 * 16.0 + 8.0;
                let list_height = content.min(MENU_LIST_MAX);
                let width = if *placement == Placement::Top { TOP_PANEL_WIDTH } else { FORMAT_PANEL_WIDTH };
                let height = SEARCH + list_height + 2.0;
                let top = if *placement == Placement::Top { full.top() + TOP_PANEL_Y } else { full.bottom() - FORMAT_PANEL_BOTTOM - height };
                let frame = Rect::from_min_size(pos2(full.center().x - width / 2.0, top), vec2(width, height));
                frame_rect = Some(frame);
                *selected = (*selected).min(rows.saturating_sub(1));
                panel_frame(ui, frame);
                if search_field(ui, frame, "menu", query, "Search for actions...") {
                    *selected = 0;
                }
                let list = Rect::from_min_max(pos2(frame.left() + 1.0, frame.top() + 1.0 + SEARCH), pos2(frame.right() - 1.0, frame.bottom() - 1.0));
                let pointer_moved = ui.input(|i| i.pointer.delta() != egui::Vec2::ZERO);
                ui.scope_builder(UiBuilder::new().max_rect(list), |ui| {
                    egui::ScrollArea::vertical().id_salt("notes-menu-list").scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden).max_height(list_height).show(ui, |ui| {
                        ui.add_space(7.0);
                        let width = ui.available_width();
                        let (head, _) = ui.allocate_exact_size(vec2(width, 24.0), Sense::hover());
                        paint_text(ui, pos2(head.left() + 15.0, head.center().y), Align2::LEFT_CENTER, title, theme::weighted(11.0, theme::SEMIBOLD), theme::FG_60(), width - 30.0);
                        if rows == 0 {
                            let (row, _) = ui.allocate_exact_size(vec2(width, 32.0), Sense::hover());
                            paint_text(ui, pos2(row.left() + 15.0, row.center().y), Align2::LEFT_CENTER, "No matching actions", theme::font(13.0), theme::FG_60(), width - 30.0);
                        }
                        let mut index = 0;
                        for (g, group) in groups.iter().enumerate() {
                            if g > 0 {
                                ui.add_space(8.0);
                                let (line, _) = ui.allocate_exact_size(vec2(width, 1.0), Sense::hover());
                                ui.painter().rect_filled(line, 0.0, theme::PANEL_LINE());
                                ui.add_space(7.0);
                            }
                            for action in group {
                                let (slot, response) = ui.allocate_exact_size(vec2(width, 32.0), Sense::click());
                                let row = slot.shrink2(vec2(7.0, 0.0));
                                let is_selected = index == *selected;
                                response.widget_info(|| egui::WidgetInfo::selected(egui::Role::ListBoxOption, !action.disabled, is_selected, &action.title));
                                if is_selected {
                                    ui.painter().rect_filled(row, 6.0, theme::PANEL_LINE());
                                }
                                if response.hovered() && pointer_moved {
                                    *selected = index;
                                }
                                if response.clicked() && !action.disabled {
                                    *run = Some(action.cmd.clone());
                                }
                                let base = if action.destructive { theme::RED() } else { theme::FG() };
                                let color = if action.disabled { base.gamma_multiply(0.4) } else { base };
                                icons::paint_glyph(ui, action.icon, Rect::from_min_size(pos2(row.left() + 8.0, row.center().y - 8.0), vec2(16.0, 16.0)), color);
                                let end = action.shortcut.map(|s| keycaps(ui, row.right() - 8.0, row.center().y, &s.labels())).unwrap_or(row.right() - 8.0);
                                paint_text(ui, pos2(row.left() + 32.0, row.center().y), Align2::LEFT_CENTER, &action.title, theme::weighted(13.0, theme::MEDIUM), color, end - row.left() - 40.0);
                                index += 1;
                            }
                        }
                        ui.add_space(8.0);
                    });
                });
            }
            Some(Overlay::Browse { query, selected }) => {
                let flat: Vec<usize> = browse_sections.iter().flat_map(|(_, l)| l.iter().copied()).collect();
                let content = 7.0 + browse_sections.len() as f32 * 24.0 + flat.len().max(1) as f32 * 52.0 + 8.0;
                let list_height = if flat.is_empty() { 7.0 + 32.0 + 8.0 } else { content.min(BROWSE_LIST_MAX) };
                let height = SEARCH + list_height + 2.0;
                let frame = Rect::from_min_size(pos2(full.center().x - TOP_PANEL_WIDTH / 2.0, full.top() + TOP_PANEL_Y), vec2(TOP_PANEL_WIDTH, height));
                frame_rect = Some(frame);
                *selected = (*selected).min(flat.len().saturating_sub(1));
                panel_frame(ui, frame);
                if search_field(ui, frame, "browse", query, "Search for notes...") {
                    *selected = 0;
                }
                let list = Rect::from_min_max(pos2(frame.left() + 1.0, frame.top() + 1.0 + SEARCH), pos2(frame.right() - 1.0, frame.bottom() - 1.0));
                let pointer_moved = ui.input(|i| i.pointer.delta() != egui::Vec2::ZERO);
                ui.scope_builder(UiBuilder::new().max_rect(list), |ui| {
                    egui::ScrollArea::vertical().id_salt("notes-browse-list").scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden).max_height(list_height).show(ui, |ui| {
                        ui.add_space(7.0);
                        let width = ui.available_width();
                        if flat.is_empty() {
                            let (row, _) = ui.allocate_exact_size(vec2(width, 32.0), Sense::hover());
                            paint_text(ui, pos2(row.left() + 15.0, row.center().y), Align2::LEFT_CENTER, "No matching notes", theme::font(13.0), theme::FG_60(), width - 30.0);
                        }
                        let mut index = 0;
                        for (title, list) in &browse_sections {
                            let (head, _) = ui.allocate_exact_size(vec2(width, 24.0), Sense::hover());
                            paint_text(ui, pos2(head.left() + 15.0, head.center().y), Align2::LEFT_CENTER, title, theme::weighted(11.0, theme::SEMIBOLD), theme::FG_60(), width - 30.0);
                            for &n in list {
                                let (id, text, pinned, accessed) = &notes[n];
                                let (slot, response) = ui.allocate_exact_size(vec2(width, 52.0), Sense::click());
                                let row = slot.shrink2(vec2(7.0, 0.0));
                                let is_selected = index == *selected;
                                let name = note_title(text);
                                response.widget_info(|| egui::WidgetInfo::selected(egui::Role::ListBoxOption, true, is_selected, &name));
                                if is_selected {
                                    ui.painter().rect_filled(row, 6.0, theme::PANEL_LINE());
                                }
                                if response.hovered() && pointer_moved {
                                    *selected = index;
                                }
                                let buttons = Rect::from_min_max(pos2(row.right() - 8.0 - 52.0, row.center().y - 12.0), pos2(row.right() - 8.0, row.center().y + 12.0));
                                let mut end = row.right() - 8.0;
                                if is_selected {
                                    end = buttons.left() - 8.0;
                                    let pin_rect = Rect::from_min_size(buttons.min, vec2(24.0, 24.0));
                                    let delete_rect = Rect::from_min_size(pos2(buttons.left() + 28.0, buttons.top()), vec2(24.0, 24.0));
                                    for (rect, glyph, label, key) in [(pin_rect, if *pinned { "pin-off" } else { "pin" }, if *pinned { "Unpin note" } else { "Pin note" }, "pin"), (delete_rect, "trash-2", "Delete note", "delete")] {
                                        let b = ui.interact(rect, egui::Id::unique(("notes-browse-button", id.as_str(), key)), Sense::click());
                                        b.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, label));
                                        if b.hovered() {
                                            ui.painter().rect_filled(rect, 5.0, theme::CONTROL());
                                        }
                                        icons::paint_glyph(ui, glyph, Rect::from_center_size(rect.center(), vec2(16.0, 16.0)), if b.hovered() { theme::FG() } else { theme::FG_60() });
                                        if b.clicked() {
                                            if key == "pin" {
                                                pin = Some(id.clone());
                                            } else {
                                                *run = Some(Cmd::Delete(id.clone()));
                                            }
                                        }
                                    }
                                }
                                if response.clicked() && run.is_none() && pin.is_none() {
                                    open = Some(id.clone());
                                }
                                paint_text(ui, pos2(row.left() + 8.0, row.top() + 17.0), Align2::LEFT_CENTER, &name, theme::weighted(13.0, theme::MEDIUM), theme::FG(), end - row.left() - 8.0);
                                let count = crate::i18n::tr(&count_label(text, false)).into_owned();
                                let mut x = row.left() + 8.0;
                                let subtitle = if *id == current {
                                    ui.painter().circle_filled(pos2(x + 2.5, row.top() + 35.0), 2.5, theme::RED());
                                    x += 9.0;
                                    format!("{} • {count}", crate::i18n::tr("Current"))
                                } else {
                                    format!("{} • {count}", crate::i18n::tr(&crate::extensions::calculator::time::updated_ago(*accessed as u64, now as u64).replacen("Updated", "Opened", 1)))
                                };
                                paint_text(ui, pos2(x, row.top() + 35.0), Align2::LEFT_CENTER, &subtitle, theme::font(12.0), theme::FG_60(), end - x);
                                index += 1;
                            }
                        }
                        ui.add_space(8.0);
                    });
                });
            }
            Some(Overlay::Link { url }) => {
                let frame = Rect::from_min_size(pos2(full.center().x - FORMAT_PANEL_WIDTH / 2.0, full.bottom() - FORMAT_PANEL_BOTTOM - SEARCH - 2.0), vec2(FORMAT_PANEL_WIDTH, SEARCH + 2.0));
                frame_rect = Some(frame);
                panel_frame(ui, frame);
                let field = frame.shrink2(vec2(16.0, 1.0));
                let id = egui::Id::unique("notes-link");
                let edit = egui::TextEdit::singleline(url).id(id).frame(egui::Frame::NONE).margin(egui::Margin::ZERO).font(theme::font(13.0)).text_color(theme::FG()).hint_text(egui::RichText::new(crate::i18n::tr("Paste or type a link...")).color(theme::FG_40())).desired_width(field.width());
                ui.put(Rect::from_center_size(field.center(), vec2(field.width(), 18.0)), edit);
                ui.memory_mut(|m| m.request_focus(id));
            }
            Some(Overlay::Confirm { title, message, confirm, ok, .. }) => {
                ui.painter().rect_filled(full, 0.0, Color32::from_black_alpha(102));
                let width = 300.0;
                let message_galley = crate::root::wrapped(ui, message, theme::font(13.0), theme::FG_60(), width - 32.0);
                let height = 16.0 + 18.0 + 4.0 + message_galley.size().y + 16.0 + 32.0 + 8.0 + 32.0 + 16.0;
                let panel = Rect::from_center_size(full.center(), vec2(width, height));
                frame_rect = Some(full);
                ui.painter().rect(panel, 6.0, theme::PANEL(), Stroke::new(1.0, theme::PANEL_BORDER()), egui::StrokeKind::Inside);
                paint_text(ui, pos2(panel.center().x, panel.top() + 16.0), Align2::CENTER_TOP, title, theme::weighted(13.0, theme::SEMIBOLD), theme::FG(), width - 32.0);
                ui.painter().galley(pos2(panel.center().x - message_galley.size().x / 2.0, panel.top() + 38.0), message_galley.clone(), theme::FG_60());
                let mut y = panel.top() + 38.0 + message_galley.size().y + 16.0;
                for (label, color, is_ok) in [(confirm.as_str(), theme::RED(), true), ("Cancel", theme::FG(), false)] {
                    let rect = Rect::from_min_size(pos2(panel.left() + 16.0, y), vec2(width - 32.0, 32.0));
                    let response = ui.interact(rect, egui::Id::unique(("notes-confirm", is_ok)), Sense::click());
                    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, label));
                    ui.painter().rect_filled(rect, 6.0, if response.hovered() { theme::CONTROL_HOVER() } else { theme::CONTROL() });
                    if *ok == is_ok {
                        ui.painter().rect_stroke(rect.expand(2.0), 8.0, Stroke::new(2.0, theme::ACCENT()), egui::StrokeKind::Outside);
                    }
                    paint_text(ui, rect.center(), Align2::CENTER_CENTER, label, theme::weighted(13.0, theme::MEDIUM), color, rect.width());
                    if response.clicked() {
                        confirm_answer = Some(is_ok);
                    }
                    y += 40.0;
                }
            }
            None => {}
        });
        if let Some(id) = pin {
            self.toggle_pin(&ctx, &id);
        }
        if let Some(id) = open {
            self.overlay = None;
            self.open_note(&ctx, &id, true);
            return;
        }
        match confirm_answer {
            Some(true) => {
                if let Some(Overlay::Confirm { cmd, .. }) = self.overlay.take() {
                    self.confirmed(&ctx, cmd);
                }
                return;
            }
            Some(false) => {
                self.overlay = None;
                self.editor.focus(&ctx);
                return;
            }
            None => {}
        }
        let outside = ctx.input(|i| i.pointer.any_pressed()) && ctx.input(|i| i.pointer.interact_pos()).zip(frame_rect).is_some_and(|(p, f)| !f.contains(p));
        if outside && run.is_none() {
            self.overlay = None;
            self.editor.focus(&ctx);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titles_counts_and_links_match_the_tauri_helpers() {
        assert_eq!(note_title("\n  First line \nsecond"), "First line");
        assert_eq!(note_title("  \n "), "Untitled");
        assert_eq!(count_label(" a ", false), "1 character");
        assert_eq!(count_label(&"x".repeat(1234), false), "1,234 characters");
        assert_eq!(count_label("one two  three", true), "3 words");
        assert_eq!(count_label("", true), "0 words");
        assert_eq!(with_protocol("openray.dev"), "https://openray.dev");
        assert_eq!(with_protocol("mailto:a@b.c"), "mailto:a@b.c");
        assert!(markdown_html("**hi**").contains("<strong>hi</strong>"));
    }
}
