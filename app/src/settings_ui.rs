use super::{back_button, paint_text, Launcher};
use crate::extension::{Icon, Tone};
use crate::store::{CommandPrefs, EscapeKeyBehavior, Settings, ShowOn};
use crate::{icons, theme};
use egui::{pos2, vec2, Align2, Color32, Key, Rect, Sense, Stroke, Ui, UiBuilder};
use crate::flowkey_host::{self as flowkey, Installed};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_CONTROL, VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT};

pub const LAUNCHER_OWNER: &str = "launcher";
const DEFAULT_HOTKEY: &str = "Alt+Space";
const START_MENU_HOTKEY: &str = "Win";
const SIDEBAR: f32 = 230.0;
const ROW: f32 = 47.0;
const CONTROL: f32 = 24.0;
const SLOT: usize = 1000;
const CARD: Color32 = theme::fg(3);
const SEPARATOR: Color32 = theme::fg(6);
const PAGES: [(&str, &str, &str, &[&str]); 5] = [
    ("general", "General", "settings", &["open at login", "system tray", "hotkey", "start menu"]),
    ("launcher", "Launcher", "rocket", &["show openray on", "screen", "monitor"]),
    ("shortcuts", "Shortcuts", "command", &["alias", "hotkey", "commands"]),
    ("keyboard", "Keyboard", "keyboard", &["escape key", "esc"]),
    ("extensions", "Extensions", "puzzle", &["install", "flowkey", "package", "uninstall"]),
];
const KEY_NAMES: [&str; 25] = [
    "Space", "Enter", "Tab", "`", ".", ",", "/", ";", "-", "=", "[", "]", "\\", "'", "ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight", "Home", "End", "PageUp", "PageDown", "Insert", "Delete", "Backspace",
];

pub struct ExtensionInfo {
    pub id: String,
    pub title: String,
    pub icon: Icon,
    pub commands: Vec<(String, String, Icon)>,
}

enum AskAction {
    Install { path: String, id: String, label: String },
    Uninstall(Arc<Installed>),
}

struct Ask {
    title: String,
    message: String,
    ok: &'static str,
    action: AskAction,
}

pub struct Conflict {
    owner: String,
    hotkey: String,
    name: String,
    overwritable: bool,
}

pub struct SettingsWindow {
    page: String,
    history: Vec<String>,
    nav_filter: String,
    command_filter: String,
    collapsed: HashSet<String>,
    extensions: Vec<ExtensionInfo>,
    generation: u64,
    login: Option<bool>,
    error: Option<String>,
    recording: Option<String>,
    waiting_release: bool,
    conflict: Option<Conflict>,
    focus: Option<crate::extensions::FocusPreferences>,
    dictionary: Option<String>,
    flowkey: Vec<Arc<Installed>>,
    flowkey_failures: Vec<String>,
    flowkey_text: HashMap<String, String>,
    flowkey_status: Option<(String, bool)>,
    ask: Option<Ask>,
}

impl SettingsWindow {
    pub fn open(page: &str) -> Self {
        Self {
            page: page.into(),
            history: Vec::new(),
            nav_filter: String::new(),
            command_filter: String::new(),
            collapsed: HashSet::new(),
            extensions: Vec::new(),
            generation: u64::MAX,
            login: None,
            error: None,
            recording: None,
            waiting_release: false,
            conflict: None,
            focus: None,
            dictionary: None,
            flowkey: Vec::new(),
            flowkey_failures: Vec::new(),
            flowkey_text: HashMap::new(),
            flowkey_status: None,
            ask: None,
        }
    }

    fn navigate(&mut self, page: String) {
        if page != self.page {
            self.history.push(std::mem::replace(&mut self.page, page));
            self.error = None;
            self.flowkey_status = None;
        }
    }
}

pub fn normalize(hotkey: &str) -> String {
    let mut parts: Vec<String> = hotkey.split('+').map(|p| p.trim().to_lowercase()).filter(|p| !p.is_empty()).collect();
    let key = parts.pop().unwrap_or_default();
    parts.sort();
    parts.push(key);
    parts.join("+")
}

pub fn same_hotkey(a: &str, b: &str) -> bool {
    !a.trim().is_empty() && !b.trim().is_empty() && normalize(a) == normalize(b)
}

pub fn hotkey_owner(settings: &Settings, hotkey: &str, owner: &str) -> Option<String> {
    if owner != LAUNCHER_OWNER && same_hotkey(&settings.hotkey, hotkey) {
        return Some(LAUNCHER_OWNER.into());
    }
    let mut keys: Vec<&String> = settings.commands.iter().filter(|(key, p)| *key != owner && same_hotkey(&p.hotkey, hotkey)).map(|(k, _)| k).collect();
    keys.sort();
    keys.first().map(|k| k.to_string())
}

pub fn with_hotkey(settings: &mut Settings, owner: &str, hotkey: &str, overwrite: bool) {
    if overwrite {
        for (key, prefs) in settings.commands.iter_mut() {
            if key != owner && same_hotkey(&prefs.hotkey, hotkey) {
                prefs.hotkey.clear();
            }
        }
    }
    if owner == LAUNCHER_OWNER {
        settings.hotkey = hotkey.into();
    } else {
        settings.commands.entry(owner.into()).or_default().hotkey = hotkey.into();
    }
}

fn key_table() -> &'static Vec<(u32, String)> {
    static TABLE: std::sync::OnceLock<Vec<(u32, String)>> = std::sync::OnceLock::new();
    TABLE.get_or_init(|| {
        let mut names: Vec<String> = KEY_NAMES.iter().map(|s| s.to_string()).collect();
        names.extend(('A'..='Z').map(String::from));
        names.extend(('0'..='9').map(String::from));
        names.extend((1..=24).map(|n| format!("F{n}")));
        names.extend((0..=9).map(|n| format!("Numpad{n}")));
        names.extend(["PrintScreen", "Pause", "ScrollLock"].map(String::from));
        names.into_iter().filter_map(|name| crate::hotkey::parse(&format!("Ctrl+{name}")).ok().map(|h| (h.vk, name))).collect()
    })
}

fn down(vk: u32) -> bool {
    unsafe { GetAsyncKeyState(vk as i32) as u16 & 0x8000 != 0 }
}

fn pressed_hotkey(last_down: u32) -> Option<Result<String, ()>> {
    // Print Screen never shows as held in the async key state, so the keyboard hook's last key-down stands in for it.
    let (vk, name) = key_table().iter().find(|(vk, _)| down(*vk) || (*vk == 0x2C && last_down == 0x2C))?;
    let modifiers: Vec<&str> = [(VK_CONTROL.0, "Ctrl"), (VK_MENU.0, "Alt"), (VK_SHIFT.0, "Shift")].iter().filter(|(vk, _)| down(u32::from(*vk))).map(|(_, n)| *n).collect();
    let win = down(u32::from(VK_LWIN.0)) || down(u32::from(VK_RWIN.0));
    let mut parts = modifiers;
    if win {
        parts.push("Win");
    }
    if parts.is_empty() && !crate::hotkey::standalone(*vk) {
        return Some(Err(()));
    }
    parts.push(name);
    Some(Ok(parts.join("+")))
}

fn any_key_down() -> bool {
    key_table().iter().any(|(vk, _)| down(*vk))
}

pub fn hotkey_labels(hotkey: &str) -> Vec<String> {
    hotkey
        .split('+')
        .map(|part| match part {
            "ArrowUp" => "↑".into(),
            "ArrowDown" => "↓".into(),
            "ArrowLeft" => "←".into(),
            "ArrowRight" => "→".into(),
            "Backspace" => "⌫".into(),
            "PageUp" => "PgUp".into(),
            "PageDown" => "PgDn".into(),
            "Delete" => "Del".into(),
            "Insert" => "Ins".into(),
            "PrintScreen" => "PrtSc".into(),
            "ScrollLock" => "ScrLk".into(),
            other => other.into(),
        })
        .collect()
}

fn keycap_row(ui: &Ui, left: f32, center_y: f32, labels: &[String]) -> f32 {
    let mut x = left;
    for label in labels {
        let galley = super::text_galley(ui, label, theme::weighted(12.0, theme::MEDIUM), theme::FG, 200.0);
        let width = (galley.size().x + 12.0).max(CONTROL);
        let rect = Rect::from_min_size(pos2(x, center_y - CONTROL / 2.0), vec2(width, CONTROL));
        ui.painter().rect_filled(rect, 4.0, theme::CONTROL);
        ui.painter().galley(rect.center() - galley.size() / 2.0, galley, theme::FG);
        x = rect.right() + 4.0;
    }
    x
}

fn text_button(ui: &mut Ui, right: f32, center_y: f32, label: &str, id: &str, enabled: bool) -> (Rect, bool) {
    let font = theme::weighted(13.0, theme::MEDIUM);
    let width = ui.fonts_mut(|f| f.layout_no_wrap(label.to_owned(), font.clone(), theme::FG)).size().x + 20.0;
    let rect = Rect::from_min_size(pos2(right - width, center_y - CONTROL / 2.0), vec2(width, CONTROL));
    let response = ui.interact(rect, egui::Id::unique(("settings-button", id)), if enabled { Sense::click() } else { Sense::hover() });
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, enabled, label));
    ui.painter().rect_filled(rect, 5.0, if response.hovered() && enabled { theme::CONTROL_HOVER } else { theme::CONTROL });
    if response.has_focus() {
        ui.painter().rect_stroke(rect.expand(2.0), 7.0, Stroke::new(1.0, theme::ACCENT), egui::StrokeKind::Outside);
    }
    paint_text(ui, rect.center(), Align2::CENTER_CENTER, label, font, if enabled { theme::FG } else { theme::FG_40 }, width);
    (rect, enabled && response.clicked())
}

fn switch(ui: &mut Ui, right: f32, center_y: f32, on: bool, label: &str) -> bool {
    let rect = Rect::from_min_size(pos2(right - 28.0, center_y - 8.0), vec2(28.0, 16.0));
    let response = ui.interact(rect, egui::Id::unique(("settings-switch", label)), Sense::click());
    response.widget_info(|| egui::WidgetInfo::selected(egui::Role::Switch, true, on, label));
    if on {
        ui.painter().rect_filled(rect, 8.0, theme::ACCENT);
        ui.painter().circle_filled(pos2(rect.right() - 8.0, rect.center().y), 5.0, Color32::WHITE);
    } else {
        ui.painter().rect_stroke(rect, 8.0, Stroke::new(1.0, theme::fg(25)), egui::StrokeKind::Inside);
        ui.painter().circle_filled(pos2(rect.left() + 7.0, rect.center().y), 4.0, theme::FG.gamma_multiply(0.9));
    }
    if response.has_focus() {
        ui.painter().rect_stroke(rect.expand(2.0), 10.0, Stroke::new(1.0, theme::ACCENT), egui::StrokeKind::Outside);
    }
    response.clicked() || (response.has_focus() && ui.input(|i| i.key_pressed(Key::Space)))
}

fn checkbox(ui: &mut Ui, rect: Rect, on: bool, label: &str) -> bool {
    let response = ui.interact(rect.expand(4.0), egui::Id::unique(("settings-check", label)), Sense::click());
    response.widget_info(|| egui::WidgetInfo::selected(egui::Role::CheckBox, true, on, label));
    if on {
        ui.painter().rect_filled(rect, 4.0, theme::CONTROL_HOVER);
        icons::paint_glyph(ui, "check", rect.shrink(2.0), theme::FG);
    } else {
        ui.painter().rect_stroke(rect, 4.0, Stroke::new(1.0, theme::fg(25)), egui::StrokeKind::Inside);
    }
    if response.has_focus() {
        ui.painter().rect_stroke(rect.expand(2.0), 6.0, Stroke::new(1.0, theme::ACCENT), egui::StrokeKind::Outside);
    }
    response.clicked() || (response.has_focus() && ui.input(|i| i.key_pressed(Key::Space)))
}

struct Card {
    left: f32,
    width: f32,
    top: f32,
    y: f32,
}

impl Card {
    fn row(&mut self, ui: &Ui, title: &str, description: Option<&str>, reserve: f32) -> Rect {
        let text_width = self.width - 24.0 - reserve - 16.0;
        let galley = description.map(|text| super::wrapped(ui, text, theme::font(12.0), theme::FG_60, text_width));
        let height = galley.as_ref().map_or(ROW, |g| (16.0 + 16.0 + 4.0 + g.size().y + 14.0).max(ROW));
        let row = Rect::from_min_size(pos2(self.left, self.y), vec2(self.width, height));
        if self.y > self.top {
            ui.painter().hline(row.x_range(), row.top(), Stroke::new(1.0, SEPARATOR));
        }
        match galley {
            Some(galley) => {
                let top = row.center().y - (16.0 + 4.0 + galley.size().y) / 2.0;
                paint_text(ui, pos2(row.left() + 12.0, top), Align2::LEFT_TOP, title, theme::weighted(13.0, theme::MEDIUM), theme::FG, text_width);
                ui.painter().galley(pos2(row.left() + 12.0, top + 20.0), galley, theme::FG_60);
            }
            None => {
                paint_text(ui, pos2(row.left() + 12.0, row.center().y), Align2::LEFT_CENTER, title, theme::weighted(13.0, theme::MEDIUM), theme::FG, text_width);
            }
        }
        self.y += height;
        row
    }
}

fn card(ui: &mut Ui, y: &mut f32, left: f32, width: f32, body: impl FnOnce(&mut Ui, &mut Card)) {
    let background = ui.painter().add(egui::Shape::Noop);
    let mut c = Card { left, width, top: *y, y: *y };
    body(ui, &mut c);
    let rect = Rect::from_min_max(pos2(left, c.top), pos2(left + width, c.y));
    ui.painter().set(background, egui::epaint::RectShape::filled(rect, 8.0, CARD));
    *y = rect.bottom() + 16.0;
}

impl Launcher {
    pub(super) fn extension_infos(&self) -> Vec<ExtensionInfo> {
        self.extensions
            .iter()
            .map(|extension| {
                let commands = extension.commands();
                let id = extension.id().to_owned();
                let title = match id.as_str() {
                    "applications" => "Applications".to_owned(),
                    "openray-core" => "Openray".to_owned(),
                    _ => commands.iter().find_map(|c| c.subtitle.clone()).unwrap_or_else(|| id.replace('-', " ")),
                };
                let icon = if let Some(icon) = extension.icon() {
                    icon
                } else if id == "applications" { Icon::Tile("layout-grid", Tone::Blue) } else { commands.first().map(|c| c.icon.clone()).unwrap_or(Icon::Tile("puzzle", Tone::Slate)) };
                ExtensionInfo { id, title, icon, commands: commands.into_iter().map(|c| (c.key, c.title, c.icon)).collect() }
            })
            .collect()
    }

    pub(super) fn save_settings(&mut self) {
        if let Err(e) = self.store.set_settings(self.settings.clone()) {
            if let Some(window) = self.settings_window.as_mut() {
                window.error = Some(format!("Could not save settings: {e}"));
            }
        }
        crate::extensions::GENERATION.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        crate::TRAY_VISIBLE.store(self.settings.show_in_tray, std::sync::atomic::Ordering::SeqCst);
    }

    pub(super) fn show_settings(&mut self, ctx: &egui::Context) {
        let Some(mut window) = self.settings_window.take() else { return };
        let generation = crate::extensions::GENERATION.load(std::sync::atomic::Ordering::SeqCst);
        if window.generation != generation {
            window.generation = generation;
            window.extensions = self.extension_infos();
            (window.flowkey, window.flowkey_failures) = flowkey::list();
            window.flowkey_text.clear();
        }
        let builder = egui::ViewportBuilder::default().with_title("Openray Settings").with_inner_size([1000.0, 680.0]).with_min_inner_size([800.0, 500.0]);
        let close = ctx.show_viewport_immediate(egui::ViewportId::from_hash_of("settings"), builder, |ui, _| {
            self.settings_ui(ui, &mut window);
            ui.input(|i| i.viewport().close_requested())
        });
        if close {
            if window.recording.is_some() {
                crate::hotkey::suspend(false);
            }
        } else {
            self.settings_window = Some(window);
        }
    }

    fn settings_ui(&mut self, ui: &mut Ui, w: &mut SettingsWindow) {
        let ctx = ui.ctx().clone();
        let full = ui.max_rect();
        ui.painter().rect_filled(full, 0.0, theme::BG);
        if self.dropdown.as_ref().is_some_and(|d| d.slot >= SLOT) {
            self.dropdown_keys(ui);
        } else if w.recording.is_none() && w.conflict.is_none() && w.ask.is_none() && ui.input(|i| i.key_pressed(Key::Escape)) && self.settings.escape_key_closes_window {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        self.record_hotkey(ui, w);
        let sidebar = Rect::from_min_max(full.min, pos2(full.left() + SIDEBAR, full.bottom()));
        ui.painter().vline(sidebar.right() - 0.5, sidebar.y_range(), Stroke::new(1.0, theme::SEPARATOR));
        self.sidebar(ui, sidebar, w);
        let content = Rect::from_min_max(pos2(sidebar.right(), full.top()), full.max);
        let header = Rect::from_min_size(content.min, vec2(content.width(), 44.0));
        let back = Rect::from_center_size(pos2(header.left() + 16.0 + 11.0, header.center().y), vec2(22.0, 22.0));
        if !w.history.is_empty() && back_button(ui, back) {
            w.page = w.history.pop().unwrap_or_default();
        }
        let title = self.page_title(w);
        let title_left = if w.history.is_empty() { header.left() + 16.0 } else { back.right() + 12.0 };
        paint_text(ui, pos2(title_left, header.center().y), Align2::LEFT_CENTER, &title, theme::weighted(15.0, theme::SEMIBOLD), theme::FG, 400.0);
        if w.page == "shortcuts" {
            let field = Rect::from_min_size(pos2(header.right() - 16.0 - 220.0, header.center().y - 14.0), vec2(220.0, 28.0));
            ui.painter().rect(field, 6.0, theme::fg(5), Stroke::new(1.0, theme::KEY_BORDER), egui::StrokeKind::Inside);
            let hint = egui::RichText::new("Search commands...").color(theme::FG_40).font(theme::font(13.0));
            let inner = field.shrink2(vec2(10.0, 6.0));
            let response = ui.put(inner, egui::TextEdit::singleline(&mut w.command_filter).id(egui::Id::unique("settings-command-filter")).frame(egui::Frame::NONE).margin(egui::Margin::ZERO).font(theme::font(13.0)).text_color(theme::FG).hint_text(hint).desired_width(inner.width()));
            response.widget_info(|| egui::WidgetInfo::text_edit(true, "Search commands", &w.command_filter, ""));
        }
        let body = Rect::from_min_max(pos2(content.left(), header.bottom()), content.max);
        let mut body_ui = ui.new_child(UiBuilder::new().max_rect(body));
        egui::ScrollArea::vertical().id_salt(("settings-body", w.page.clone())).auto_shrink(false).show(&mut body_ui, |ui| {
            let origin = ui.max_rect().min;
            let (left, width) = (origin.x + 16.0, ui.available_width() - 32.0);
            let mut y = origin.y + 4.0;
            if let Some(error) = w.error.clone() {
                let galley = super::wrapped(ui, &error, theme::font(12.0), theme::RED, width);
                let h = galley.size().y;
                ui.painter().galley(pos2(left, y), galley, theme::RED);
                y += h + 12.0;
            }
            match w.page.as_str() {
                "general" => self.general_page(ui, w, left, width, &mut y),
                "launcher" => self.launcher_page(ui, left, width, &mut y),
                "keyboard" => self.keyboard_page(ui, left, width, &mut y),
                "extensions" => flowkey_install_page(ui, w, left, width, &mut y),
                "shortcuts" => {
                    let term = w.command_filter.trim().to_lowercase();
                    for index in 0..w.extensions.len() {
                        self.extension_table(ui, w, index, &term, left, width, &mut y);
                    }
                }
                page => {
                    if let Some(index) = w.extensions.iter().position(|e| format!("extension:{}", e.id) == page) {
                        let id = w.extensions[index].id.clone();
                        match id.as_str() {
                            "focus" => focus_settings(self, ui, w, left, width, &mut y),
                            "screenshots" => screenshot_settings(ui, w, left, width, &mut y),
                            "dictionary" => dictionary_settings(self, ui, w, left, width, &mut y),
                            other if other.starts_with("flowkey.") => flowkey_settings(self, ui, w, &other["flowkey.".len()..], left, width, &mut y),
                            _ => {}
                        }
                        self.extension_table(ui, w, index, "", left, width, &mut y);
                    }
                }
            }
            ui.expand_to_include_rect(Rect::from_min_max(origin, pos2(origin.x + 1.0, y + 16.0)));
        });
        self.conflict_ui(ui, w, full);
        ask_ui(ui, w, full);
    }

    fn page_title(&self, w: &SettingsWindow) -> String {
        PAGES.iter().find(|p| p.0 == w.page).map(|p| p.1.to_owned()).or_else(|| w.extensions.iter().find(|e| format!("extension:{}", e.id) == w.page).map(|e| e.title.clone())).unwrap_or_default()
    }

    fn sidebar(&mut self, ui: &mut Ui, rect: Rect, w: &mut SettingsWindow) {
        let field = Rect::from_min_size(pos2(rect.left() + 9.0, rect.top() + 16.0), vec2(rect.width() - 18.0, 32.0));
        ui.painter().rect_filled(field, 6.0, theme::fg(10));
        icons::paint_glyph(ui, "search", Rect::from_center_size(pos2(field.left() + 18.0, field.center().y), vec2(16.0, 16.0)), theme::FG_60);
        let inner = Rect::from_min_max(pos2(field.left() + 32.0, field.top() + 7.0), pos2(field.right() - 8.0, field.bottom() - 7.0));
        let hint = egui::RichText::new("Search settings...").color(theme::FG_60).font(theme::font(13.0));
        let response = ui.put(inner, egui::TextEdit::singleline(&mut w.nav_filter).id(egui::Id::unique("settings-nav-filter")).frame(egui::Frame::NONE).margin(egui::Margin::ZERO).font(theme::font(13.0)).text_color(theme::FG).hint_text(hint).desired_width(inner.width()));
        response.widget_info(|| egui::WidgetInfo::text_edit(true, "Search settings", &w.nav_filter, ""));
        let q = w.nav_filter.trim().to_lowercase();
        let matches = |title: &str, keywords: &[&str]| q.is_empty() || std::iter::once(title).chain(keywords.iter().copied()).any(|t| t.to_lowercase().contains(&q));
        let mut links: Vec<(String, String, Icon)> = PAGES.iter().filter(|p| matches(p.1, p.3)).map(|p| (p.0.to_owned(), p.1.to_owned(), Icon::Tile(p.2, Tone::Slate))).collect();
        let first_extension = links.len();
        links.extend(w.extensions.iter().filter(|e| matches(&e.title, &[])).map(|e| (format!("extension:{}", e.id), e.title.clone(), e.icon.clone())));
        let list = Rect::from_min_max(pos2(rect.left(), field.bottom() + 16.0), rect.max);
        let mut list_ui = ui.new_child(UiBuilder::new().max_rect(list));
        let mut picked = None;
        egui::ScrollArea::vertical().id_salt("settings-nav").auto_shrink(false).show(&mut list_ui, |ui| {
            for (n, (id, title, icon)) in links.iter().enumerate() {
                if n == first_extension && n > 0 {
                    ui.add_space(16.0);
                }
                let (row, response) = ui.allocate_exact_size(vec2(rect.width(), 32.0), Sense::click());
                let row = row.shrink2(vec2(9.0, 0.0));
                response.widget_info(|| egui::WidgetInfo::selected(egui::Role::Tab, true, w.page == *id, title));
                if w.page == *id {
                    ui.painter().rect_filled(row, 6.0, theme::fg(5));
                } else if response.hovered() {
                    ui.painter().rect_filled(row, 6.0, theme::fg(5));
                }
                if response.has_focus() {
                    ui.painter().rect_stroke(row, 6.0, Stroke::new(1.0, theme::ACCENT), egui::StrokeKind::Inside);
                }
                let icon_rect = Rect::from_min_size(pos2(row.left() + 6.0, row.center().y - 10.0), vec2(20.0, 20.0));
                icons::command_icon(&mut ui.new_child(UiBuilder::new().max_rect(icon_rect)), icon, title, 20.0);
                paint_text(ui, pos2(icon_rect.right() + 8.0, row.center().y), Align2::LEFT_CENTER, title, theme::weighted(13.0, theme::MEDIUM), theme::FG, row.width() - 40.0);
                if response.clicked() {
                    picked = Some(id.clone());
                }
            }
        });
        if let Some(page) = picked {
            w.navigate(page);
        }
    }

    fn select(&mut self, ui: &mut Ui, right: f32, center_y: f32, slot: usize, label: &str, options: &[(&str, &str, &'static str)], current: &str) -> Option<String> {
        let rect = Rect::from_min_size(pos2(right - 240.0, center_y - 15.0), vec2(240.0, 30.0));
        let response = Self::dropdown_trigger(ui, rect, SLOT + slot);
        let list: Vec<(String, Icon)> = options.iter().map(|(_, text, glyph)| (text.to_string(), Icon::Glyph(glyph))).collect();
        let selected = options.iter().position(|(value, _, _)| *value == current).unwrap_or(0);
        self.dropdown_ui(ui, response, SLOT + slot, label, &list, selected).map(|i| options[i].0.to_owned())
    }

    fn general_page(&mut self, ui: &mut Ui, w: &mut SettingsWindow, left: f32, width: f32, y: &mut f32) {
        let login = *w.login.get_or_insert_with(crate::registry::login_item_enabled);
        let mut toggled_login = None;
        let mut tray = None;
        let mut hotkey_action = None;
        card(ui, y, left, width, |ui, c| {
            let row = c.row(ui, "Open at Login", None, 40.0);
            if switch(ui, row.right() - 12.0, row.center().y, login, "Open at Login") {
                toggled_login = Some(!login);
            }
            let row = c.row(ui, "Show in System Tray", None, 40.0);
            if switch(ui, row.right() - 12.0, row.center().y, self.settings.show_in_tray, "Show in System Tray") {
                tray = Some(!self.settings.show_in_tray);
            }
            let row = c.row(ui, "Openray Hotkey", None, 300.0);
            let reset = Rect::from_min_size(pos2(row.right() - 12.0 - CONTROL, row.center().y - CONTROL / 2.0), vec2(CONTROL, CONTROL));
            let enabled = self.settings.hotkey != DEFAULT_HOTKEY;
            let response = ui.interact(reset, egui::Id::unique("settings-reset-hotkey"), if enabled { Sense::click() } else { Sense::hover() });
            response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, enabled, "Reset hotkey to Alt+Space"));
            ui.painter().rect_filled(reset, 5.0, if response.hovered() && enabled { theme::CONTROL_HOVER } else { theme::CONTROL });
            icons::paint_glyph(ui, "rotate-ccw", reset.shrink(5.0), if enabled { theme::FG } else { theme::FG_40 });
            if response.clicked() {
                hotkey_action = Some(DEFAULT_HOTKEY.to_owned());
            }
            if self.settings.hotkey != START_MENU_HOTKEY {
                let recorder = self.hotkey_cell(ui, w, LAUNCHER_OWNER, reset.left() - 8.0, row.center().y, &self.settings.hotkey.clone(), false);
                let (_, start_menu) = text_button(ui, recorder - 8.0, row.center().y, "Replace Start Menu", "start-menu", true);
                if start_menu {
                    hotkey_action = Some(START_MENU_HOTKEY.to_owned());
                }
            } else {
                keycap_row(ui, reset.left() - 8.0 - 40.0, row.center().y, &["Win".to_owned()]);
            }
        });
        let update_state = crate::updater::state();
        let mut update_row = Rect::NOTHING;
        card(ui, y, left, width, |ui, c| update_row = c.row(ui, "Updates", Some(&crate::updater::describe(&update_state)), 180.0));
        let ready = matches!(update_state, crate::updater::State::Ready(_));
        let busy = matches!(update_state, crate::updater::State::Checking | crate::updater::State::Downloading(_));
        let label = if ready { "Restart to Update" } else { "Check for Updates" };
        if text_button(ui, update_row.right() - 12.0, update_row.center().y, label, "check-updates", !busy).1 {
            if ready {
                match crate::updater::apply() {
                    Ok(()) => ui.ctx().send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Close),
                    Err(e) => w.error = Some(e),
                }
            } else {
                crate::updater::check_now(ui.ctx());
            }
        }
        if let Some(enabled) = toggled_login {
            match crate::registry::set_login_item(enabled) {
                Ok(()) => w.login = Some(enabled),
                Err(e) => w.error = Some(format!("Could not change Open at Login: {e}")),
            }
        }
        if let Some(on) = tray {
            self.settings.show_in_tray = on;
            self.save_settings();
        }
        if let Some(hotkey) = hotkey_action {
            self.settings.hotkey = hotkey;
            self.save_settings();
        }
    }

    fn launcher_page(&mut self, ui: &mut Ui, left: f32, width: f32, y: &mut f32) {
        let options = [("mouse-screen", "Screen containing mouse", "monitor"), ("active-screen", "Screen with active window", "app-window"), ("primary-screen", "Primary screen", "monitor")];
        let current = match self.settings.show_on {
            ShowOn::MouseScreen => "mouse-screen",
            ShowOn::ActiveScreen => "active-screen",
            ShowOn::PrimaryScreen => "primary-screen",
        };
        let mut row = Rect::NOTHING;
        card(ui, y, left, width, |ui, c| row = c.row(ui, "Show Openray on", None, 240.0));
        if let Some(value) = self.select(ui, row.right() - 12.0, row.center().y, 0, "Show Openray on", &options, current) {
            self.settings.show_on = match value.as_str() {
                "active-screen" => ShowOn::ActiveScreen,
                "primary-screen" => ShowOn::PrimaryScreen,
                _ => ShowOn::MouseScreen,
            };
            self.save_settings();
        }
    }

    fn keyboard_page(&mut self, ui: &mut Ui, left: f32, width: f32, y: &mut f32) {
        let options = [("pop-back-or-close", "Navigate back or close window", "arrow-left"), ("close-and-pop-to-root", "Close window and pop to root", "x")];
        let current = if self.settings.escape_key_behavior == EscapeKeyBehavior::CloseAndPopToRoot { "close-and-pop-to-root" } else { "pop-back-or-close" };
        let (mut behavior, mut closes) = (Rect::NOTHING, false);
        card(ui, y, left, width, |ui, c| {
            behavior = c.row(ui, "Escape Key Behavior", Some("What Escape does in the launcher. The default clears the search, then closes the window."), 240.0);
            let row = c.row(ui, "Escape Key Closes Window", Some("Close windows such as Settings with the Escape key."), 40.0);
            closes = switch(ui, row.right() - 12.0, row.center().y, self.settings.escape_key_closes_window, "Escape Key Closes Window");
        });
        if closes {
            self.settings.escape_key_closes_window = !self.settings.escape_key_closes_window;
            self.save_settings();
        }
        if let Some(value) = self.select(ui, behavior.right() - 12.0, behavior.center().y, 1, "Escape Key Behavior", &options, current) {
            self.settings.escape_key_behavior = if value == "close-and-pop-to-root" { EscapeKeyBehavior::CloseAndPopToRoot } else { EscapeKeyBehavior::PopBackOrClose };
            self.save_settings();
        }
    }

    fn hotkey_cell(&self, ui: &mut Ui, w: &mut SettingsWindow, owner: &str, right: f32, center_y: f32, hotkey: &str, clearable: bool) -> f32 {
        let recording = w.recording.as_deref() == Some(owner);
        let labels = hotkey_labels(hotkey);
        let text = if recording { "Press keys…" } else if hotkey.is_empty() { "Record Hotkey" } else { "" };
        let font = theme::weighted(12.0, theme::MEDIUM);
        let content = if text.is_empty() {
            labels.iter().map(|l| ui.fonts_mut(|f| f.layout_no_wrap(l.clone(), font.clone(), theme::FG)).size().x.max(CONTROL - 12.0) + 16.0).sum::<f32>()
        } else {
            ui.fonts_mut(|f| f.layout_no_wrap(text.to_owned(), font.clone(), theme::FG)).size().x + 16.0
        };
        let clear_width = if clearable && !hotkey.is_empty() && !recording { CONTROL } else { 0.0 };
        let rect = Rect::from_min_size(pos2(right - content - clear_width - 8.0, center_y - CONTROL / 2.0 - 2.0), vec2(content + clear_width + 8.0, CONTROL + 4.0));
        let response = ui.interact(rect, egui::Id::unique(("hotkey-cell", owner)), Sense::click());
        response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, format!("Hotkey: {}", if hotkey.is_empty() { "none" } else { hotkey })));
        let border = if recording { theme::ACCENT } else if response.hovered() || response.has_focus() { theme::FG_40 } else { theme::KEY_BORDER };
        ui.painter().rect(rect, 6.0, Color32::TRANSPARENT, Stroke::new(1.0, border), egui::StrokeKind::Inside);
        if text.is_empty() {
            keycap_row(ui, rect.left() + 4.0, center_y, &labels);
        } else {
            paint_text(ui, pos2(rect.left() + 8.0, center_y), Align2::LEFT_CENTER, text, font, if recording { theme::FG } else { theme::FG_40 }, content);
        }
        let mut cleared = false;
        if clear_width > 0.0 {
            let clear = Rect::from_min_size(pos2(rect.right() - CONTROL - 2.0, center_y - 10.0), vec2(20.0, 20.0));
            let x = ui.interact(clear, egui::Id::unique(("hotkey-clear", owner)), Sense::CLICK);
            x.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, "Clear hotkey"));
            icons::paint_glyph(ui, "x", clear.shrink(4.0), if x.hovered() { theme::FG } else { theme::FG_60 });
            if x.clicked() {
                cleared = true;
                w.recording = Some(format!("clear:{owner}"));
            }
        }
        if response.clicked() && !cleared {
            if recording {
                w.recording = None;
                crate::hotkey::suspend(false);
            } else {
                w.recording = Some(owner.to_owned());
                w.waiting_release = true;
                crate::hotkey::suspend(true);
            }
        }
        rect.left()
    }

    fn record_hotkey(&mut self, ui: &Ui, w: &mut SettingsWindow) {
        let Some(owner) = w.recording.clone() else { return };
        if let Some(target) = owner.strip_prefix("clear:") {
            w.recording = None;
            if let Some(prefs) = self.settings.commands.get_mut(target) {
                prefs.hotkey.clear();
            }
            self.save_settings();
            return;
        }
        ui.ctx().request_repaint();
        if w.waiting_release {
            w.waiting_release = any_key_down() || unsafe { GetAsyncKeyState(0x01) as u16 & 0x8000 != 0 };
            crate::hotkey::LAST_DOWN.store(0, std::sync::atomic::Ordering::SeqCst);
            return;
        }
        if ui.input(|i| i.key_pressed(Key::Escape)) {
            w.recording = None;
            crate::hotkey::suspend(false);
            return;
        }
        let last_down = crate::hotkey::LAST_DOWN.swap(0, std::sync::atomic::Ordering::SeqCst);
        let Some(Ok(hotkey)) = pressed_hotkey(last_down) else { return };
        w.recording = None;
        crate::hotkey::suspend(false);
        match hotkey_owner(&self.settings, &hotkey, &owner) {
            Some(found) => {
                let name = if found == LAUNCHER_OWNER { "Openray".to_owned() } else { self.command_title(w, &found) };
                w.conflict = Some(Conflict { owner, hotkey, name, overwritable: found != LAUNCHER_OWNER });
            }
            None => {
                with_hotkey(&mut self.settings, &owner, &hotkey, false);
                self.save_settings();
            }
        }
    }

    fn command_title(&self, w: &SettingsWindow, key: &str) -> String {
        w.extensions.iter().flat_map(|e| &e.commands).find(|c| c.0 == key).map(|c| c.1.clone()).unwrap_or_else(|| key.to_owned())
    }

    fn conflict_ui(&mut self, ui: &mut Ui, w: &mut SettingsWindow, full: Rect) {
        let Some(conflict) = &w.conflict else { return };
        let message = if conflict.overwritable { format!("{} is already used by {}. Replace it?", conflict.hotkey, conflict.name) } else { format!("{} is already used by {}.", conflict.hotkey, conflict.name) };
        let mut answer = None;
        egui::Area::new(egui::Id::unique("settings-conflict")).order(egui::Order::Foreground).fixed_pos(full.min).show(ui.ctx(), |ui| {
            ui.painter().rect_filled(full, 0.0, Color32::from_black_alpha(102));
            let panel = Rect::from_center_size(full.center(), vec2(320.0, 132.0));
            ui.painter().rect(panel, 8.0, theme::PANEL, Stroke::new(1.0, theme::PANEL_BORDER), egui::StrokeKind::Inside);
            paint_text(ui, pos2(panel.center().x, panel.top() + 18.0), Align2::CENTER_TOP, "Hotkey in use", theme::weighted(13.0, theme::SEMIBOLD), theme::FG, 290.0);
            let galley = super::wrapped(ui, &message, theme::font(12.0), theme::FG_60, 288.0);
            ui.painter().galley(pos2(panel.center().x - galley.size().x / 2.0, panel.top() + 40.0), galley, theme::FG_60);
            let right = panel.right() - 16.0;
            let y = panel.bottom() - 30.0;
            if conflict.overwritable {
                let (rect, replace) = text_button(ui, right, y, "Replace", "conflict-replace", true);
                if replace {
                    answer = Some(true);
                }
                let (_, cancel) = text_button(ui, rect.left() - 8.0, y, "Cancel", "conflict-cancel", true);
                if cancel {
                    answer = Some(false);
                }
            } else if text_button(ui, right, y, "OK", "conflict-ok", true).1 {
                answer = Some(false);
            }
        });
        if ui.input(|i| i.key_pressed(Key::Escape)) {
            answer = Some(false);
        }
        if let Some(replace) = answer {
            let conflict = w.conflict.take().expect("conflict shown");
            if replace {
                with_hotkey(&mut self.settings, &conflict.owner, &conflict.hotkey, true);
                self.save_settings();
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn extension_table(&mut self, ui: &mut Ui, w: &mut SettingsWindow, index: usize, term: &str, left: f32, width: f32, y: &mut f32) {
        let (id, title, icon) = (w.extensions[index].id.clone(), w.extensions[index].title.clone(), w.extensions[index].icon.clone());
        let commands: Vec<(String, String, Icon)> = w.extensions[index].commands.iter().filter(|c| term.is_empty() || c.1.to_lowercase().contains(term)).cloned().collect();
        if !term.is_empty() && commands.is_empty() {
            return;
        }
        let collapsed = w.collapsed.contains(&id);
        let header_height = 46.0;
        let table_height = if collapsed || commands.is_empty() { 0.0 } else { 24.0 + commands.len() as f32 * 46.0 };
        let rect = Rect::from_min_size(pos2(left, *y), vec2(width, header_height + table_height));
        *y = rect.bottom() + 16.0;
        if !ui.is_rect_visible(rect) {
            return;
        }
        ui.painter().rect_filled(rect, 8.0, CARD);
        let header = Rect::from_min_size(rect.min, vec2(width, header_height));
        let icon_rect = Rect::from_min_size(pos2(header.left() + 12.0, header.center().y - 10.0), vec2(20.0, 20.0));
        icons::command_icon(&mut ui.new_child(UiBuilder::new().max_rect(icon_rect)), &icon, &title, 20.0);
        let label = paint_text(ui, pos2(icon_rect.right() + 8.0, header.center().y), Align2::LEFT_CENTER, &title, theme::weighted(13.0, theme::MEDIUM), theme::FG, 300.0);
        let toggle = Rect::from_min_max(pos2(icon_rect.right() + 4.0, header.top() + 8.0), pos2(label.right() + 22.0, header.bottom() - 8.0));
        let response = ui.interact(toggle, egui::Id::unique(("settings-collapse", &id)), Sense::click());
        response.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, true, !collapsed, &title));
        let chevron = Rect::from_center_size(pos2(label.right() + 12.0, header.center().y), vec2(12.0, 12.0));
        icons::paint_glyph(ui, if collapsed { "chevron-right" } else { "chevron-down" }, chevron, theme::FG_60);
        if response.clicked() {
            if collapsed {
                w.collapsed.remove(&id);
            } else {
                w.collapsed.insert(id.clone());
            }
        }
        let enabled = !self.settings.commands.get(&id).is_some_and(|p| p.disabled);
        let check = Rect::from_min_size(pos2(header.right() - 12.0 - 16.0, header.center().y - 8.0), vec2(16.0, 16.0));
        if checkbox(ui, check, enabled, &format!("Enable {title}")) {
            self.settings.commands.entry(id.clone()).or_insert_with(CommandPrefs::default).disabled = enabled;
            self.save_settings();
        }
        if table_height == 0.0 {
            return;
        }
        let (alias_x, hotkey_right) = (rect.right() - 12.0 - 40.0 - 133.0 - 140.0, rect.right() - 12.0 - 40.0);
        let head_y = header.bottom() + 12.0;
        ui.painter().hline(rect.x_range(), header.bottom() + 24.0, Stroke::new(1.0, SEPARATOR));
        let dim = if enabled { 1.0 } else { 0.5 };
        for (text, x) in [("Name", rect.left() + 12.0), ("Alias", alias_x), ("Hotkey", hotkey_right - 133.0)] {
            paint_text(ui, pos2(x, head_y), Align2::LEFT_CENTER, text, theme::weighted(12.0, theme::SEMIBOLD), theme::FG_60.gamma_multiply(dim), 120.0);
        }
        let mut changes: Vec<(String, CommandPrefs)> = Vec::new();
        for (n, (key, name, command_icon)) in commands.iter().enumerate() {
            let row = Rect::from_min_size(pos2(rect.left(), header.bottom() + 24.0 + n as f32 * 46.0), vec2(width, 46.0));
            if n > 0 {
                ui.painter().hline(row.x_range(), row.top(), Stroke::new(1.0, SEPARATOR));
            }
            if !ui.is_rect_visible(row) {
                continue;
            }
            let prefs = self.settings.commands.get(key).cloned().unwrap_or_default();
            let icon_rect = Rect::from_min_size(pos2(row.left() + 12.0, row.center().y - 8.0), vec2(16.0, 16.0));
            icons::command_icon(&mut ui.new_child(UiBuilder::new().max_rect(icon_rect)), command_icon, name, 16.0);
            paint_text(ui, pos2(icon_rect.right() + 8.0, row.center().y), Align2::LEFT_CENTER, name, theme::weighted(13.0, theme::MEDIUM), theme::FG.gamma_multiply(dim), alias_x - icon_rect.right() - 20.0);
            let alias_id = egui::Id::unique(("settings-alias", key));
            let mut alias: String = ui.data(|d| d.get_temp(alias_id)).unwrap_or_else(|| prefs.alias.clone());
            let field = Rect::from_min_size(pos2(alias_x - 6.0, row.center().y - 12.0), vec2(120.0, 24.0));
            let focused = ui.memory(|m| m.has_focus(alias_id));
            if focused || ui.rect_contains_pointer(field) {
                ui.painter().rect_filled(field, 5.0, theme::CONTROL);
            }
            let hint = egui::RichText::new("Add Alias").color(theme::FG_40).font(theme::weighted(13.0, theme::MEDIUM));
            let inner = field.shrink2(vec2(6.0, 4.0));
            let response = ui.put(inner, egui::TextEdit::singleline(&mut alias).id(alias_id).frame(egui::Frame::NONE).margin(egui::Margin::ZERO).font(theme::weighted(13.0, theme::MEDIUM)).text_color(theme::FG).hint_text(hint).desired_width(inner.width()));
            response.widget_info(|| egui::WidgetInfo::text_edit(true, format!("Alias for {name}"), &alias, ""));
            if response.lost_focus() {
                ui.data_mut(|d| d.remove::<String>(alias_id));
                if alias.trim() != prefs.alias {
                    changes.push((key.clone(), CommandPrefs { alias: alias.trim().to_owned(), ..prefs.clone() }));
                }
            } else if response.has_focus() {
                ui.data_mut(|d| d.insert_temp(alias_id, alias));
            }
            self.hotkey_cell(ui, w, key, hotkey_right, row.center().y, &prefs.hotkey, true);
            let check = Rect::from_min_size(pos2(row.right() - 12.0 - 16.0, row.center().y - 8.0), vec2(16.0, 16.0));
            if checkbox(ui, check, !prefs.disabled, &format!("Enable {name}")) {
                changes.push((key.clone(), CommandPrefs { disabled: !prefs.disabled, ..prefs.clone() }));
            }
        }
        if !changes.is_empty() {
            for (key, prefs) in changes {
                self.settings.commands.insert(key, prefs);
            }
            self.save_settings();
        }
    }
}

fn focus_settings(launcher: &mut Launcher, ui: &mut Ui, w: &mut SettingsWindow, left: f32, width: f32, y: &mut f32) {
    let mut prefs = w.focus.get_or_insert_with(crate::extensions::focus_preferences).clone();
    let before = prefs.clone();
    let mut snooze_row = Rect::NOTHING;
    card(ui, y, left, width, |ui, c| {
        snooze_row = c.row(ui, "Snooze Duration", Some("How long blocked apps and websites stay snoozed."), 240.0);
        let row = c.row(ui, "Play Sound", Some("Play sounds for Focus notifications."), 40.0);
        if switch(ui, row.right() - 12.0, row.center().y, prefs.play_sound, "Play Sound") {
            prefs.play_sound = !prefs.play_sound;
        }
    });
    let options = [("60000", "1 minute", "clock"), ("180000", "3 minutes", "clock"), ("300000", "5 minutes", "clock"), ("600000", "10 minutes", "clock"), ("0", "No Snooze", "x")];
    let current = prefs.snooze_ms.filter(|ms| *ms > 0).map_or("0".to_owned(), |ms| ms.to_string());
    if let Some(value) = launcher.select(ui, snooze_row.right() - 12.0, snooze_row.center().y, 3, "Snooze Duration", &options, &current) {
        prefs.snooze_ms = value.parse().ok().filter(|ms| *ms > 0);
    }
    card(ui, y, left, width, |ui, c| {
        let row = c.row(ui, "Allow all Websites", Some("Allow every website when a browser is allowed and no websites are listed."), 40.0);
        if switch(ui, row.right() - 12.0, row.center().y, prefs.allow_all_websites, "Allow all Websites") {
            prefs.allow_all_websites = !prefs.allow_all_websites;
        }
        let row = c.row(ui, "Restore Terminated Apps", Some("Reopen the apps Focus closed when the session ends."), 40.0);
        if switch(ui, row.right() - 12.0, row.center().y, prefs.restore_apps, "Restore Terminated Apps") {
            prefs.restore_apps = !prefs.restore_apps;
        }
    });
    if prefs != before {
        match crate::extensions::save_focus_preferences(&prefs) {
            Ok(()) => w.focus = Some(prefs),
            Err(e) => w.error = Some(format!("Could not save Focus settings: {e}")),
        }
    }
}

fn screenshot_settings(ui: &mut Ui, w: &mut SettingsWindow, left: f32, width: f32, y: &mut f32) {
    let folder = crate::capture::folder();
    let custom = crate::capture::CUSTOM_FOLDER.lock().unwrap_or_else(|e| e.into_inner()).is_some();
    let description = format!("{}{}", folder.display(), if custom { "" } else { " (default)" });
    let mut row = Rect::NOTHING;
    card(ui, y, left, width, |ui, c| row = c.row(ui, "Save Screenshots To", Some(&description), 260.0));
    let (change_rect, change) = text_button(ui, row.right() - 12.0, row.center().y, "Change…", "screenshots-change", true);
    let mut right = change_rect.left() - 8.0;
    let mut reset = false;
    if custom {
        let (rect, clicked) = text_button(ui, right, row.center().y, "Use Default", "screenshots-default", true);
        reset = clicked;
        right = rect.left() - 8.0;
    }
    let (_, open) = text_button(ui, right, row.center().y, "Open", "screenshots-open", true);
    let result = if open {
        std::fs::create_dir_all(&folder).map_err(|e| e.to_string()).and_then(|_| crate::apps::launch(&folder.to_string_lossy(), false))
    } else if reset {
        set_screenshot_folder("")
    } else if change {
        match crate::apps::pick_path(true, false, &[]) {
            Ok(list) => list.first().map_or(Ok(()), |path| set_screenshot_folder(path)),
            Err(e) => Err(e),
        }
    } else {
        Ok(())
    };
    if let Err(e) = result {
        w.error = Some(e);
    }
}

pub fn set_screenshot_folder(path: &str) -> Result<(), String> {
    let value = serde_json::json!({ "folder": path });
    let folder = crate::capture::custom_folder(&value);
    match &folder {
        Some(dir) => std::fs::create_dir_all(dir).map_err(|e| format!("Can't use {}: {e}", dir.display()))?,
        None if !path.trim().is_empty() => return Err("Choose a full folder path".into()),
        None => {}
    }
    crate::store::write_json(&crate::data_dir().join("screenshots.json"), &value).map_err(|e| e.to_string())?;
    *crate::capture::CUSTOM_FOLDER.lock().unwrap_or_else(|e| e.into_inner()) = folder;
    crate::capture_ui::VERSION.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    Ok(())
}

fn dictionary_settings(launcher: &mut Launcher, ui: &mut Ui, w: &mut SettingsWindow, left: f32, width: f32, y: &mut f32) {
    let current = w.dictionary.get_or_insert_with(crate::extensions::dictionary_language).clone();
    let mut row = Rect::NOTHING;
    card(ui, y, left, width, |ui, c| row = c.row(ui, "Language", Some("Language of the words and definitions in Define Word."), 240.0));
    let options = [("en", "English", "languages"), ("pt", "Português", "languages")];
    if let Some(value) = launcher.select(ui, row.right() - 12.0, row.center().y, 2, "Language", &options, &current) {
        match crate::extensions::set_dictionary_language(&value) {
            Ok(()) => w.dictionary = Some(value),
            Err(e) => w.error = Some(format!("Could not save the dictionary language: {e}")),
        }
    }
}

fn status_line(ui: &Ui, w: &SettingsWindow, left: f32, width: f32, y: &mut f32) {
    if let Some((text, error)) = &w.flowkey_status {
        let color = if *error { theme::RED } else { theme::FG_60 };
        let galley = super::wrapped(ui, text, theme::font(12.0), color, width);
        let h = galley.size().y;
        ui.painter().galley(pos2(left, *y), galley, color);
        *y += h + 12.0;
    }
}

fn changed() {
    crate::extensions::GENERATION.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
}

fn start_install(w: &mut SettingsWindow) -> Result<(), String> {
    let Some(path) = crate::apps::pick_path(false, false, &["flowkey".into(), "zip".into()])?.into_iter().next() else { return Ok(()) };
    let manifest = crate::flowkey_core::package_manifest(&path).map_err(|e| e.message)?;
    flowkey::check_package(&manifest)?;
    let text = |field: &str| manifest[field].as_str().unwrap_or_default().to_owned();
    let (id, name, version) = (text("id"), text("name"), text("version"));
    if let Some(current) = w.flowkey.iter().find(|e| e.id() == id && e.source == "installed") {
        let installed = current.manifest["version"].as_str().unwrap_or_default();
        if !flowkey::newer_version(&version, installed) {
            return Err(format!("{name} {installed} is already installed. Only a newer version can replace it."));
        }
    }
    let grants = crate::flowkey_gate::Grants::of(&manifest).describe();
    let message = if grants.is_empty() { "It asks for no permissions.".to_owned() } else { format!("It will be allowed to:\n{}", grants.iter().map(|g| format!("•  {g}")).collect::<Vec<_>>().join("\n")) };
    w.ask = Some(Ask { title: format!("Install “{name}” {version}?"), message, ok: "Install", action: AskAction::Install { path, id, label: format!("{name} {version}") } });
    Ok(())
}

fn flowkey_install_page(ui: &mut Ui, w: &mut SettingsWindow, left: f32, width: f32, y: &mut f32) {
    let folder = crate::flowkey_core::installed_root();
    let (mut install_row, mut folder_row) = (Rect::NOTHING, Rect::NOTHING);
    let failures = w.flowkey_failures.clone();
    card(ui, y, left, width, |ui, c| {
        install_row = c.row(ui, "Install Extension", Some("Add a .flowkey package built with the FlowKey CLI."), 150.0);
        folder_row = c.row(ui, "Extensions Folder", Some(&folder.to_string_lossy()), 120.0);
        for failure in &failures {
            let (id, issue) = failure.split_once(": ").unwrap_or((failure, ""));
            c.row(ui, &format!("{id} could not load"), Some(issue), 0.0);
        }
    });
    if text_button(ui, install_row.right() - 12.0, install_row.center().y, "Install from File…", "flowkey-install", true).1 {
        w.flowkey_status = start_install(w).err().map(|e| (e, true));
    }
    if text_button(ui, folder_row.right() - 12.0, folder_row.center().y, "Open Folder", "flowkey-folder", true).1 {
        if let Err(e) = std::fs::create_dir_all(&folder).map_err(|e| e.to_string()).and_then(|_| crate::apps::launch(&folder.to_string_lossy(), false)) {
            w.flowkey_status = Some((e, true));
        }
    }
    status_line(ui, w, left, width, y);
}

fn ask_ui(ui: &mut Ui, w: &mut SettingsWindow, full: Rect) {
    let Some(ask) = &w.ask else { return };
    let mut answer = None;
    egui::Area::new(egui::Id::unique("settings-ask")).order(egui::Order::Foreground).fixed_pos(full.min).show(ui.ctx(), |ui| {
        ui.painter().rect_filled(full, 0.0, Color32::from_black_alpha(102));
        let galley = super::wrapped(ui, &ask.message, theme::font(12.0), theme::FG_60, 328.0);
        let panel = Rect::from_center_size(full.center(), vec2(360.0, 100.0 + galley.size().y));
        ui.painter().rect(panel, 8.0, theme::PANEL, Stroke::new(1.0, theme::PANEL_BORDER), egui::StrokeKind::Inside);
        paint_text(ui, pos2(panel.center().x, panel.top() + 18.0), Align2::CENTER_TOP, &ask.title, theme::weighted(13.0, theme::SEMIBOLD), theme::FG, 330.0);
        ui.painter().galley(pos2(panel.left() + 16.0, panel.top() + 42.0), galley, theme::FG_60);
        let (right, y) = (panel.right() - 16.0, panel.bottom() - 26.0);
        let (rect, ok) = text_button(ui, right, y, ask.ok, "ask-ok", true);
        if ok {
            answer = Some(true);
        }
        if text_button(ui, rect.left() - 8.0, y, "Cancel", "ask-cancel", true).1 {
            answer = Some(false);
        }
    });
    if ui.input(|i| i.key_pressed(Key::Escape)) {
        answer = Some(false);
    }
    if answer != Some(true) {
        if answer == Some(false) {
            w.ask = None;
        }
        return;
    }
    let Some(ask) = w.ask.take() else { return };
    let result = match ask.action {
        AskAction::Install { path, id, label } => crate::flowkey_core::install_package(&path, &id, &crate::flowkey_core::installed_root()).map_err(|e| e.message).and_then(|dir| {
            flowkey::unload(&id);
            let manifest = std::fs::read_to_string(dir.join("manifest.json")).ok().and_then(|t| serde_json::from_str::<Value>(&t).ok()).unwrap_or(Value::Null);
            flowkey::grant_consent(&Installed { manifest, source: "installed", dir })?;
            Ok(format!("Installed {label}."))
        }),
        AskAction::Uninstall(ext) => crate::flowkey_core::uninstall(&flowkey::data_base(), ext.id()).map_err(|e| e.message).map(|()| {
            flowkey::unload(ext.id());
            if w.page == format!("extension:flowkey.{}", ext.id()) {
                w.page = "extensions".into();
                w.history.clear();
            }
            format!("Uninstalled {}.", ext.name())
        }),
    };
    w.flowkey_status = Some(match result {
        Ok(text) => (text, false),
        Err(e) => (e, true),
    });
    changed();
}

fn preference_value(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Null) | None => String::new(),
        Some(other) => other.to_string(),
    }
}

#[allow(clippy::too_many_arguments)]
fn flowkey_settings(launcher: &mut Launcher, ui: &mut Ui, w: &mut SettingsWindow, id: &str, left: f32, width: f32, y: &mut f32) {
    let Some(ext) = w.flowkey.iter().find(|e| e.id() == id).cloned() else { return };
    let manifest = &ext.manifest;
    let installed = ext.source == "installed";
    let (mut about, mut remove) = (Rect::NOTHING, Rect::NOTHING);
    card(ui, y, left, width, |ui, c| {
        about = c.row(ui, &format!("{} {}", ext.name(), manifest["version"].as_str().unwrap_or_default()), manifest["description"].as_str(), 160.0);
        if installed {
            remove = c.row(ui, "Uninstall", Some(&ext.dir.to_string_lossy()), 110.0);
        }
    });
    let source = if installed { "Installed" } else { "Bundled with Openray" };
    paint_text(ui, pos2(about.right() - 12.0, about.center().y), Align2::RIGHT_CENTER, source, theme::font(12.0), theme::FG_60, 150.0);
    if installed && text_button(ui, remove.right() - 12.0, remove.center().y, "Uninstall…", "flowkey-uninstall", true).1 {
        w.ask = Some(Ask {
            title: format!("Uninstall “{}”?", ext.name()),
            message: "This deletes the extension and its saved data, secrets and sign-ins from this computer. You can install it again later.".into(),
            ok: "Uninstall",
            action: AskAction::Uninstall(ext.clone()),
        });
    }
    let schema: Vec<Value> = manifest["preferences"].as_array().cloned().unwrap_or_default();
    let mut values = flowkey::preferences(&ext);
    let mut updates: Vec<(String, Value)> = Vec::new();
    if !schema.is_empty() {
        let mut rows = Vec::new();
        card(ui, y, left, width, |ui, c| {
            for pref in &schema {
                let title = pref["title"].as_str().or(pref["name"].as_str()).unwrap_or_default();
                let title = if pref["required"] == true { format!("{title} (required)") } else { title.to_owned() };
                let reserve = match pref["type"].as_str() {
                    Some("checkbox") => 40.0 + pref["label"].as_str().map_or(0.0, |_| 140.0),
                    Some("file" | "directory") => 320.0,
                    _ => 240.0,
                };
                rows.push(c.row(ui, &title, pref["description"].as_str(), reserve));
            }
        });
        for (i, (pref, row)) in schema.iter().zip(rows).enumerate() {
            let Some(name) = pref["name"].as_str() else { continue };
            let (right, cy) = (row.right() - 12.0, row.center().y);
            let current = values.get(name).cloned();
            match pref["type"].as_str().unwrap_or("text") {
                "checkbox" => {
                    let on = current == Some(Value::Bool(true));
                    if let Some(label) = pref["label"].as_str() {
                        paint_text(ui, pos2(right - 40.0, cy), Align2::RIGHT_CENTER, label, theme::font(12.0), theme::FG_60, 140.0);
                    }
                    if switch(ui, right, cy, on, &format!("{id}-{name}")) {
                        updates.push((name.to_owned(), Value::Bool(!on)));
                    }
                }
                kind @ ("dropdown" | "appPicker") => {
                    let mut options: Vec<(String, String)> = if kind == "dropdown" {
                        pref["options"].as_array().into_iter().flatten().map(|o| (o["value"].as_str().unwrap_or_default().to_owned(), o["title"].as_str().unwrap_or_default().to_owned())).collect()
                    } else {
                        crate::extensions::APPS.lock().unwrap_or_else(|e| e.into_inner()).iter().map(|a| (a.path.clone(), a.name.clone())).collect()
                    };
                    let selected = preference_value(current.as_ref());
                    if !options.iter().any(|(v, _)| *v == selected) {
                        options.insert(0, (selected.clone(), if selected.is_empty() { "Choose…".into() } else { selected.clone() }));
                    }
                    let refs: Vec<(&str, &str, &'static str)> = options.iter().map(|(v, t)| (v.as_str(), t.as_str(), if kind == "dropdown" { "list" } else { "layout-grid" })).collect();
                    if let Some(value) = launcher.select(ui, right, cy, 100 + i, &format!("{id}-{name}"), &refs, &selected) {
                        updates.push((name.to_owned(), Value::String(value)));
                    }
                }
                kind => {
                    let browse = matches!(kind, "file" | "directory");
                    let mut field_right = right;
                    if browse {
                        let (rect, clicked) = text_button(ui, right, cy, "Browse…", &format!("{id}-{name}-browse"), true);
                        field_right = rect.left() - 8.0;
                        if clicked {
                            match crate::apps::pick_path(kind == "directory", false, &[]) {
                                Ok(list) => {
                                    if let Some(path) = list.into_iter().next() {
                                        updates.push((name.to_owned(), Value::String(path)));
                                    }
                                }
                                Err(e) => w.flowkey_status = Some((e, true)),
                            }
                        }
                    }
                    let key = format!("{id}/{name}");
                    let stored = preference_value(current.as_ref());
                    let buffer = w.flowkey_text.entry(key.clone()).or_insert_with(|| stored.clone());
                    let field = Rect::from_min_size(pos2(field_right - 220.0, cy - CONTROL / 2.0), vec2(220.0, CONTROL));
                    ui.painter().rect_filled(field, 5.0, theme::fg(5));
                    let hint = egui::RichText::new(pref["placeholder"].as_str().unwrap_or_default()).color(theme::FG_40).font(theme::font(12.0));
                    let inner = field.shrink2(vec2(8.0, 4.0));
                    let label = pref["title"].as_str().unwrap_or(name);
                    let edit = egui::TextEdit::singleline(buffer).id(egui::Id::unique(("flowkey-pref", key.as_str()))).frame(egui::Frame::NONE).margin(egui::Margin::ZERO).font(theme::font(12.0)).text_color(theme::FG).hint_text(hint).desired_width(inner.width()).password(kind == "password");
                    let response = ui.put(inner, edit);
                    response.widget_info(|| egui::WidgetInfo::text_edit(true, label, "", ""));
                    if response.has_focus() {
                        ui.painter().rect_stroke(field, 5.0, Stroke::new(1.0, theme::ACCENT), egui::StrokeKind::Inside);
                    }
                    if response.lost_focus() && *buffer != stored {
                        updates.push((name.to_owned(), Value::String(buffer.clone())));
                    }
                }
            }
        }
    }
    if !updates.is_empty() {
        for (name, value) in updates {
            w.flowkey_text.insert(format!("{id}/{name}"), preference_value(Some(&value)));
            values.insert(name, value);
        }
        w.flowkey_status = flowkey::set_preferences(&ext, values).err().map(|e| (format!("Could not save preferences: {e}"), true));
    }
    let granted = flowkey::grants(&ext).describe();
    let pending = flowkey::pending_consent(&ext).map(|g| g.describe()).unwrap_or_default();
    let mut permissions = Rect::NOTHING;
    let summary = if granted.is_empty() { "No permissions granted".to_owned() } else { granted.join(" · ") };
    card(ui, y, left, width, |ui, c| {
        permissions = c.row(ui, "Permissions", Some(&summary), 230.0);
        if !pending.is_empty() {
            c.row(ui, "Waiting for Approval", Some(&pending.join(" · ")), 0.0);
        }
    });
    let mut right = permissions.right() - 12.0;
    if installed && !granted.is_empty() {
        let (rect, revoke) = text_button(ui, right, permissions.center().y, "Revoke All", "flowkey-revoke", true);
        right = rect.left() - 8.0;
        if revoke {
            w.flowkey_status = flowkey::revoke_consent(&ext).err().map(|e| (e, true));
        }
    }
    if !pending.is_empty() && text_button(ui, right, permissions.center().y, &format!("Allow {} Pending", pending.len()), "flowkey-allow", true).1 {
        w.flowkey_status = flowkey::grant_consent(&ext).err().map(|e| (e, true));
    }
    status_line(ui, w, left, width, y);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hotkeys_match_regardless_of_modifier_order_and_case() {
        assert!(same_hotkey("Ctrl+Alt+K", "alt+ctrl+k"));
        assert!(!same_hotkey("Ctrl+K", "Ctrl+J"));
        assert!(!same_hotkey("", ""));
    }

    #[test]
    fn finds_and_replaces_the_owner_of_a_hotkey() {
        let mut settings = Settings::default();
        settings.commands.insert("a/one".into(), CommandPrefs { hotkey: "Ctrl+K".into(), ..Default::default() });
        assert_eq!(hotkey_owner(&settings, "Alt+Space", "a/one").as_deref(), Some(LAUNCHER_OWNER));
        assert_eq!(hotkey_owner(&settings, "ctrl+k", "b/two").as_deref(), Some("a/one"));
        assert_eq!(hotkey_owner(&settings, "Ctrl+K", "a/one"), None);
        with_hotkey(&mut settings, "b/two", "Ctrl+K", true);
        assert_eq!(settings.commands["a/one"].hotkey, "");
        assert_eq!(settings.commands["b/two"].hotkey, "Ctrl+K");
        with_hotkey(&mut settings, LAUNCHER_OWNER, "Ctrl+Space", false);
        assert_eq!(settings.hotkey, "Ctrl+Space");
    }

    #[test]
    fn every_recordable_key_has_a_virtual_key() {
        let table = key_table();
        assert!(table.iter().any(|(vk, name)| *vk == 0x2C && name == "PrintScreen"));
        assert!(table.iter().any(|(vk, name)| *vk == 0x41 && name == "A"));
        assert!(table.len() >= 90, "{}", table.len());
        assert_eq!(hotkey_labels("Ctrl+ArrowUp"), vec!["Ctrl", "↑"]);
    }
}
