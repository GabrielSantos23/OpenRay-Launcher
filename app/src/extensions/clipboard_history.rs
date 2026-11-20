use super::calculator::color;
use super::command;
use crate::clipboard::{self, Content, Entry, History, PasteSequence};
use crate::extension::{frontmost_app, paste_title, Action, Command, Dropdown, Empty, Extension, Host, Icon, Item, Section, Shortcut, Tone, View};
use crate::icu::{self, Style};
use crate::{root, theme};
use egui::{pos2, vec2, Rect, Ui};
use regex::Regex;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{LazyLock, Mutex, OnceLock};

pub static HISTORY: OnceLock<History> = OnceLock::new();
pub static VERSION: AtomicU64 = AtomicU64::new(0);
static SEQUENCE: Mutex<Option<PasteSequence>> = Mutex::new(None);
const SEQUENCE_TIMEOUT_MS: u64 = 60_000;
const ICON: Icon = Icon::Tile("clipboard-list", Tone::Red);
const DAY: i64 = 86_400_000;

pub fn start(ctx: &egui::Context) {
    let now = crate::store::now_ms();
    if HISTORY.set(History::load(crate::data_dir().join("clipboard"), now)).is_err() {
        return;
    }
    let ctx = ctx.clone();
    let result = clipboard::listen(Box::new(move || {
        if clipboard::suppressed(crate::store::now_ms()) {
            return;
        }
        let Some(captured) = clipboard::capture() else { return };
        let (name, path) = clipboard::owner_app().unwrap_or_default();
        let Some(history) = HISTORY.get() else { return };
        let now = crate::store::now_ms();
        history.prune(now);
        match history.record(captured, name, path, now) {
            Ok(()) => {
                VERSION.fetch_add(1, Ordering::SeqCst);
                ctx.request_repaint();
            }
            Err(e) => eprintln!("openray: could not save clipboard entry: {e}"),
        }
    }));
    if let Err(e) = result {
        eprintln!("openray: clipboard history unavailable: {e}");
    }
}

fn history() -> Result<&'static History, String> {
    HISTORY.get().ok_or_else(|| "Clipboard History is not running".to_string())
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Kind {
    Text,
    Image,
    File,
    Link,
    Email,
    Color,
}

impl Kind {
    fn label(self) -> &'static str {
        match self {
            Kind::Text => "Text",
            Kind::Image => "Image",
            Kind::File => "File",
            Kind::Link => "Link",
            Kind::Email => "Email",
            Kind::Color => "Color",
        }
    }
}

static EMAIL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^(mailto:)?[^\s@]+@[^\s@]+\.[^\s@]+$").unwrap());
static LINK: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^(https?|ftp)://\S+$").unwrap());

fn text_of(entry: &Entry) -> &str {
    match &entry.content {
        Content::Text { text } => text,
        _ => "",
    }
}

pub fn kind(entry: &Entry) -> Kind {
    match &entry.content {
        Content::Image { .. } => Kind::Image,
        Content::Files { .. } => Kind::File,
        Content::Text { text } => {
            let text = text.trim();
            if EMAIL.is_match(text) {
                Kind::Email
            } else if LINK.is_match(text) {
                Kind::Link
            } else if color::parse(text).is_some() {
                Kind::Color
            } else {
                Kind::Text
            }
        }
    }
}

const FILTERS: [(&str, &str); 7] = [("All Types", "list"), ("Text Only", "type"), ("Images Only", "image"), ("Files Only", "file"), ("Links Only", "link"), ("Emails Only", "at-sign"), ("Colors Only", "pipette")];

pub fn matches(entry: &Entry, filter: usize) -> bool {
    let kind = kind(entry);
    match filter {
        1 => matches!(entry.content, Content::Text { .. }),
        2 => kind == Kind::Image,
        3 => kind == Kind::File,
        4 => kind == Kind::Link || kind == Kind::Email,
        5 => kind == Kind::Email,
        6 => kind == Kind::Color,
        _ => true,
    }
}

fn basename(path: &str) -> &str {
    path.split(['\\', '/']).filter(|p| !p.is_empty()).last().unwrap_or(path)
}

pub fn title(entry: &Entry) -> String {
    match &entry.content {
        Content::Image { width, height, .. } => format!("Image ({width}×{height})"),
        Content::Files { paths } if paths.len() == 1 => basename(&paths[0]).to_owned(),
        Content::Files { paths } => format!("{} Files", paths.len()),
        Content::Text { text } => text.lines().find(|l| !l.trim().is_empty()).unwrap_or("").trim().chars().take(200).collect(),
    }
}

pub fn word_count(text: &str) -> usize {
    text.split_whitespace().count()
}

fn local_day(ms: u64, offset: i64) -> i64 {
    (ms as i64 + offset).div_euclid(DAY)
}

pub fn day_title(ms: u64, now: u64, offset: i64) -> String {
    match local_day(now, offset) - local_day(ms, offset) {
        d if d <= 0 => "Today".into(),
        1 => "Yesterday".into(),
        _ => icu::format_pattern(ms as f64, "en-US", "MMMM d", None),
    }
}

pub fn group(entries: Vec<Entry>, title_of: impl Fn(u64) -> String) -> Vec<(String, Vec<Entry>)> {
    let mut groups: Vec<(String, Vec<Entry>)> = Vec::new();
    for entry in entries {
        let title = if entry.pinned.is_some() { "Pinned".to_owned() } else { title_of(entry.updated_at) };
        match groups.last_mut() {
            Some((last, items)) if *last == title => items.push(entry),
            _ => groups.push((title, vec![entry])),
        }
    }
    groups
}

pub fn format_bytes(bytes: u64) -> String {
    let b = bytes as f64;
    match bytes {
        0..1024 => format!("{bytes} B"),
        1024..1_048_576 => format!("{:.1} KB", b / 1024.0),
        1_048_576..1_073_741_824 => format!("{:.1} MB", b / 1_048_576.0),
        _ => format!("{:.1} GB", b / 1_073_741_824.0),
    }
}

fn changed(toast: &'static str, change: impl Fn(&History) -> Result<(), String> + 'static) -> impl Fn(&mut Host) -> Result<(), String> + 'static {
    move |host| {
        change(history()?)?;
        VERSION.fetch_add(1, Ordering::SeqCst);
        host.toast = Some(crate::extension::Toast::Success(toast.into()));
        Ok(())
    }
}

fn entry_actions(entry: &Entry, app: &Option<String>) -> Vec<Vec<Action>> {
    let kind = kind(entry);
    let text = text_of(entry).trim().to_owned();
    let mut secondary = Vec::new();
    if kind == Kind::Color {
        let rgba = color::parse(&text).map(|c| c.rgba);
        secondary.push(
            Action::new("copy-color", "Copy Color as…", |_| Ok(()))
                .icon("clipboard")
                .shortcut(Shortcut::key(egui::Key::C).ctrl().alt())
                .submenu(move || {
                    rgba.map(|c| color::formats(&c))
                        .unwrap_or_default()
                        .into_iter()
                        .map(|f| {
                            let (value, title) = (f.value.clone(), format!("Copied {} to Clipboard", f.label));
                            Action::new(f.id, f.label, move |host| {
                                host.copy_and_close(&value, &title);
                                Ok(())
                            })
                            .accessory(f.value)
                        })
                        .collect()
                }),
        );
    }
    if kind == Kind::Link {
        let url = text.clone();
        secondary.push(Action::new("open-link", "Open in Browser", move |_| crate::apps::open_url(&url, None)).icon("external-link").shortcut(Shortcut::key(egui::Key::O).ctrl()));
    }
    if kind == Kind::Email {
        let url = if text.starts_with("mailto:") { text.clone() } else { format!("mailto:{text}") };
        secondary.push(Action::new("send-email", "Compose Email", move |_| crate::apps::open_url(&url, None)).icon("at-sign").shortcut(Shortcut::key(egui::Key::O).ctrl()));
    }
    if let Content::Files { paths } = &entry.content {
        if let [path] = paths.as_slice() {
            let (open, show) = (path.clone(), path.clone());
            secondary.push(Action::new("open-file", "Open", move |_| crate::apps::launch(&open, false)).icon("external-link").shortcut(Shortcut::key(egui::Key::O).ctrl()));
            secondary.push(Action::new("show-in-explorer", "Show in File Explorer", move |_| crate::apps::show_in_explorer(&show)).icon("folder").shortcut(Shortcut::key(egui::Key::O).ctrl().shift()));
        }
    }
    let id = entry.id.clone();
    let pin = entry.pinned.is_none();
    let pin_id = id.clone();
    secondary.push(
        Action::new(if pin { "pin" } else { "unpin" }, if pin { "Pin Entry" } else { "Unpin Entry" }, changed(if pin { "Pinned Entry" } else { "Unpinned Entry" }, move |h| h.set_pinned(&pin_id, pin, crate::store::now_ms())))
            .icon(if pin { "pin" } else { "pin-off" })
            .shortcut(Shortcut::key(egui::Key::Period).ctrl())
            .keep_open(),
    );
    let (paste_id, copy_id, delete_id) = (id.clone(), id.clone(), id);
    vec![
        vec![
            Action::new("paste", paste_title(app), move |host| {
                let h = history()?;
                clipboard::write_entry(h, &h.get(&paste_id).ok_or("entry no longer exists")?)?;
                host.hide(true);
                std::thread::spawn(|| {
                    std::thread::sleep(std::time::Duration::from_millis(80));
                    clipboard::send_paste();
                });
                Ok(())
            })
            .icon("clipboard-paste"),
            Action::new("copy", "Copy to Clipboard", move |host| {
                let h = history()?;
                clipboard::write_entry(h, &h.get(&copy_id).ok_or("entry no longer exists")?)?;
                host.hud(Ok("Copied Entry to Clipboard".into()));
                Ok(())
            })
            .icon("clipboard")
            .shortcut(Shortcut::key(egui::Key::Enter).ctrl()),
        ],
        secondary,
        vec![
            Action::new("delete", "Delete Entry", changed("Deleted Entry", move |h| h.delete(&delete_id))).icon("trash-2").destructive().keep_open().shortcut(Shortcut::key(egui::Key::D).ctrl()),
            Action::new("delete-recent", "Delete Recent Entries…", |_| Ok(()))
                .icon("clock")
                .destructive()
                .shortcut(Shortcut::key(egui::Key::D).ctrl().shift())
                .submenu(|| {
                    [("delete-5", "Last 5 Minutes", 5), ("delete-15", "Last 15 Minutes", 15), ("delete-30", "Last 30 Minutes", 30), ("delete-60", "Last Hour", 60), ("delete-1440", "Last 24 Hours", 1440)]
                        .into_iter()
                        .map(|(id, title, minutes)| Action::new(id, title, changed("Deleted Recent Entries", move |h| h.delete_since(crate::store::now_ms().saturating_sub(minutes * 60_000)))).keep_open())
                        .collect()
                }),
            Action::new("delete-all", "Delete All Entries", changed("Deleted All Entries", |h| h.delete_all()))
                .icon("trash-2")
                .destructive()
                .keep_open()
                .shortcut(Shortcut::key(egui::Key::D).ctrl().alt())
                .confirm("Delete All Entries?", "Every entry except pinned ones will be removed from Clipboard History. This can't be undone.", "Delete All Entries"),
        ],
    ]
}

fn paste_next(host: &mut Host) -> Result<(), String> {
    let h = history()?;
    let recent = h.recent();
    let now = crate::store::now_ms();
    let next = SEQUENCE.lock().unwrap_or_else(|e| e.into_inner()).get_or_insert_with(Default::default).next(&recent, now, SEQUENCE_TIMEOUT_MS);
    let Some(entry) = next else {
        host.hud(Err("No clipboard item found to paste.".into()));
        return Ok(());
    };
    clipboard::suppress_for(now, 2_000);
    match &entry.content {
        Content::Text { text } => clipboard::write_text(&format!("{text}\n"))?,
        _ => clipboard::write_entry(h, &entry)?,
    }
    host.hide(true);
    let top = recent.into_iter().next();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(80));
        clipboard::send_paste();
        std::thread::sleep(std::time::Duration::from_millis(500));
        if let (Some(top), Some(h)) = (top, HISTORY.get()) {
            clipboard::suppress_for(crate::store::now_ms(), 1_000);
            let _ = clipboard::write_entry(h, &top);
        }
    });
    Ok(())
}

pub struct ClipboardHistory;

impl Extension for ClipboardHistory {
    fn id(&self) -> &'static str {
        "clipboard-history"
    }

    fn commands(&self) -> Vec<Command> {
        let mut open = command(self.id(), "history", "Clipboard History", ICON, "Command", "Open Command", |_| Ok(()));
        open.keywords = ["clipboard", "copy", "paste", "history", "histórico", "área de transferência", "colar"].map(String::from).to_vec();
        open.view = Some(Rc::new(|ctx| Box::new(HistoryView::new(ctx))));
        let mut next = command(self.id(), "paste-sequentially", "Paste Sequentially", Icon::Tile("list-ordered", Tone::Red), "Command", "Run Command", paste_next);
        next.keywords = ["clipboard", "paste", "sequence", "next", "colar", "sequência", "próximo"].map(String::from).to_vec();
        let mut reset = command(self.id(), "reset-paste-sequence", "Reset Paste Sequence", Icon::Tile("rotate-ccw", Tone::Red), "Command", "Run Command", |host| {
            if let Some(sequence) = SEQUENCE.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
                sequence.reset();
            }
            host.hud(Ok("Paste sequence reset".into()));
            Ok(())
        });
        reset.keywords = ["clipboard", "paste", "sequence", "colar", "sequência", "redefinir"].map(String::from).to_vec();
        let mut commands = vec![open, next, reset];
        for cmd in &mut commands {
            cmd.subtitle = Some("Clipboard History".into());
        }
        commands
    }
}

struct HistoryView {
    entries: Vec<Entry>,
    version: u64,
    filter: usize,
    app: Option<String>,
    cache: Option<((u64, usize, i64), Vec<Section>)>,
}

impl HistoryView {
    fn new(_ctx: &egui::Context) -> Self {
        Self { entries: Vec::new(), version: u64::MAX, filter: 0, app: frontmost_app(), cache: None }
    }

    fn reload(&mut self) {
        let version = VERSION.load(Ordering::SeqCst);
        if version != self.version {
            self.version = version;
            self.entries = HISTORY.get().map(History::list).unwrap_or_default();
        }
    }
}

fn entry_icon(entry: &Entry) -> Icon {
    match (&entry.content, kind(entry)) {
        (Content::Image { .. }, _) => Icon::Image(format!("clipimg://{}", entry.id)),
        (Content::Files { paths }, _) => paths.first().map_or(Icon::Boxed("file"), |p| Icon::File(p.clone())),
        (Content::Text { text }, Kind::Color) => Icon::Swatch(color::parse(text.trim()).map_or(egui::Color32::TRANSPARENT, |c| crate::extensions::calculator::swatch(&c.rgba))),
        (_, Kind::Link) => Icon::Boxed("link"),
        (_, Kind::Email) => Icon::Boxed("at-sign"),
        _ => Icon::Boxed("file-text"),
    }
}

fn when(ms: u64) -> String {
    icu::format_date(ms as f64, "en-US", Style::Medium, Style::Short, None)
}

impl View for HistoryView {
    fn placeholder(&self) -> String {
        "Type to filter entries…".into()
    }

    fn footer(&self) -> (String, Icon) {
        ("Clipboard History".into(), ICON)
    }

    fn empty(&self) -> Empty {
        Empty { title: if self.filter == 0 { "Copy text, images or files and they will show up here.".into() } else { "No entries of this type.".into() }, description: None }
    }

    fn dropdown(&self) -> Option<Dropdown> {
        Some(Dropdown { label: "Filter by Type", options: FILTERS.iter().map(|(l, g)| (l.to_string(), Icon::Glyph(g))).collect(), selected: self.filter })
    }

    fn select(&mut self, index: usize) {
        self.filter = index;
    }

    fn has_detail(&self) -> bool {
        true
    }

    fn detail(&mut self, ui: &mut Ui, rect: Rect, id: &str) {
        let Some(entry) = self.entries.iter().find(|e| e.id == id) else { return };
        let kind = kind(entry);
        let mut rows: Vec<(String, String)> = Vec::new();
        if !entry.source_name.is_empty() {
            rows.push(("Source".into(), entry.source_name.clone()));
        }
        rows.push(("Type".into(), kind.label().into()));
        match &entry.content {
            Content::Text { text } => {
                rows.push(("Characters".into(), crate::extensions::calculator::math::group(text.chars().count() as f64, 0)));
                rows.push(("Words".into(), crate::extensions::calculator::math::group(word_count(text) as f64, 0)));
            }
            Content::Image { width, height, bytes, .. } => {
                rows.push(("Dimensions".into(), format!("{width}×{height}")));
                rows.push(("Image size".into(), format_bytes(*bytes)));
            }
            Content::Files { paths } => rows.push(if paths.len() == 1 { ("Path".into(), paths[0].clone()) } else { ("Files".into(), paths.len().to_string()) }),
        }
        if entry.copy_count == 1 {
            rows.push(("Copied".into(), when(entry.created_at)));
        } else {
            rows.push(("Times copied".into(), entry.copy_count.to_string()));
            rows.push(("Last copied".into(), when(entry.updated_at)));
            rows.push(("First copied".into(), when(entry.created_at)));
        }
        let info = root::metadata(ui, rect, &rows);
        let preview = Rect::from_min_max(rect.min, pos2(rect.right(), info.top())).shrink(16.0);
        match (&entry.content, kind) {
            (Content::Image { .. }, _) => {
                egui::Image::new(format!("clipimg://{}", entry.id)).max_size(preview.size()).paint_at(ui, Rect::from_center_size(preview.center(), preview.size()));
            }
            (Content::Text { text }, Kind::Color) => {
                let swatch = Rect::from_center_size(preview.center(), vec2(128.0, 128.0));
                let fill = color::parse(text.trim()).map_or(egui::Color32::TRANSPARENT, |c| crate::extensions::calculator::swatch(&c.rgba));
                ui.painter().rect_filled(swatch, 12.0, fill);
                ui.painter().rect_stroke(swatch, 12.0, egui::Stroke::new(1.0, theme::KEY_BORDER()), egui::StrokeKind::Inside);
            }
            (Content::Files { paths }, _) => {
                let mut y = preview.top();
                for path in paths {
                    let galley = root::wrapped(ui, path, egui::FontId::monospace(13.0), theme::FG(), preview.width() - 24.0);
                    let height = galley.size().y;
                    let icon = Rect::from_min_size(pos2(preview.left(), y + (height - 16.0).max(0.0) / 2.0), vec2(16.0, 16.0));
                    crate::icons::command_icon(&mut ui.new_child(egui::UiBuilder::new().max_rect(icon)), &Icon::File(path.clone()), path, 16.0);
                    ui.painter().galley(pos2(preview.left() + 24.0, y), galley, theme::FG());
                    y += height + 8.0;
                }
            }
            (Content::Text { text }, _) => {
                let mut child = ui.new_child(egui::UiBuilder::new().max_rect(preview));
                egui::ScrollArea::vertical().id_salt(("clip-preview", &entry.id)).auto_shrink(false).show(&mut child, |ui| {
                    let galley = root::wrapped(ui, text, egui::FontId::monospace(13.0), theme::FG(), preview.width());
                    let (rect, _) = ui.allocate_exact_size(galley.size(), egui::Sense::hover());
                    ui.painter().galley(rect.min, galley, theme::FG());
                });
            }
        }
    }

    fn sections(&mut self) -> Vec<Section> {
        self.reload();
        let now = crate::store::now_ms();
        let offset = icu::local_offset_minutes(now as f64) as i64 * 60_000;
        let key = (self.version, self.filter, local_day(now, offset));
        if let Some((cached, sections)) = &self.cache {
            if *cached == key {
                return sections.clone();
            }
        }
        let entries: Vec<Entry> = self.entries.iter().filter(|e| matches(e, self.filter)).cloned().collect();
        let sections: Vec<Section> = group(entries, |ms| day_title(ms, now, offset))
            .into_iter()
            .map(|(heading, entries)| Section {
                title: heading,
                items: entries
                    .iter()
                    .map(|entry| Item {
                        id: entry.id.clone(),
                        title: Some(title(entry)).filter(|t| !t.is_empty()).unwrap_or_else(|| "Empty Text".into()),
                        subtitle: None,
                        icon: entry_icon(entry),
                        accessories: Vec::new(),
                        keywords: match &entry.content {
                            Content::Text { text } => vec![text.clone()],
                            Content::Files { paths } => paths.clone(),
                            Content::Image { .. } => Vec::new(),
                        },
                        actions: entry_actions(entry, &self.app),
                    })
                    .collect(),
            })
            .collect();
        self.cache = Some((key, sections.clone()));
        sections
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(t: &str) -> Entry {
        Entry { id: "1".into(), content: Content::Text { text: t.into() }, source_name: String::new(), source_path: String::new(), copy_count: 1, created_at: 0, updated_at: 0, pinned: None }
    }

    #[test]
    fn classifies_text_entries() {
        assert_eq!(kind(&text("https://raycast.com/x")), Kind::Link);
        assert_eq!(kind(&text("me@example.com")), Kind::Email);
        assert_eq!(kind(&text("#FF6363")), Kind::Color);
        assert_eq!(kind(&text("Built-in features")), Kind::Text);
        let image = Entry { content: Content::Image { width: 2, height: 2, bytes: 0, hash: 0 }, ..text("") };
        assert_eq!(kind(&image), Kind::Image);
        assert!(matches(&text("me@example.com"), 4));
        assert!(matches(&text("#fff"), 1));
        assert!(!matches(&Entry { content: Content::Files { paths: vec![r"C:\a.txt".into()] }, ..text("") }, 1));
    }

    #[test]
    fn titles_entries_by_content() {
        assert_eq!(title(&text("\n  first line  \nsecond")), "first line");
        assert_eq!(title(&Entry { content: Content::Image { width: 340, height: 189, bytes: 0, hash: 0 }, ..text("") }), "Image (340×189)");
        assert_eq!(title(&Entry { content: Content::Files { paths: vec![r"C:\Users\gabs\notes.txt".into()] }, ..text("") }), "notes.txt");
        assert_eq!(title(&Entry { content: Content::Files { paths: vec!["a".into(), "b".into()] }, ..text("") }), "2 Files");
        assert_eq!(word_count(" and other  things "), 3);
        assert_eq!(format_bytes(1536), "1.5 KB");
    }

    #[test]
    fn sections_are_rebuilt_only_when_their_inputs_change() {
        let mut view = HistoryView { entries: vec![text("a")], version: VERSION.load(Ordering::SeqCst), filter: 0, app: None, cache: None };
        let ids = |view: &mut HistoryView| view.sections().into_iter().flat_map(|s| s.items).map(|i| i.id).collect::<Vec<_>>();
        assert_eq!(ids(&mut view), ["1"]);
        view.entries.push(Entry { id: "2".into(), ..text("b") });
        assert_eq!(ids(&mut view), ["1"]);
        view.filter = 1;
        assert_eq!(ids(&mut view), ["1", "2"]);
    }

    #[test]
    fn groups_pinned_entries_first_then_by_day() {
        let at = |d: i64, h: i64| (crate::extensions::calculator::history::days_from_civil(2026, 10, d) * DAY + h * 3_600_000) as u64;
        let now = at(18, 12);
        let entry = |id: &str, updated_at: u64, pinned: Option<u64>| Entry { id: id.into(), updated_at, pinned, ..text("x") };
        let groups = group(vec![entry("p", at(14, 9), Some(1)), entry("a", now, None), entry("b", at(17, 23), None), entry("c", at(14, 9), None)], |ms| day_title(ms, now, 0));
        let titles: Vec<(String, Vec<String>)> = groups.into_iter().map(|(t, e)| (t, e.into_iter().map(|e| e.id).collect())).collect();
        let expected = [("Pinned", vec!["p"]), ("Today", vec!["a"]), ("Yesterday", vec!["b"]), ("October 14", vec!["c"])];
        assert_eq!(titles, expected.map(|(t, ids)| (t.to_string(), ids.into_iter().map(String::from).collect::<Vec<_>>())));
    }
}
