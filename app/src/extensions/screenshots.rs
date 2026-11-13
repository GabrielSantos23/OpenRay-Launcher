use super::calendar::local_day;
use super::command;
use crate::capture;
use crate::capture_ui;
use crate::extension::{notify, Action, Command, Empty, Extension, Host, Icon, Item, Load, Section, Shortcut, Toast, Tone, View};
use egui::Key;
use std::rc::Rc;
use std::sync::atomic::Ordering;

const ICON: Icon = Icon::Tile("camera", Tone::Blue);
const TASKS: [(&str, &str, &str, &[&str]); 10] = [
    ("region", "Capture Region", "crop", &["screenshot", "area", "selection", "snip", "window", "região", "seleção", "área", "recorte"]),
    ("fullscreen", "Capture Fullscreen", "monitor-up", &["screenshot", "print screen", "all screens", "tela inteira", "todas as telas"]),
    ("window", "Capture Active Window", "app-window", &["screenshot", "window", "janela ativa"]),
    ("monitor", "Capture Active Monitor", "monitor", &["screenshot", "display", "screen", "monitor", "tela"]),
    ("last-region", "Capture Last Region", "redo-2", &["screenshot", "repeat", "region", "repetir", "última região"]),
    ("color", "Pick Color from Screen", "pipette", &["color picker", "eyedropper", "hex", "cor", "conta-gotas", "seletor de cores"]),
    ("ocr", "Capture Text (OCR)", "scan-text", &["ocr", "text recognition", "extract text", "copy text", "extrair texto", "copiar texto", "reconhecimento de texto"]),
    ("qr", "Scan QR Code", "qr-code", &["qr", "barcode", "decode", "código qr", "qr code", "decodificar"]),
    ("pin", "Pin Region to Screen", "pin", &["pin", "float", "overlay", "sticky", "fixar", "sobreposição"]),
    ("ruler", "Screen Ruler", "ruler", &["measure", "pixels", "ruler", "size", "régua", "medir"]),
];

type Shot = (std::path::PathBuf, u64, u64);

fn folders() -> Vec<std::path::PathBuf> {
    vec![capture::folder(), capture::system_folder()]
}

fn run_task(id: &'static str, ctx: &egui::Context) {
    match id {
        "fullscreen" | "window" | "monitor" | "last-region" => capture_ui::direct(ctx, id),
        _ => capture_ui::start(ctx, id),
    }
}

pub fn section_title(time: i64, now: i64, month: impl Fn(i64) -> String) -> String {
    let (day, today) = (local_day(time), local_day(now));
    match today - day {
        i64::MIN..=0 => "Today".into(),
        1 => "Yesterday".into(),
        2..=6 => "Previous 7 Days".into(),
        _ => month(time),
    }
}

fn copy_image_file(path: &str) -> Result<(), String> {
    if path.to_lowercase().ends_with(".png") {
        crate::clipboard::write_image(&std::fs::read(path).map_err(|e| e.to_string())?)
    } else {
        crate::clipboard::write_files(&[path.to_owned()])
    }
}

fn paste_file(path: &str) -> Result<(), String> {
    copy_image_file(path)?;
    std::thread::spawn(|| {
        std::thread::sleep(std::time::Duration::from_millis(80));
        crate::clipboard::send_paste();
    });
    Ok(())
}

fn copy_text(ctx: egui::Context, path: String) {
    std::thread::spawn(move || {
        let result = capture_ui::file_text(&path).and_then(|text| if text.trim().is_empty() { Err("No text found in this image".to_owned()) } else { crate::clipboard::write_text(&text) });
        notify(&ctx, match result {
            Ok(()) => Toast::Success("Copied Text to Clipboard".into()),
            Err(e) => Toast::Failure(e, String::new()),
        });
    });
}

fn shot_actions(path: &str, name: &str) -> Vec<Vec<Action>> {
    let owned = |s: &str| s.to_owned();
    let (copy, paste, open, show, edit, pin, text, trash, copy_name, copy_path) = (owned(path), owned(path), owned(path), owned(path), owned(path), owned(path), owned(path), owned(path), owned(name), owned(path));
    let mut second = vec![Action::new("edit", "Edit Image", move |_| crate::apps::edit(&edit)).icon("pen-tool").shortcut(Shortcut::key(Key::E).ctrl())];
    if path.to_lowercase().ends_with(".png") {
        second.push(Action::new("pin", "Pin to Screen", move |host: &mut Host| capture_ui::pin_file(&host.ctx, &pin)).icon("pin").shortcut(Shortcut::key(Key::P).ctrl().shift()));
        second.push(
            Action::new("copy-text", "Copy Text (OCR)", move |host: &mut Host| {
                copy_text(host.ctx.clone(), text.clone());
                Ok(())
            })
            .icon("scan-text")
            .shortcut(Shortcut::key(Key::T).ctrl().shift()),
        );
    }
    vec![
        vec![
            Action::new("copy", "Copy to Clipboard", move |host: &mut Host| {
                copy_image_file(&copy)?;
                host.hud(Ok("Copied to Clipboard".into()));
                Ok(())
            })
            .icon("clipboard")
            .shortcut(Shortcut::key(Key::C).ctrl().shift()),
            Action::new("paste", "Paste to Active App", move |_| paste_file(&paste)).icon("clipboard-paste").shortcut(Shortcut::key(Key::Enter).ctrl()),
            Action::new("open", "Open", move |_| crate::apps::launch(&open, false)).icon("external-link").shortcut(Shortcut::key(Key::O).ctrl()),
            Action::new("show-in-explorer", "Show in File Explorer", move |_| crate::apps::show_in_explorer(&show)).icon("folder").shortcut(Shortcut::key(Key::F).ctrl().shift()),
        ],
        second,
        vec![
            Action::new("copy-name", "Copy Name", move |host: &mut Host| {
                host.copy_and_close(&copy_name, "Copied Name to Clipboard");
                Ok(())
            })
            .icon("clipboard")
            .shortcut(Shortcut::key(Key::C).ctrl().alt()),
            Action::new("copy-path", "Copy Path", move |host: &mut Host| {
                host.copy_and_close(&copy_path, "Copied Path to Clipboard");
                Ok(())
            })
            .icon("clipboard")
            .shortcut(Shortcut::key(Key::C).ctrl().alt().shift()),
        ],
        vec![Action::new("delete", "Move to Recycle Bin", move |host: &mut Host| {
            crate::file_search::recycle(std::path::Path::new(&trash))?;
            capture_ui::VERSION.fetch_add(1, Ordering::SeqCst);
            host.toast = Some(Toast::Success("Moved to Recycle Bin".into()));
            Ok(())
        })
        .icon("trash-2")
        .destructive()
        .keep_open()
        .shortcut(Shortcut::key(Key::D).ctrl())
        .confirm("Move to Recycle Bin?", &format!("{name} will be moved to the Recycle Bin."), "Move to Recycle Bin")],
    ]
}

struct SearchScreenshots {
    ctx: egui::Context,
    version: u64,
    load: Load<Vec<Shot>>,
    built: Option<Vec<Section>>,
}

fn list(ctx: &egui::Context) -> Load<Vec<Shot>> {
    Load::start(ctx, || Ok(capture::screenshots(&folders())))
}

fn sections_of(shots: &[Shot]) -> Vec<Section> {
    let (now, locale) = (crate::store::now_ms() as i64, crate::icu::display_locale());
    let mut sections: Vec<Section> = Vec::new();
    for (path, modified, _) in shots {
        let time = *modified as i64;
        let title = section_title(time, now, |t| crate::icu::format_skeleton(t as f64, &locale, "yMMMM", None));
        let (text, name) = (path.to_string_lossy().into_owned(), path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default());
        let item = Item {
            id: text.clone(),
            title: name.clone(),
            subtitle: Some(crate::icu::format_skeleton(time as f64, &locale, "yMMMdjm", None)),
            icon: Icon::Thumbnail(text.clone()),
            accessories: Vec::new(),
            keywords: name.split('_').next().map(String::from).into_iter().collect(),
            actions: shot_actions(&text, &name),
        };
        match sections.last_mut() {
            Some(last) if last.title == title => last.items.push(item),
            _ => sections.push(Section { title, items: vec![item] }),
        }
    }
    sections
}

impl View for SearchScreenshots {
    fn placeholder(&self) -> String {
        "Search screenshots…".into()
    }

    fn grid(&self) -> Option<usize> {
        Some(4)
    }

    fn footer(&self) -> (String, Icon) {
        ("Search Screenshots".into(), ICON)
    }

    fn loading(&self) -> bool {
        self.load.get().is_none()
    }

    fn empty(&self) -> Empty {
        Empty { title: "No Screenshots".into(), description: Some("Screenshots you take with Openray or Win+Print Screen show up here. Press Ctrl+N to capture a region.".into()) }
    }

    fn empty_actions(&self) -> Vec<Vec<Action>> {
        vec![vec![Action::new("capture", "Capture Region", |host: &mut Host| {
            capture_ui::start(&host.ctx, "region");
            Ok(())
        })
        .icon("crop")
        .shortcut(Shortcut::key(Key::N).ctrl())]]
    }

    fn sections(&mut self) -> Vec<Section> {
        let version = capture_ui::VERSION.load(Ordering::SeqCst);
        if version != self.version {
            self.version = version;
            self.load = list(&self.ctx);
            self.built = None;
        }
        if self.built.is_none() {
            if let Some(result) = self.load.get() {
                match result {
                    Ok(shots) => self.built = Some(sections_of(&shots)),
                    Err(e) => {
                        notify(&self.ctx, Toast::Failure("Could not load screenshots".into(), e));
                        self.built = Some(Vec::new());
                    }
                }
            }
        }
        self.built.as_ref().map(|list| list.iter().map(|s| Section { title: s.title.clone(), items: s.items.clone() }).collect()).unwrap_or_default()
    }
}

pub struct Screenshots;

impl Extension for Screenshots {
    fn id(&self) -> &'static str {
        "screenshots"
    }

    fn commands(&self) -> Vec<Command> {
        *capture::CUSTOM_FOLDER.lock().unwrap_or_else(|e| e.into_inner()) = capture::custom_folder(&crate::store::read_json::<serde_json::Value>(&crate::data_dir().join("screenshots.json")));
        let id = self.id();
        let mut search = command(id, "search-screenshots", "Search Screenshots", ICON, "Command", "Open Command", |_| Ok(()));
        search.view = Some(Rc::new(|ctx: &egui::Context| Box::new(SearchScreenshots { ctx: ctx.clone(), version: capture_ui::VERSION.load(Ordering::SeqCst), load: list(ctx), built: None }) as Box<dyn View>));
        search.keywords = ["screenshots", "images", "captures", "capturas de tela", "imagens", "print"].map(String::from).to_vec();
        let mut paste = command(id, "paste-latest", "Paste Latest Screenshot", Icon::Tile("clipboard-paste", Tone::Blue), "Command", "Paste Latest Screenshot", |_| {
            let (latest, _, _) = capture::screenshots(&folders()).into_iter().next().ok_or("No screenshots found")?;
            paste_file(&latest.to_string_lossy())
        });
        paste.keywords = ["screenshot", "paste", "recent", "last", "colar", "recente", "última", "print"].map(String::from).to_vec();
        let mut open = command(id, "open-folder", "Open Screenshots Folder", Icon::Tile("folder-open", Tone::Blue), "Command", "Open Folder", |_| {
            let folder = capture::folder();
            std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
            crate::apps::launch(&folder.to_string_lossy(), false)
        });
        open.keywords = ["screenshots", "folder", "pictures", "pasta", "imagens", "print"].map(String::from).to_vec();
        let mut commands = vec![search, paste, open];
        for (task, title, glyph, keywords) in TASKS {
            let mut cmd = command(id, task, title, Icon::Tile(glyph, Tone::Blue), "Command", "Run Command", move |host: &mut Host| {
                run_task(task, &host.ctx);
                Ok(())
            });
            cmd.keywords = ["capture", "captura", "captura de tela", "print"].iter().chain(keywords).map(|k| k.to_string()).collect();
            commands.push(cmd);
        }
        for cmd in &mut commands {
            cmd.subtitle = Some("Screenshots".into());
        }
        commands
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thumbnails_crop_to_a_centered_square() {
        let uv = crate::icons::cover_uv(egui::vec2(200.0, 100.0));
        assert_eq!((uv.left(), uv.right(), uv.top(), uv.bottom()), (0.25, 0.75, 0.0, 1.0));
    }

    #[test]
    fn groups_by_relative_day_then_month() {
        const DAY: i64 = 86_400_000;
        let now = 1_792_000_000_000;
        let title = |back: i64| section_title(now - back * DAY, now, |_| "Month".to_owned());
        assert_eq!(title(0), "Today");
        assert_eq!(title(1), "Yesterday");
        assert_eq!(title(6), "Previous 7 Days");
        assert_eq!(title(7), "Month");
    }
}
