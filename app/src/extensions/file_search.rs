use super::clipboard_history::format_bytes;
use super::command;
use crate::extension::{Action, Command, Dropdown, Empty, Extension, Host, Icon, Item, Load, Section, Shortcut, Tone, View};
use crate::file_search::{FileHit, FileInfo, Results};
use crate::icu::{self, Style};
use crate::{root, theme};
use egui::{pos2, vec2, Key, Rect, Ui};
use regex::Regex;
use std::cell::Cell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, LazyLock};

const ICON: Icon = Icon::Tile("file-search", Tone::Blue);
const KINDS: [(&str, &str, &str); 10] = [
    ("allTypes", "All Types", "list"),
    ("directory", "Directory", "folder"),
    ("file", "File", "file"),
    ("archive", "Archive", "archive"),
    ("document", "Document", "file-text"),
    ("image", "Image", "image"),
    ("video", "Video", "video"),
    ("audio", "Audio", "music"),
    ("text", "Text", "type"),
    ("other", "Other", "ellipsis"),
];
const TEXT_PREVIEW_LIMIT: u64 = 1_000_000;
static TEXT_PREVIEW: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)\.(txt|md|markdown|log|csv|tsv|json|jsonc|xml|ya?ml|toml|ini|cfg|conf|html?|css|scss|[cm]?jsx?|tsx?|py|rs|go|java|kt|c|h|cpp|hpp|cs|rb|php|swift|sh|ps1|bat|cmd|sql|lua|dart|vue)$").unwrap());
static VERSION: AtomicU64 = AtomicU64::new(0);
static SHOW_DETAILS: AtomicBool = AtomicBool::new(true);

pub fn parent_folder(path: &str) -> &str {
    match path.rfind('\\') {
        Some(at) if at > 0 => &path[..at],
        _ => path,
    }
}

pub fn short_location(path: &str, home: &str) -> String {
    let parent = parent_folder(path);
    if home.is_empty() {
        return parent.to_owned();
    }
    let (lower, home_lower) = (parent.to_lowercase(), home.to_lowercase());
    if lower == home_lower {
        return "~".into();
    }
    if lower.starts_with(&format!("{home_lower}\\")) {
        return format!("~{}", &parent[home.len()..]);
    }
    parent.to_owned()
}

fn when(ms: Option<u64>) -> String {
    ms.map_or_else(|| "—".into(), |ms| icu::format_date(ms as f64, "en-US", Style::Medium, Style::Short, None))
}

fn file_actions(file: &FileHit, dirty: Rc<Cell<bool>>, with_details: bool) -> Vec<Vec<Action>> {
    let path = file.path.clone();
    let name = file.name.clone();
    let (open, show, copy_file, copy_path, trash) = (path.clone(), path.clone(), path.clone(), format!("\"{path}\""), path.clone());
    let mut first = vec![
        Action::new("open", "Open", move |_| crate::apps::launch(&open, false)).icon("external-link"),
        Action::new("show-in-explorer", "Show in File Explorer", move |_| crate::apps::show_in_explorer(&show)).icon("folder").shortcut(Shortcut::key(Key::Enter).ctrl()),
    ];
    if with_details {
        first.push(
            Action::new("toggle-details", "Toggle Info Panel", |_| {
                SHOW_DETAILS.fetch_xor(true, Ordering::SeqCst);
                Ok(())
            })
            .icon("panel-right")
            .shortcut(Shortcut::key(Key::I).ctrl().shift())
            .keep_open(),
        );
    }
    let copy_name = name.clone();
    let copies = vec![
        Action::new("copy-file", "Copy File", move |host| {
            crate::clipboard::write_files(&[copy_file.clone()])?;
            host.hud(Ok("Copied File to Clipboard".into()));
            Ok(())
        })
        .icon("clipboard")
        .shortcut(Shortcut::key(Key::C).ctrl().shift()),
        Action::new("copy-name", "Copy Name", move |host| {
            host.copy_and_close(&copy_name, "Copied Name to Clipboard");
            Ok(())
        })
        .icon("clipboard")
        .shortcut(Shortcut::key(Key::C).ctrl().alt()),
        Action::new("copy-path", "Copy Path", move |host| {
            host.copy_and_close(&copy_path, "Copied Path to Clipboard");
            Ok(())
        })
        .icon("clipboard")
        .shortcut(Shortcut::key(Key::C).ctrl().alt().shift()),
    ];
    if !with_details {
        return vec![first, copies];
    }
    let message = format!("Are you sure you want to delete {name}? It will be moved to the Recycle Bin.");
    let (deleted, reindexed) = (dirty.clone(), dirty);
    vec![
        first,
        copies,
        vec![Action::new("delete", "Delete", move |host: &mut Host| {
            crate::file_search::recycle(std::path::Path::new(&trash))?;
            host.toast = Some(crate::extension::Toast::Success("Moved to Recycle Bin".into()));
            deleted.set(true);
            Ok(())
        })
        .icon("trash-2")
        .destructive()
        .keep_open()
        .shortcut(Shortcut::key(Key::D).ctrl())
        .confirm("Delete", &message, "Delete")],
        vec![Action::new("index-files", "Index Files", move |host| {
            let ctx = host.ctx.clone();
            crate::file_search::reindex(move || {
                VERSION.fetch_add(1, Ordering::SeqCst);
                ctx.request_repaint();
            });
            reindexed.set(true);
            Ok(())
        })
        .icon("refresh-cw")
        .keep_open()],
    ]
}

struct Preview {
    path: String,
    info: Load<Arc<(FileInfo, Option<String>)>>,
}

impl Preview {
    fn start(ctx: &egui::Context, path: &str) -> Self {
        let owned = path.to_owned();
        let info = Load::start(ctx, move || {
            let info = crate::file_search::info(&owned)?;
            let text = (!info.dir && TEXT_PREVIEW.is_match(&owned) && info.size.unwrap_or(0) <= TEXT_PREVIEW_LIMIT)
                .then(|| std::fs::read_to_string(&owned).ok())
                .flatten()
                .map(|t| t.chars().take(20_000).collect());
            Ok(Arc::new((info, text)))
        });
        Self { path: path.to_owned(), info }
    }
}

fn paint_detail(ui: &mut Ui, rect: Rect, file: &FileHit, home: &str, preview: &Preview) {
    let mut rows = vec![("Name".to_string(), file.name.clone()), ("Where".to_string(), short_location(&file.path, home))];
    let loaded = preview.info.get();
    if let Some(Ok(data)) = &loaded {
        let info = &data.0;
        rows.push(("Type".into(), info.type_name.clone()));
        if let Some(size) = info.size {
            rows.push(("Size".into(), format_bytes(size)));
        }
        rows.push(("Created".into(), when(info.created)));
        rows.push(("Modified".into(), when(info.modified)));
        rows.push(("Accessed".into(), when(info.accessed)));
    }
    let info = root::metadata(ui, rect, &rows);
    let area = Rect::from_min_max(rect.min, pos2(rect.right(), info.top())).shrink(16.0);
    match loaded {
        Some(Err(_)) => {
            let galley = root::wrapped(ui, "This file is no longer available.", theme::font(13.0), theme::FG_60, area.width());
            ui.painter().galley(area.center() - galley.size() / 2.0, galley, theme::FG_60);
        }
        Some(Ok(data)) if data.1.is_some() => {
            let text = data.1.as_deref().unwrap_or_default();
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(area));
            egui::ScrollArea::vertical().id_salt(("file-preview", &file.path)).auto_shrink(false).show(&mut child, |ui| {
                let galley = root::wrapped(ui, text, egui::FontId::monospace(12.0), theme::FG, area.width());
                let (r, _) = ui.allocate_exact_size(galley.size(), egui::Sense::hover());
                ui.painter().galley(r.min, galley, theme::FG);
            });
        }
        _ => {
            let side = area.width().min(area.height()).min(256.0);
            let uri = format!("appicon://{}t/{}", (256.0 * ui.ctx().pixels_per_point()).round() as u32, file.path);
            egui::Image::new(uri).fit_to_exact_size(vec2(side, side)).maintain_aspect_ratio(true).paint_at(ui, Rect::from_center_size(area.center(), vec2(side, side)));
        }
    }
}

struct SearchFiles {
    ctx: egui::Context,
    query: String,
    kind: usize,
    key: Option<(String, usize, u64)>,
    results: Option<Load<Arc<Results>>>,
    last: Option<Arc<Results>>,
    dirty: Rc<Cell<bool>>,
    preview: Option<Preview>,
}

impl SearchFiles {
    fn new(ctx: &egui::Context) -> Self {
        Self { ctx: ctx.clone(), query: String::new(), kind: 0, key: None, results: None, last: None, dirty: Rc::new(Cell::new(false)), preview: None }
    }

    fn current(&mut self) -> Option<Arc<Results>> {
        if self.dirty.replace(false) {
            VERSION.fetch_add(1, Ordering::SeqCst);
        }
        let key = (self.query.trim().to_owned(), self.kind, VERSION.load(Ordering::SeqCst));
        if self.key.as_ref() != Some(&key) {
            let (query, kind, ctx) = (key.0.clone(), KINDS[self.kind].0, self.ctx.clone());
            self.results = Some(Load::start(&self.ctx, move || {
                Ok(Arc::new(crate::file_search::search(
                    move || {
                        VERSION.fetch_add(1, Ordering::SeqCst);
                        ctx.request_repaint();
                    },
                    &query,
                    kind,
                )))
            }));
            self.key = Some(key);
        }
        if let Some(Some(Ok(results))) = self.results.as_ref().map(Load::get) {
            self.last = Some(results);
        }
        self.last.clone()
    }
}

impl View for SearchFiles {
    fn placeholder(&self) -> String {
        "Search files…".into()
    }

    fn footer(&self) -> (String, Icon) {
        ("Search Files".into(), ICON)
    }

    fn filters(&self) -> bool {
        false
    }

    fn set_query(&mut self, query: &str) {
        self.query = query.to_owned();
    }

    fn loading(&self) -> bool {
        self.last.as_ref().is_none_or(|r| r.building)
    }

    fn empty(&self) -> Empty {
        let indexing = self.last.as_ref().is_some_and(|r| r.first_index) && !self.query.trim().is_empty();
        if indexing {
            Empty { title: "File Search is indexing…".into(), description: Some("Results will appear as soon as your files are indexed.".into()) }
        } else if self.query.trim().is_empty() {
            Empty { title: "No Recent Files".into(), description: None }
        } else {
            Empty { title: "No Files Found".into(), description: None }
        }
    }

    fn dropdown(&self) -> Option<Dropdown> {
        Some(Dropdown { label: "Filter by Type", options: KINDS.iter().map(|(_, label, glyph)| (label.to_string(), Icon::Glyph(glyph))).collect(), selected: self.kind })
    }

    fn select(&mut self, index: usize) {
        self.kind = index;
    }

    fn has_detail(&self) -> bool {
        SHOW_DETAILS.load(Ordering::SeqCst)
    }

    fn detail(&mut self, ui: &mut Ui, rect: Rect, id: &str) {
        let Some(results) = self.last.clone() else { return };
        let Some(file) = results.files.iter().find(|f| f.path == id) else { return };
        if self.preview.as_ref().is_none_or(|p| p.path != file.path) {
            self.preview = Some(Preview::start(&self.ctx, &file.path));
        }
        if let Some(preview) = &self.preview {
            paint_detail(ui, rect, file, &results.home, preview);
        }
    }

    fn sections(&mut self) -> Vec<Section> {
        let Some(results) = self.current() else { return Vec::new() };
        let items = results
            .files
            .iter()
            .map(|file| Item {
                id: file.path.clone(),
                title: file.name.clone(),
                subtitle: Some(short_location(&file.path, &results.home)),
                icon: Icon::File(file.path.clone()),
                accessories: Vec::new(),
                keywords: Vec::new(),
                actions: file_actions(file, self.dirty.clone(), true),
            })
            .collect();
        vec![Section { title: if self.query.trim().is_empty() { "Recently Used".into() } else { String::new() }, items }]
    }
}

struct QuickLook {
    ctx: egui::Context,
    selection: Load<Arc<Vec<String>>>,
    preview: Option<Preview>,
}

fn home_of(path: &str) -> String {
    static HOME: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^[a-z]:\\users\\[^\\]+").unwrap());
    HOME.find(path).map(|m| m.as_str().to_owned()).unwrap_or_default()
}

fn hit(path: &str) -> FileHit {
    FileHit { path: path.to_owned(), name: path.rsplit('\\').next().filter(|n| !n.is_empty()).unwrap_or(path).to_owned(), dir: false }
}

impl View for QuickLook {
    fn placeholder(&self) -> String {
        "Filter selection…".into()
    }

    fn footer(&self) -> (String, Icon) {
        ("Quick Look".into(), ICON)
    }

    fn loading(&self) -> bool {
        self.selection.get().is_none()
    }

    fn empty(&self) -> Empty {
        match self.selection.get() {
            Some(Err(e)) => Empty { title: e, description: Some("Open File Explorer, select a file and run Quick Look again.".into()) },
            _ => Empty { title: "Nothing selected".into(), description: None },
        }
    }

    fn has_detail(&self) -> bool {
        true
    }

    fn detail(&mut self, ui: &mut Ui, rect: Rect, id: &str) {
        if self.preview.as_ref().is_none_or(|p| p.path != id) {
            self.preview = Some(Preview::start(&self.ctx, id));
        }
        if let Some(preview) = &self.preview {
            paint_detail(ui, rect, &hit(id), &home_of(id), preview);
        }
    }

    fn sections(&mut self) -> Vec<Section> {
        let Some(Ok(paths)) = self.selection.get() else { return Vec::new() };
        let dirty = Rc::new(Cell::new(false));
        let items = paths.iter().map(|path| hit(path)).map(|file| Item { id: file.path.clone(), title: file.name.clone(), subtitle: None, icon: Icon::File(file.path.clone()), accessories: Vec::new(), keywords: Vec::new(), actions: file_actions(&file, dirty.clone(), false) }).collect();
        vec![Section { title: "Selection".into(), items }]
    }
}

pub struct FileSearch;

impl Extension for FileSearch {
    fn id(&self) -> &'static str {
        "file-search"
    }

    fn commands(&self) -> Vec<Command> {
        let mut quick = command(self.id(), "quick-look", "Quick Look", Icon::Tile("eye", Tone::Blue), "Command", "Quick Look", |_| Ok(()));
        quick.keywords = ["preview", "explorer", "selection", "peek", "visualizar", "prévia", "seleção"].map(String::from).to_vec();
        quick.view = Some(Rc::new(|ctx| {
            let foreground = crate::launcher::previous_window().0 as isize;
            let selection = Load::start(ctx, move || crate::explorer::selected_files(windows::Win32::Foundation::HWND(foreground as _)).map(Arc::new));
            Box::new(QuickLook { ctx: ctx.clone(), selection, preview: None })
        }));
        let mut search = command(self.id(), "search-files", "Search Files", ICON, "Command", "Open Command", |_| Ok(()));
        search.keywords = ["files", "folders", "find", "explorer", "documents", "arquivos", "buscar arquivos", "pastas"].map(String::from).to_vec();
        search.view = Some(Rc::new(|ctx| Box::new(SearchFiles::new(ctx))));
        let mut commands = vec![quick, search];
        for cmd in &mut commands {
            cmd.subtitle = Some("File Search".into());
        }
        commands
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shortens_locations_under_the_home_folder() {
        assert_eq!(parent_folder(r"C:\Users\ana\Documents\plan.md"), r"C:\Users\ana\Documents");
        assert_eq!(short_location(r"C:\Users\ana\Documents\plan.md", r"C:\Users\ana"), r"~\Documents");
        assert_eq!(short_location(r"C:\Users\ana\plan.md", r"C:\Users\Ana"), "~");
        assert_eq!(short_location(r"C:\Users\anabel\plan.md", r"C:\Users\ana"), r"C:\Users\anabel");
        assert_eq!(short_location(r"D:\work\plan.md", r"C:\Users\ana"), r"D:\work");
    }
}
