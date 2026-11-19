#[path = "settings_ui.rs"]
mod settings_ui;

use crate::extension::{Accessory, Action, Async, CardKind, Command, Confirmation, Extension, Host, Icon, Inline, Page, Request, Shortcut, Side, Toast, View, ViewFactory};
use crate::search::{match_score, search, Frecency};
use crate::store::{EscapeKeyBehavior, Settings, Store};
use crate::{icons, launcher, theme};
use egui::text::{LayoutJob, TextFormat, TextWrapping};
use egui::{pos2, vec2, Align2, Color32, FontId, Key, Modifiers, Pos2, Rect, Sense, Stroke, Ui, UiBuilder};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::mpsc::Receiver;

const HEADER: f32 = 66.0;
const FOOTER: f32 = 42.0;
const ROW: f32 = 38.0;
const CARD: f32 = 124.0;
const SECTION: f32 = 32.0;
const INSET: f32 = 8.0;
const POP_TO_ROOT_AFTER: f64 = 90.0;
const TOAST_SECONDS: f64 = 3.0;
const HUD_SECONDS: f64 = 1.2;
const HUD_FAILURE_SECONDS: f64 = 3.0;
const PANEL_WIDTH: f32 = 350.0;
const PANEL_MAX_LIST: f32 = 239.0;
const DETAIL_LIST: f32 = 285.0;
const DROPDOWN_WIDTH: f32 = 197.0;
const GRID_GAP: f32 = 8.0;
const GRID_ICON: f32 = 50.0;

#[derive(Clone)]
struct Open {
    slot: usize,
    highlight: usize,
    count: usize,
    enter: bool,
    query: String,
    scroll: bool,
}

impl Open {
    fn new(slot: usize, highlight: usize, count: usize) -> Self {
        Self { slot, highlight, count, enter: false, query: String::new(), scroll: true }
    }
}

const MENU_SEARCH: f32 = 40.0;
const MENU_LIST_MAX: f32 = 240.0;

const HEADER_SLOT: usize = usize::MAX;
const LOADER_DELAY: f64 = 0.3;
const LOADER_FADE: f64 = 0.3;
const LOADER_SWEEP: f64 = 1.5;

pub enum Signal {
    Hotkey(String),
    Show,
    Quit,
    Settings,
}

struct Panel {
    title: String,
    groups: Vec<Vec<Action>>,
    selected: usize,
    filter: String,
    command: Option<String>,
}

struct Pending {
    confirmation: Confirmation,
    action: Action,
    command: Option<String>,
    ok_focused: bool,
}

#[derive(Clone, Copy, PartialEq)]
enum Source {
    Command(usize),
    Query(usize),
    Fallback(usize),
    Item,
}

#[derive(Clone)]
struct Row {
    id: String,
    section: String,
    title: String,
    subtitle: Option<String>,
    icon: Icon,
    accessories: Vec<Accessory>,
    kind: Option<&'static str>,
    source: Source,
    actions: Vec<Vec<Action>>,
    card: Option<Rc<Inline>>,
}

struct OpenView {
    view: Box<dyn View>,
    query: String,
    selected: usize,
    picked_initial: bool,
}

#[derive(PartialEq)]
enum Focus {
    Search,
    Argument,
}

pub struct Launcher {
    extensions: Vec<Box<dyn Extension>>,
    commands: Vec<Command>,
    generation: u64,
    store: Store,
    settings: Settings,
    frecency: Frecency,
    favorites: Vec<String>,
    query: String,
    selected: usize,
    argument: String,
    focus: Focus,
    views: Vec<OpenView>,
    scroll_to_selected: bool,
    keep_selected: Option<String>,
    panel: Option<Panel>,
    pending: Option<Pending>,
    toast: Option<(Toast, f64)>,
    hud: Option<(Toast, f64)>,
    hidden_at: f64,
    was_shown: bool,
    notes: Option<crate::notes::window::NotesWindow>,
    settings_window: Option<settings_ui::SettingsWindow>,
    data_dir: std::path::PathBuf,
    signals: Receiver<Signal>,
    hotkey_error: Option<String>,
    inline: Option<(String, u64, Option<Row>)>,
    cursor_to_end: bool,
    dropdown: Option<Open>,
    argument_for: String,
    weather_week: bool,
    loading_since: Option<f64>,
}

fn loader_frame(elapsed: f64) -> Option<(f32, f32)> {
    let shown = elapsed - LOADER_DELAY;
    if shown < 0.0 {
        return None;
    }
    let cycle = (shown % (2.0 * LOADER_SWEEP)) / LOADER_SWEEP;
    let t = if cycle > 1.0 { 2.0 - cycle } else { cycle };
    let eased = t * t * (3.0 - 2.0 * t);
    Some(((eased as f32 - 0.5) * 0.7, (shown / LOADER_FADE).min(1.0) as f32))
}

fn paint_loader(ui: &Ui, line: Rect, offset: f32, alpha: f32) {
    let (left, width) = (line.left() + offset * line.width(), line.width());
    let mut mesh = egui::Mesh::default();
    for (stop, color) in [(0.25, Color32::TRANSPARENT), (0.5, theme::FG().gamma_multiply(alpha)), (0.75, Color32::TRANSPARENT)] {
        mesh.colored_vertex(pos2(left + stop * width, line.top()), color);
        mesh.colored_vertex(pos2(left + stop * width, line.bottom()), color);
    }
    for i in 0..4 {
        mesh.add_triangle(i, i + 1, i + 2);
    }
    ui.painter().with_clip_rect(line).add(mesh);
}

fn text_galley(ui: &Ui, text: &str, font: FontId, color: Color32, max_width: f32) -> std::sync::Arc<egui::Galley> {
    let mut job = LayoutJob::single_section(crate::i18n::tr(text).into_owned(), TextFormat { font_id: font, color, ..Default::default() });
    job.wrap = TextWrapping::truncate_at_width(max_width.max(0.0));
    ui.ctx().fonts_mut(|f| f.layout_job(job))
}

pub fn wrapped(ui: &Ui, text: &str, font: FontId, color: Color32, max_width: f32) -> std::sync::Arc<egui::Galley> {
    let mut job = LayoutJob::single_section(crate::i18n::tr(text).into_owned(), TextFormat { font_id: font, color, ..Default::default() });
    job.wrap.max_width = max_width.max(1.0);
    ui.ctx().fonts_mut(|f| f.layout_job(job))
}

pub fn metadata(ui: &mut Ui, rect: Rect, rows: &[(String, String)]) -> Rect {
    let height = (12.0 + 16.0 + 8.0 + rows.len() as f32 * 33.0 + 8.0).min(148.0);
    let area = Rect::from_min_max(pos2(rect.left(), rect.bottom() - height), rect.max);
    ui.painter().hline(area.x_range(), area.top() + 0.5, Stroke::new(1.0, theme::SEPARATOR()));
    let inner = area.shrink2(vec2(16.0, 0.0));
    paint_text(ui, pos2(inner.left(), area.top() + 12.0), Align2::LEFT_TOP, "Information", theme::weighted(12.0, theme::SEMIBOLD), theme::FG_60(), inner.width());
    let list = Rect::from_min_max(pos2(area.left(), area.top() + 12.0 + 16.0 + 8.0), pos2(area.right(), area.bottom() - 8.0));
    let mut child = ui.new_child(UiBuilder::new().max_rect(list));
    child.spacing_mut().item_spacing = vec2(0.0, 0.0);
    egui::ScrollArea::vertical().id_salt("information").auto_shrink(false).show(&mut child, |ui| {
        for (i, (label, value)) in rows.iter().enumerate() {
            let (row, _) = ui.allocate_exact_size(vec2(ui.available_width(), 33.0), Sense::hover());
            let row = row.shrink2(vec2(16.0, 0.0));
            if i > 0 {
                ui.painter().hline(row.x_range(), row.top() + 0.5, Stroke::new(1.0, theme::SEPARATOR()));
            }
            let label_galley = text_galley(ui, label, theme::font(13.0), theme::FG_60(), row.width() / 2.0);
            let label_width = label_galley.size().x;
            ui.painter().galley(pos2(row.left(), row.center().y - label_galley.size().y / 2.0), label_galley, theme::FG_60());
            let value_galley = text_galley(ui, value, theme::weighted(13.0, theme::MEDIUM), theme::FG(), row.width() - label_width - 16.0);
            ui.painter().galley(pos2(row.right() - value_galley.size().x, row.center().y - value_galley.size().y / 2.0), value_galley, theme::FG());
        }
    });
    area
}

fn paint_text(ui: &Ui, anchor: Pos2, align: Align2, text: &str, font: FontId, color: Color32, max_width: f32) -> Rect {
    let galley = text_galley(ui, text, font, color, max_width);
    let rect = align.anchor_size(anchor, galley.size());
    ui.painter().galley(rect.min, galley, color);
    rect
}

fn keycaps(ui: &Ui, right: f32, center_y: f32, labels: &[String]) -> f32 {
    let mut x = right;
    for label in labels.iter().rev() {
        let galley = text_galley(ui, label, theme::font(11.0), theme::FG_60(), 200.0);
        let width = (galley.size().x + 10.0).max(18.0);
        let rect = Rect::from_min_size(pos2(x - width, center_y - 9.0), vec2(width, 18.0));
        ui.painter().rect_stroke(rect, 4.0, Stroke::new(1.0, theme::KEY_BORDER()), egui::StrokeKind::Inside);
        ui.painter().galley(rect.center() - galley.size() / 2.0, galley, theme::FG_60());
        x = rect.left() - 2.0;
    }
    x + 2.0
}

fn accessories(ui: &Ui, right: f32, center_y: f32, parts: &[Accessory]) -> f32 {
    let mut x = right;
    for part in parts.iter().rev() {
        match part {
            Accessory::Text(text) => x = paint_text(ui, pos2(x, center_y), Align2::RIGHT_CENTER, text, theme::font(12.0), theme::FG_60(), 200.0).left(),
            Accessory::Tinted(text, color) => x = paint_text(ui, pos2(x, center_y), Align2::RIGHT_CENTER, text, theme::font(12.0), *color, 200.0).left(),
            Accessory::Tag(text) => {
                let galley = text_galley(ui, text, theme::weighted(11.0, theme::MEDIUM), theme::ACCENT(), 200.0);
                let rect = Rect::from_min_size(pos2(x - galley.size().x - 12.0, center_y - 9.0), vec2(galley.size().x + 12.0, 18.0));
                ui.painter().rect_filled(rect, 4.0, theme::ACCENT().gamma_multiply(0.15));
                ui.painter().galley(rect.center() - galley.size() / 2.0, galley, theme::ACCENT());
                x = rect.left();
            }
            Accessory::Volume { level, muted, input } => {
                let glyph = match (input, muted) {
                    (false, false) => "volume-2",
                    (false, true) => "volume-x",
                    (true, false) => "mic",
                    (true, true) => "mic-off",
                };
                icons::paint_glyph(ui, glyph, Rect::from_min_size(pos2(x - 14.0, center_y - 7.0), vec2(14.0, 14.0)), theme::FG_60());
                let bar = Rect::from_min_size(pos2(x - 14.0 - 8.0 - 48.0, center_y - 2.0), vec2(48.0, 4.0));
                ui.painter().rect_filled(bar, 2.0, theme::CONTROL());
                ui.painter().rect_filled(Rect::from_min_size(bar.min, vec2(48.0 * level.clamp(0.0, 100.0) / 100.0, 4.0)), 2.0, theme::FG_60());
                let text = format!("{}%", level.round());
                x = paint_text(ui, pos2(bar.left() - 8.0, center_y), Align2::RIGHT_CENTER, &text, theme::font(12.0), theme::FG_60(), 40.0).left();
            }
        }
        x -= 8.0;
    }
    x + 8.0
}

static CONNECTOR: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| regex::Regex::new(r"(?i)\s+(?:in|to|as|para|em|como|=)\s+").expect("connector pattern"));

fn card_side(ui: &Ui, rect: Rect, side: &Side, input: bool) {
    let chip_text = text_galley(ui, &side.label, theme::weighted(12.0, theme::MEDIUM), theme::FG(), rect.width() - 44.0);
    let chip = Rect::from_min_size(pos2(rect.center().x - chip_text.size().x / 2.0 - 6.0, rect.bottom() - 13.0 - chip_text.size().y - 2.0), chip_text.size() + vec2(12.0, 2.0));
    ui.painter().rect_filled(chip, 4.0, theme::CONTROL());
    ui.painter().galley(chip.min + vec2(6.0, 1.0), chip_text, theme::FG());
    let area = Rect::from_min_max(pos2(rect.left() + 16.0, rect.top() + 4.0), pos2(rect.right() - 16.0, chip.top()));
    if let Some(color) = side.swatch {
        let swatch = Rect::from_center_size(area.center(), vec2(98.0, 72.0));
        ui.painter().rect_filled(swatch, 6.0, color);
        ui.painter().rect_stroke(swatch, 6.0, Stroke::new(2.0, theme::FG()), egui::StrokeKind::Outside);
        return;
    }
    let font = theme::weighted(24.0, theme::SEMIBOLD);
    let format = |color| TextFormat { font_id: font.clone(), color, ..Default::default() };
    let mut job = LayoutJob::default();
    let mut last = 0;
    if input {
        let ranges: Vec<(usize, usize)> = match &side.dim {
            Some(ranges) => ranges.iter().copied().filter(|(a, b)| a < b && *b <= side.text.len() && side.text.is_char_boundary(*a) && side.text.is_char_boundary(*b)).collect(),
            None => CONNECTOR.find_iter(&side.text).map(|m| (m.start(), m.end())).collect(),
        };
        for (start, end) in ranges {
            if start < last {
                continue;
            }
            job.append(&side.text[last..start], 0.0, format(theme::FG()));
            job.append(&side.text[start..end], 0.0, format(theme::FG_40()));
            last = end;
        }
    }
    job.append(&side.text[last..], 0.0, format(theme::FG()));
    job.wrap = TextWrapping::truncate_at_width(area.width());
    let galley = ui.ctx().fonts_mut(|f| f.layout_job(job));
    ui.painter().galley(area.center() - galley.size() / 2.0, galley, theme::FG());
}

fn emoji(ui: &Ui, text: &str, center: Pos2, size: f32) {
    let px = (size * ui.ctx().pixels_per_point()).round() as u32;
    egui::Image::new(format!("emoji://{px}/{text}")).paint_at(ui, Rect::from_center_size(center, vec2(size, size)));
}

fn paint_weather(ui: &Ui, rect: Rect, card: &crate::extension::WeatherCard, week: bool) {
    let left = Rect::from_min_size(rect.min, vec2(rect.width() * 0.255, rect.height()));
    let x = left.left() + 24.0;
    let temperature = paint_text(ui, pos2(x, left.top() + 22.0), Align2::LEFT_TOP, &card.temperature, theme::weighted(24.0, theme::SEMIBOLD), theme::FG(), 90.0);
    emoji(ui, &card.icon, pos2(temperature.right() + 13.0, temperature.center().y + 1.0), 20.0);
    let width = left.right() - x - 4.0;
    let place = paint_text(ui, pos2(x, temperature.bottom() + 6.0), Align2::LEFT_TOP, &card.place, theme::weighted(13.0, theme::SEMIBOLD), theme::FG(), width);
    let condition = paint_text(ui, pos2(x, place.bottom() + 3.0), Align2::LEFT_TOP, &card.condition, theme::font(11.0), theme::FG_60(), width);
    paint_text(ui, pos2(x, condition.bottom() + 3.0), Align2::LEFT_TOP, &card.range, theme::font(11.0), theme::FG_40(), width);
    let list = if week { &card.week } else { &card.today };
    let columns = Rect::from_min_max(pos2(left.right() + 8.0, rect.top()), pos2(rect.right() - 8.0, rect.bottom()));
    let step = columns.width() / list.len().max(1) as f32;
    for (i, forecast) in list.iter().enumerate() {
        let cx = columns.left() + step * (i as f32 + 0.5);
        paint_text(ui, pos2(cx, rect.top() + 34.0), Align2::CENTER_CENTER, &forecast.label, theme::weighted(13.0, if i == 0 { theme::SEMIBOLD } else { theme::MEDIUM }), if i == 0 { theme::FG() } else { theme::FG_60() }, step - 4.0);
        emoji(ui, &forecast.icon, pos2(cx, rect.top() + 61.0), 20.0);
        match &forecast.low {
            Some(low) => {
                let high = text_galley(ui, &forecast.high, theme::weighted(13.0, theme::SEMIBOLD), theme::FG(), step);
                let low = text_galley(ui, low, theme::weighted(13.0, theme::SEMIBOLD), theme::FG(), step);
                let total = high.size().x + 4.0 + low.size().x;
                let y = rect.top() + 89.0 - high.size().y / 2.0;
                ui.painter().galley(pos2(cx - total / 2.0, y), high.clone(), theme::FG());
                ui.painter().galley(pos2(cx - total / 2.0 + high.size().x + 4.0, y), low, theme::FG());
            }
            None => {
                paint_text(ui, pos2(cx, rect.top() + 89.0), Align2::CENTER_CENTER, &forecast.high, theme::weighted(13.0, theme::SEMIBOLD), theme::FG(), step);
            }
        }
    }
}

fn chip(ui: &Ui, center: Pos2, text: &str, color: Color32, background: Color32) -> Rect {
    let galley = text_galley(ui, text, theme::weighted(12.0, theme::MEDIUM), color, 200.0);
    let rect = Rect::from_center_size(center, galley.size() + vec2(12.0, 4.0));
    ui.painter().rect_filled(rect, 4.0, background);
    ui.painter().galley(rect.center() - galley.size() / 2.0, galley, color);
    rect
}

fn paint_flight(ui: &Ui, rect: Rect, card: &crate::extension::FlightCard) {
    let half = rect.width() / 2.0;
    for (i, (name, detail, code)) in [&card.from, &card.to].into_iter().enumerate() {
        let side = Rect::from_min_size(pos2(rect.left() + half * i as f32, rect.top()), vec2(half, rect.height()));
        let width = side.width() - 48.0;
        paint_text(ui, pos2(side.center().x, side.top() + 41.0), Align2::CENTER_CENTER, name, theme::weighted(24.0, theme::SEMIBOLD), theme::FG(), width);
        paint_text(ui, pos2(side.center().x, side.top() + 66.0), Align2::CENTER_CENTER, detail, theme::font(11.0), theme::FG_60(), width);
        chip(ui, pos2(side.center().x, side.bottom() - 21.0), code, theme::FG(), theme::CONTROL());
    }
    let x = rect.center().x;
    for range in [rect.top()..=rect.center().y - 30.0, rect.center().y + 34.0..=rect.bottom()] {
        ui.painter().vline(x, range, Stroke::new(1.0, theme::SEPARATOR()));
    }
    let plane = Rect::from_center_size(pos2(x, rect.center().y - 15.0), vec2(22.0, 22.0));
    egui::Image::from_bytes("bytes://flight-plane.svg", PLANE_SVG.as_bytes()).tint(theme::FG()).paint_at(ui, plane);
    let (color, background) = if card.live { (theme::GREEN(), Color32::TRANSPARENT) } else { (theme::FG_60(), Color32::TRANSPARENT) };
    let status = chip(ui, pos2(x, rect.center().y + 14.0), &card.status, color, background);
    if let Some(note) = &card.note {
        chip(ui, pos2(status.right() + 8.0 + 30.0, status.center().y), note, Color32::from_rgb(0xFF, 0x9E, 0x4F), Color32::from_rgba_unmultiplied(0xFF, 0x9E, 0x4F, 38));
    }
}

const PLANE_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="white"><path d="M21 16v-2l-8-5V3.5A1.5 1.5 0 0 0 11.5 2 1.5 1.5 0 0 0 10 3.5V9l-8 5v2l8-2.5V19l-2 1.5V22l3.5-1 3.5 1v-1.5L13 19v-5.5l8 2.5z" transform="rotate(90 12 12)"/></svg>"#;

fn pulse(ui: &Ui) -> Color32 {
    let t = ui.input(|i| i.time) as f32;
    ui.ctx().request_repaint();
    theme::fg((10.0 + 8.0 * (0.5 + 0.5 * (t * std::f32::consts::TAU / 1.4).sin())).round() as u8)
}

fn paint_skeleton(ui: &Ui, rect: Rect) {
    let fill = pulse(ui);
    let bar = |x: f32, y: f32, w: f32, h: f32| ui.painter().rect_filled(Rect::from_min_size(pos2(x, y), vec2(w, h)), 2.0, fill);
    let x = rect.left() + 24.0;
    bar(x, rect.top() + 26.0, 64.0, 16.0);
    bar(x, rect.top() + 56.0, 120.0, 8.0);
    bar(x, rect.top() + 75.0, 84.0, 8.0);
    bar(x, rect.top() + 92.0, 68.0, 8.0);
    let columns = Rect::from_min_max(pos2(rect.left() + rect.width() * 0.255 + 8.0, rect.top()), pos2(rect.right() - 8.0, rect.bottom()));
    let step = columns.width() / 8.0;
    for i in 0..8 {
        let cx = columns.left() + step * (i as f32 + 0.5);
        bar(cx - 14.0, rect.top() + 29.0, 28.0, 10.0);
        bar(cx - 8.0, rect.top() + 53.0, 16.0, 16.0);
        bar(cx - 12.0, rect.top() + 85.0, 24.0, 8.0);
    }
}

fn paint_card(ui: &Ui, rect: Rect, card: &Inline, selected: bool, week: bool) {
    ui.painter().rect_filled(rect, 8.0, if selected { theme::SELECTION() } else { theme::fg(5) });
    match &card.kind {
        CardKind::Weather(weather) => return paint_weather(ui, rect, weather, week),
        CardKind::Flight(flight) => return paint_flight(ui, rect, flight),
        CardKind::Loading => return paint_skeleton(ui, rect),
        CardKind::Pair | CardKind::LoadingPair => {}
    }
    let half = rect.width() / 2.0;
    card_side(ui, Rect::from_min_size(rect.min, vec2(half, rect.height())), &card.input, true);
    let output = Rect::from_min_size(pos2(rect.left() + half, rect.top()), vec2(half, rect.height()));
    if card.kind == CardKind::LoadingPair {
        let fill = pulse(ui);
        ui.painter().rect_filled(Rect::from_center_size(output.center(), vec2(120.0, 20.0)), 3.0, fill);
        ui.painter().rect_filled(Rect::from_center_size(pos2(output.center().x, output.bottom() - 21.0), vec2(64.0, 14.0)), 3.0, fill);
    } else {
        card_side(ui, output, &card.output, false);
    }
    let x = rect.center().x;
    for range in [rect.top()..=rect.center().y - 12.0, rect.center().y + 12.0..=rect.bottom()] {
        ui.painter().vline(x, range, Stroke::new(1.0, theme::SEPARATOR()));
    }
    let arrow = paint_text(ui, rect.center(), Align2::CENTER_CENTER, "→", theme::font(15.0), theme::FG(), 40.0);
    if let Some(note) = &card.note {
        paint_text(ui, pos2(x, arrow.bottom() + 4.0), Align2::CENTER_TOP, note, theme::font(11.0), theme::FG_60(), half * 0.92);
    }
}

fn key_presses(ui: &Ui) -> Vec<(Key, Modifiers)> {
    ui.input(|i| {
        i.events
            .iter()
            .filter_map(|e| match e {
                egui::Event::Key { key, pressed: true, modifiers, .. } => Some((*key, *modifiers)),
                _ => None,
            })
            .collect()
    })
}

fn consume(ui: &Ui, key: Key, modifiers: Modifiers) {
    ui.input_mut(|i| i.events.retain(|e| !matches!(e, egui::Event::Key { key: k, pressed: true, modifiers: m, .. } if *k == key && *m == modifiers)));
}

fn plain(modifiers: Modifiers) -> bool {
    !modifiers.ctrl && !modifiers.alt && !modifiers.shift
}

fn section_ranges(rows: &[Row]) -> Vec<std::ops::Range<usize>> {
    let mut ranges: Vec<std::ops::Range<usize>> = Vec::new();
    for (i, row) in rows.iter().enumerate() {
        match ranges.last_mut() {
            Some(last) if rows[last.start].section == row.section => last.end = i + 1,
            _ => ranges.push(i..i + 1),
        }
    }
    ranges
}

fn grid_lines(rows: &[Row], columns: usize) -> Vec<std::ops::Range<usize>> {
    let columns = columns.max(1);
    section_ranges(rows).into_iter().flat_map(|r| r.clone().step_by(columns).map(move |start| start..(start + columns).min(r.end))).collect()
}

fn grid_move(lines: &[std::ops::Range<usize>], selected: usize, step: isize) -> usize {
    let Some(line) = lines.iter().position(|l| l.contains(&selected)) else { return selected };
    match line.checked_add_signed(step).and_then(|i| lines.get(i)) {
        Some(target) => target.start + (selected - lines[line].start).min(target.len() - 1),
        None => selected,
    }
}

fn placeholder_owned(views: &[OpenView]) -> String {
    crate::i18n::tr(&views.last().map_or_else(|| "Search for apps or commands...".to_string(), |v| v.view.placeholder())).into_owned()
}

#[derive(Clone, Default)]
struct TokenState {
    query: String,
    active: usize,
    list: Option<Rect>,
}

fn token_query_open(ui: &Ui) -> bool {
    ui.memory(|m| m.focused()).and_then(|id| ui.data(|d| d.get_temp::<TokenState>(id.with("tokens")))).is_some_and(|s| !s.query.is_empty())
}

fn tokens_field(ui: &mut Ui, origin: Pos2, field: &mut crate::extension::Field, spec: &crate::extension::Tokens, id: egui::Id) -> (f32, bool) {
    const WIDTH: f32 = 360.0;
    const CHIP: f32 = 24.0;
    let state_id = id.with("tokens");
    let mut state: TokenState = ui.data(|d| d.get_temp(state_id)).unwrap_or_default();
    let mut values: Vec<String> = field.value.lines().filter(|l| !l.is_empty()).map(String::from).collect();
    let mut edited = false;
    let focused = ui.memory(|m| m.has_focus(id));
    let query = state.query.trim().to_lowercase();
    let mut matches: Vec<(String, String, Icon)> = Vec::new();
    if let Some((prefix, create)) = &spec.create {
        if let Some(value) = create(&state.query).filter(|v| !values.contains(v)) {
            let (label, icon) = (spec.describe)(&value);
            matches.push((value, format!("{prefix}: {label}"), icon));
        }
    }
    matches.extend(spec.options.iter().filter(|(v, label, _)| !values.contains(v) && (query.is_empty() || label.to_lowercase().contains(&query))).take(50).cloned());
    state.active = state.active.min(matches.len().saturating_sub(1));
    let mut moved = false;
    if focused {
        let (down, up, enter, back, escape) = ui.input_mut(|i| {
            (
                i.consume_key(Modifiers::NONE, Key::ArrowDown),
                i.consume_key(Modifiers::NONE, Key::ArrowUp),
                i.consume_key(Modifiers::NONE, Key::Enter),
                state.query.is_empty() && i.consume_key(Modifiers::NONE, Key::Backspace),
                !state.query.is_empty() && i.consume_key(Modifiers::NONE, Key::Escape),
            )
        });
        if down {
            state.active = (state.active + 1).min(matches.len().saturating_sub(1));
            moved = true;
        }
        if up {
            state.active = state.active.saturating_sub(1);
            moved = true;
        }
        if enter {
            if let Some((value, _, _)) = matches.get(state.active) {
                values.push(value.clone());
                state.query.clear();
                state.active = 0;
                edited = true;
            }
        }
        if back && values.pop().is_some() {
            edited = true;
        }
        if escape {
            state.query.clear();
            state.active = 0;
        }
    }
    let font = theme::font(12.0);
    let (left, right) = (origin.x + 4.0, origin.x + WIDTH - 4.0);
    let (mut x, mut y) = (left, origin.y + 4.0);
    let mut chips = Vec::new();
    for value in &values {
        let (label, icon) = spec.options.iter().find(|(v, _, _)| v == value).map(|(_, l, i)| (l.clone(), i.clone())).unwrap_or_else(|| (spec.describe)(value));
        let text = text_galley(ui, &label, font.clone(), theme::FG(), right - left - 6.0 - 22.0 - 24.0);
        let width = (6.0 + 22.0 + text.size().x + 4.0 + 20.0 + 2.0).min(right - left);
        if x + width > right && x > left {
            x = left;
            y += CHIP + 4.0;
        }
        chips.push((value.clone(), Rect::from_min_size(pos2(x, y), vec2(width, CHIP)), icon, text, label));
        x += width + 4.0;
    }
    if x + 140.0 > right && x > left {
        x = left;
        y += CHIP + 4.0;
    }
    let input = Rect::from_min_max(pos2(x, y), pos2(right, y + CHIP));
    let frame = Rect::from_min_max(origin, pos2(origin.x + WIDTH, (input.bottom() + 4.0).max(origin.y + 34.0)));
    let border = if focused { theme::FG_40() } else { theme::KEY_BORDER() };
    ui.painter().rect(frame, 8.0, theme::fg(3), Stroke::new(1.0, border), egui::StrokeKind::Inside);
    let mut removed = None;
    for (value, chip, icon, text, label) in chips {
        ui.painter().rect_filled(chip, 5.0, theme::CONTROL());
        let icon_rect = Rect::from_min_size(pos2(chip.left() + 6.0, chip.center().y - 8.0), vec2(16.0, 16.0));
        icons::command_icon(&mut ui.new_child(UiBuilder::new().max_rect(icon_rect)), &icon, &label, 16.0);
        ui.painter().galley(pos2(icon_rect.right() + 6.0, chip.center().y - text.size().y / 2.0), text, theme::FG());
        let close = Rect::from_min_size(pos2(chip.right() - 22.0, chip.center().y - 10.0), vec2(20.0, 20.0));
        let response = ui.interact(close, id.with(("remove", &value)), Sense::CLICK);
        response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, format!("Remove {label}")));
        if response.hovered() {
            ui.painter().rect_filled(close, 4.0, theme::CONTROL_HOVER());
        }
        icons::paint_glyph(ui, "x", Rect::from_center_size(close.center(), vec2(12.0, 12.0)), if response.hovered() { theme::FG() } else { theme::FG_60() });
        if response.clicked() {
            removed = Some(value);
        }
    }
    if let Some(value) = removed {
        values.retain(|v| *v != value);
        edited = true;
        ui.memory_mut(|m| m.request_focus(id));
    }
    let hint = egui::RichText::new(if values.is_empty() { field.placeholder.as_str() } else { "" }).color(theme::FG_40()).font(theme::font(13.0));
    let inner = input.shrink2(vec2(6.0, 4.0));
    let before = state.query.clone();
    let response = ui.put(inner, egui::TextEdit::singleline(&mut state.query).id(id).frame(egui::Frame::NONE).margin(egui::Margin::ZERO).font(theme::font(13.0)).text_color(theme::FG()).hint_text(hint).desired_width(inner.width()).return_key(None));
    response.widget_info(|| egui::WidgetInfo::text_edit(true, field.label, &state.query, ""));
    if state.query != before {
        state.active = 0;
        field.error = None;
    }
    if response.gained_focus() {
        response.scroll_to_me(None);
    }
    let pressing_list = state.list.is_some_and(|r| ui.input(|i| i.pointer.any_down() || i.pointer.any_released()) && ui.input(|i| i.pointer.interact_pos()).is_some_and(|p| r.contains(p)));
    let mut bottom = frame.bottom();
    state.list = None;
    if (response.has_focus() || pressing_list) && !matches.is_empty() {
        let rows = matches.len().min(5) as f32;
        let list = Rect::from_min_size(pos2(origin.x, frame.bottom() + 4.0), vec2(WIDTH, rows * 32.0 + 8.0));
        ui.painter().rect(list, 8.0, theme::PANEL(), Stroke::new(1.0, theme::PANEL_BORDER()), egui::StrokeKind::Inside);
        let mut picked = None;
        let mut list_ui = ui.new_child(UiBuilder::new().max_rect(list.shrink(4.0)));
        egui::ScrollArea::vertical().id_salt(id.with("list")).max_height(list.height() - 8.0).auto_shrink([false, true]).show(&mut list_ui, |ui| {
            for (n, (value, label, icon)) in matches.iter().enumerate() {
                let (row, row_response) = ui.allocate_exact_size(vec2(ui.available_width(), 32.0), Sense::CLICK);
                if row_response.hovered() && ui.input(|i| i.pointer.delta() != egui::Vec2::ZERO) {
                    state.active = n;
                }
                if n == state.active {
                    ui.painter().rect_filled(row, 6.0, theme::SELECTION());
                    if moved {
                        ui.scroll_to_rect(row, None);
                    }
                }
                let icon_rect = Rect::from_min_size(pos2(row.left() + 8.0, row.center().y - 8.0), vec2(16.0, 16.0));
                icons::command_icon(&mut ui.new_child(UiBuilder::new().max_rect(icon_rect)), icon, label, 16.0);
                paint_text(ui, pos2(icon_rect.right() + 8.0, row.center().y), Align2::LEFT_CENTER, label, theme::font(13.0), theme::FG(), row.right() - icon_rect.right() - 16.0);
                if row_response.clicked() {
                    picked = Some(value.clone());
                }
            }
        });
        if let Some(value) = picked {
            values.push(value);
            state.query.clear();
            state.active = 0;
            edited = true;
            ui.memory_mut(|m| m.request_focus(id));
        }
        state.list = Some(list);
        bottom = list.bottom();
    }
    if edited {
        field.value = values.join("\n");
        field.error = None;
    }
    ui.data_mut(|d| d.insert_temp(state_id, state));
    (bottom, edited)
}

fn back_button(ui: &Ui, rect: Rect) -> bool {
    let response = ui.interact(rect, egui::Id::unique("view-back"), Sense::click());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, "Back"));
    ui.painter().rect_filled(rect, 5.0, if response.hovered() { theme::CONTROL_HOVER() } else { theme::CONTROL() });
    icons::paint_glyph(ui, "arrow-left", Rect::from_center_size(rect.center(), vec2(14.0, 14.0)), theme::FG());
    response.clicked()
}

impl Launcher {
    pub fn new(ctx: &egui::Context, data_dir: std::path::PathBuf, signals: Receiver<Signal>, hotkey_error: Option<String>) -> Self {
        let store = Store::load(data_dir.clone());
        let settings = store.settings();
        crate::TRAY_VISIBLE.store(settings.show_in_tray, std::sync::atomic::Ordering::SeqCst);
        let frecency = store.scores(crate::store::now_ms());
        let favorites = store.favorites();
        let extensions = crate::extensions::built_in();
        crate::extensions::clipboard_history::start(ctx);
        crate::extensions::snippets::start();
        crate::extensions::rescan_apps(ctx);
        let mut launcher = Self {
            commands: Vec::new(),
            extensions,
            generation: u64::MAX,
            store,
            settings,
            frecency,
            favorites,
            query: String::new(),
            selected: 0,
            argument: String::new(),
            focus: Focus::Search,
            views: Vec::new(),
            scroll_to_selected: false,
            keep_selected: None,
            panel: None,
            pending: None,
            toast: None,
            hud: None,
            hidden_at: f64::NEG_INFINITY,
            was_shown: false,
            notes: None,
            settings_window: None,
            data_dir,
            signals,
            hotkey_error,
            inline: None,
            weather_week: false,
            loading_since: None,
            cursor_to_end: false,
            dropdown: None,
            argument_for: String::new(),
        };
        launcher.reload();
        launcher
    }

    fn reload(&mut self) {
        let generation = crate::extensions::GENERATION.load(std::sync::atomic::Ordering::SeqCst);
        if generation == self.generation {
            return;
        }
        self.generation = generation;
        self.extensions.retain(|e| !e.id().starts_with("flowkey."));
        self.extensions.extend(crate::extensions::flowkey::extensions());
        self.commands = crate::extensions::load(&self.extensions, &self.settings);
        self.apply_hotkeys();
    }

    fn apply_hotkeys(&mut self) {
        let mut pairs = vec![("launcher".to_string(), self.settings.hotkey.clone())];
        let mut bound: Vec<_> = self.settings.commands.iter().filter(|(key, p)| !p.hotkey.trim().is_empty() && self.commands.iter().any(|c| &c.key == *key)).map(|(k, p)| (k.clone(), p.hotkey.clone())).collect();
        bound.sort();
        pairs.extend(bound);
        match crate::hotkey::bindings(&pairs) {
            Ok(bindings) => crate::hotkey::set_bindings(bindings),
            Err(e) => self.hotkey_error = Some(e),
        }
    }

    fn aliases(&self) -> HashMap<String, String> {
        self.settings.commands.iter().filter(|(_, p)| !p.alias.is_empty()).map(|(k, p)| (k.clone(), p.alias.clone())).collect()
    }

    fn command_row(&self, section: &str, index: usize, source: Source, subtitle: Option<String>) -> Row {
        let command = &self.commands[index];
        Row {
            id: command.key.clone(),
            section: section.into(),
            title: command.title.clone(),
            subtitle: subtitle.or_else(|| command.subtitle.clone()),
            icon: command.icon.clone(),
            accessories: Vec::new(),
            kind: Some(command.kind),
            source,
            actions: Vec::new(),
            card: None,
        }
    }

    fn inline_row(&mut self, ctx: &egui::Context, text: &str) -> Option<Row> {
        let version = crate::extensions::calculator::RATES_VERSION.load(std::sync::atomic::Ordering::SeqCst);
        if let Some((query, at, row)) = &self.inline {
            if query == text && *at == version {
                return row.clone();
            }
        }
        let disabled = |key: &str| self.settings.commands.get(key).is_some_and(|p| p.disabled);
        let row = self.extensions.iter().filter(|e| !disabled(e.id())).find_map(|e| e.inline(ctx, text)).map(|inline| Row {
            id: "inline".into(),
            section: inline.section.clone(),
            title: if inline.output.text.is_empty() { inline.output.label.clone() } else { inline.output.text.clone() },
            subtitle: None,
            icon: Icon::Glyph("calculator"),
            accessories: Vec::new(),
            kind: None,
            source: Source::Item,
            actions: inline.actions.clone(),
            card: Some(Rc::new(inline)),
        });
        self.inline = Some((text.to_owned(), version, row.clone()));
        row
    }

    fn rows(&mut self, ctx: &egui::Context) -> Vec<Row> {
        if let Some(top) = self.views.last_mut() {
            let query = top.query.clone();
            top.view.set_query(&query);
            let filters = top.view.filters();
            return top
                .view
                .sections()
                .into_iter()
                .flat_map(|section| {
                    let title = section.title;
                    let query = query.clone();
                    section.items.into_iter().filter(move |item| !filters || query.trim().is_empty() || match_score(&query, &item.title, &item.keywords) > 0).map(move |item| Row {
                        id: item.id,
                        section: title.clone(),
                        title: item.title,
                        subtitle: item.subtitle,
                        icon: item.icon,
                        accessories: item.accessories,
                        kind: None,
                        source: Source::Item,
                        actions: item.actions,
                        card: None,
                    })
                })
                .collect();
        }
        let aliases = self.aliases();
        let searchable: Vec<Command> = self.commands.iter().filter(|c| c.query.is_none()).cloned().collect();
        let index_of = |key: &str| self.commands.iter().position(|c| c.key == key).unwrap_or(0);
        let mut rows: Vec<Row> = search(&searchable, &self.query, &self.frecency, &self.favorites, &aliases)
            .into_iter()
            .flat_map(|section| section.items.into_iter().map(move |c| (section.title, c.key.clone())))
            .map(|(title, key)| self.command_row(title, index_of(&key), Source::Command(index_of(&key)), None))
            .collect();
        let text = self.query.trim().to_owned();
        if text.is_empty() {
            return rows;
        }
        let matched: Vec<Row> = self.commands.iter().enumerate().filter(|(_, c)| c.query.as_ref().is_some_and(|q| (q.matches)(&text))).map(|(i, _)| self.command_row("Results", i, Source::Query(i), Some(text.clone()))).collect();
        rows.splice(0..0, matched);
        if let Some(row) = self.inline_row(ctx, &text) {
            rows.insert(0, row);
        }
        let section = format!("Use “{text}” with...");
        let fallbacks: Vec<Row> = self.commands.iter().enumerate().filter(|(_, c)| c.fallback.is_some()).map(|(i, _)| self.command_row(&section, i, Source::Fallback(i), None)).collect();
        rows.extend(fallbacks);
        rows
    }

    fn command_groups(&self, index: usize) -> Vec<Vec<Action>> {
        let command = &self.commands[index];
        let mut own = command.actions.clone();
        let first = if own.is_empty() { Vec::new() } else { own.remove(0) };
        let mut primary = self.primary_of(index).icon("square-menu").shortcut(Shortcut::key(Key::Enter));
        primary.confirm = command.confirm.clone();
        let key = command.key.clone();
        let favorites = self.favorites.clone();
        let set = |next: Vec<String>| move |host: &mut Host| {
            host.requests.push(Request::SetFavorites(next.clone()));
            Ok(())
        };
        let mut favorite_actions = Vec::new();
        match favorites.iter().position(|k| *k == key) {
            None => favorite_actions.push(Action::new("add-favorite", "Add to Favorites", set([favorites.clone(), vec![key.clone()]].concat())).icon("star").shortcut(Shortcut::key(Key::F).ctrl().shift()).keep_open()),
            Some(i) => {
                favorite_actions.push(Action::new("remove-favorite", "Remove from Favorites", set(favorites.iter().filter(|k| **k != key).cloned().collect())).icon("star-off").shortcut(Shortcut::key(Key::F).ctrl().shift()).keep_open());
                let swapped = |j: usize| {
                    let mut next = favorites.clone();
                    next.swap(i, j);
                    next
                };
                if i > 0 {
                    favorite_actions.push(Action::new("move-favorite-up", "Move Favorite Up", set(swapped(i - 1))).icon("arrow-up").shortcut(Shortcut::key(Key::ArrowUp).ctrl().alt()).keep_open());
                }
                if i + 1 < favorites.len() {
                    favorite_actions.push(Action::new("move-favorite-down", "Move Favorite Down", set(swapped(i + 1))).icon("arrow-down").shortcut(Shortcut::key(Key::ArrowDown).ctrl().alt()).keep_open());
                }
            }
        }
        let mut ranking = Vec::new();
        if self.frecency.get(&key).copied().unwrap_or(0.0) > 0.0 {
            ranking.push(Action::new("reset-ranking", "Reset Ranking", move |host| {
                host.requests.push(Request::ResetRanking(key.clone()));
                Ok(())
            })
            .icon("rotate-ccw")
            .keep_open());
        }
        [vec![[vec![primary], first].concat(), favorite_actions], own, vec![ranking]].concat().into_iter().filter(|g| !g.is_empty()).collect()
    }

    fn primary_of(&self, index: usize) -> Action {
        let command = &self.commands[index];
        let title = command.primary_action_title;
        if let Some(view) = &command.view {
            let view = view.clone();
            return Action::new("primary", title, move |host| {
                host.requests.push(Request::Push(view.clone()));
                Ok(())
            })
            .keep_open();
        }
        if let Some(argument) = &command.argument {
            let (argument, value) = (argument.clone(), self.argument.clone());
            return Action::new("primary", title, move |host| {
                if value.trim().is_empty() && !argument.optional {
                    host.requests.push(Request::FocusArgument);
                    return Ok(());
                }
                (argument.run)(&value, host)
            })
            .keep_open();
        }
        let run = command.run.clone();
        Action { confirm: command.confirm.clone(), ..Action::new("primary", title, move |host| run(host)) }
    }

    fn groups(&self, row: &Row) -> Vec<Vec<Action>> {
        let text = self.query.trim().to_owned();
        match row.source {
            Source::Command(i) => self.command_groups(i),
            Source::Query(i) => {
                let run = self.commands[i].query.as_ref().map(|q| q.run.clone()).expect("query command");
                vec![vec![Action::new("primary", self.commands[i].primary_action_title, move |host| run(&text, host)).shortcut(Shortcut::key(Key::Enter))]]
            }
            Source::Fallback(i) => {
                let run = self.commands[i].fallback.clone().expect("fallback command");
                vec![vec![Action::new("primary", self.commands[i].primary_action_title, move |host| run(&text, host)).shortcut(Shortcut::key(Key::Enter))]]
            }
            Source::Item => row.actions.iter().filter(|g| !g.is_empty()).cloned().collect(),
        }
    }

    fn row_key(row: &Row) -> Option<String> {
        matches!(row.source, Source::Command(_)).then(|| row.id.clone())
    }

    fn pop_to_root(&mut self) {
        self.query.clear();
        self.selected = 0;
        self.panel = None;
        self.views.clear();
        self.focus = Focus::Search;
    }

    fn hide(&mut self, ctx: &egui::Context, restore_focus: bool) {
        if launcher::hide(restore_focus) {
            self.hidden_at = ctx.input(|i| i.time);
            self.panel = None;
            self.pending = None;
            self.frecency = self.store.scores(crate::store::now_ms());
            crate::extensions::rescan_apps(ctx);
        }
    }

    fn show(&mut self, ctx: &egui::Context) {
        if !launcher::is_shown() && ctx.input(|i| i.time) - self.hidden_at > POP_TO_ROOT_AFTER {
            self.pop_to_root();
        }
        launcher::show(self.settings.show_on);
        ctx.request_repaint();
    }

    fn fail(&mut self, ctx: &egui::Context, title: String, error: String) {
        self.toast = Some((Toast::Failure(title, error), ctx.input(|i| i.time)));
    }

    fn visit(&mut self, key: &str) {
        match self.store.visit(key, crate::store::now_ms()) {
            Ok(score) => {
                self.frecency.insert(key.to_owned(), score);
            }
            Err(e) => eprintln!("openray: frecency_visit failed: {e}"),
        }
    }

    fn run(&mut self, ctx: &egui::Context, action: Action, command: Option<String>, confirmed: bool) {
        if let Some(items) = &action.submenu {
            self.panel = Some(Panel { title: action.title.clone(), groups: vec![items()], selected: 0, filter: String::new(), command });
            return;
        }
        self.panel = None;
        if let (Some(confirmation), false) = (action.confirm.clone(), confirmed) {
            self.pending = Some(Pending { confirmation, action, command, ok_focused: false });
            return;
        }
        let in_view = !self.views.is_empty();
        let mut host = Host { ctx: ctx.clone(), toast: None, requests: Vec::new() };
        let result = (action.run)(&mut host);
        if let Some(toast) = host.toast {
            self.toast = Some((toast, ctx.input(|i| i.time)));
        }
        let pushed = host.requests.iter().any(|r| matches!(r, Request::Push(_)));
        for request in host.requests {
            self.handle(ctx, request, command.as_deref());
        }
        if let Err(error) = result {
            let title = match (&command, action.id) {
                (Some(key), "primary") => format!("Could not open {}", self.commands.iter().find(|c| &c.key == key).map_or("command", |c| c.title.as_str())),
                _ => format!("{} failed", action.title.trim_end_matches('…')),
            };
            self.fail(ctx, title, error);
            return;
        }
        if action.keep_open || pushed {
            return;
        }
        if let Some(key) = command {
            self.visit(&key);
        }
        if !in_view {
            self.pop_to_root();
        }
        self.hide(ctx, true);
    }

    fn handle(&mut self, ctx: &egui::Context, request: Request, command: Option<&str>) {
        match request {
            Request::Quit => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            Request::OpenSettings => self.open_settings(ctx),
            Request::OpenNotes { create } => {
                let mut notes = crate::notes::window::NotesWindow::open(self.data_dir.clone());
                if create {
                    notes.create(ctx);
                }
                self.notes = Some(notes);
            }
            Request::SetFavorites(keys) => {
                self.keep_selected = command.map(str::to_owned);
                match self.store.set_favorites(keys.clone()) {
                    Ok(()) => self.favorites = keys,
                    Err(e) => self.fail(ctx, "Could not save favorites".into(), e.to_string()),
                }
            }
            Request::ResetRanking(key) => {
                self.keep_selected = Some(key.clone());
                self.frecency.remove(&key);
                if let Err(e) = self.store.reset(&key) {
                    self.fail(ctx, "Could not reset ranking".into(), e.to_string());
                }
            }
            Request::Hide(restore_focus) => self.hide(ctx, restore_focus),
            Request::Hud(toast) => self.hud = Some((toast, ctx.input(|i| i.time))),
            Request::Push(factory) => self.push(ctx, &factory, command),
            Request::FocusArgument => self.focus = Focus::Argument,
            Request::SetQuery(text) => {
                match self.views.last_mut() {
                    Some(view) => view.query = text,
                    None => self.query = text,
                }
                self.cursor_to_end = true;
                self.focus = Focus::Search;
            }
        }
    }

    fn handle_async(&mut self, ctx: &egui::Context, request: Async) {
        match request {
            Async::Show => self.show(ctx),
            Async::Hide(restore_focus) => self.hide(ctx, restore_focus),
            Async::Hud(toast) => self.handle(ctx, Request::Hud(toast), None),
            Async::Toast(toast) => self.toast = Some((toast, ctx.input(|i| i.time))),
            Async::PopToRoot => self.pop_to_root(),
            Async::SetQuery(text) => self.handle(ctx, Request::SetQuery(text), None),
            Async::Push(make) => {
                let view = std::cell::RefCell::new(Some(make));
                let factory: ViewFactory = Rc::new(move |ctx| (view.borrow_mut().take().expect("pushed once"))(ctx));
                self.push(ctx, &factory, None);
            }
            Async::Launch(key, query) => {
                let Some(index) = self.commands.iter().position(|c| c.key == key) else { return };
                self.show(ctx);
                let action = self.primary_of(index);
                self.run(ctx, action, Some(key), false);
                if let Some(query) = query {
                    self.handle(ctx, Request::SetQuery(query), None);
                }
            }
            Async::Confirm(confirmation, reply) => {
                struct Reply(Option<Box<dyn FnOnce(bool) + Send>>);
                impl Drop for Reply {
                    fn drop(&mut self) {
                        if let Some(reply) = self.0.take() {
                            reply(false);
                        }
                    }
                }
                let reply = std::cell::RefCell::new(Reply(Some(reply)));
                let action = Action::new("confirm", confirmation.confirm_title.clone(), move |_| {
                    if let Some(reply) = reply.borrow_mut().0.take() {
                        reply(true);
                    }
                    Ok(())
                })
                .keep_open();
                self.pending = Some(Pending { confirmation, action, command: None, ok_focused: false });
            }
        }
    }

    fn push(&mut self, ctx: &egui::Context, factory: &ViewFactory, command: Option<&str>) {
        if let Some(key) = command {
            self.visit(key);
        }
        self.views.push(OpenView { view: factory(ctx), query: String::new(), selected: 0, picked_initial: false });
        self.focus = Focus::Search;
    }

    fn pop_view(&mut self) {
        self.dropdown = None;
        self.views.pop();
        self.focus = Focus::Search;
    }

    pub fn logic(&mut self, ctx: &egui::Context) {
        while let Ok(signal) = self.signals.try_recv() {
            match signal {
                Signal::Show => self.show(ctx),
                Signal::Quit => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
                Signal::Settings => self.open_settings(ctx),
                Signal::Hotkey(id) if id == "launcher" => {
                    if launcher::is_shown() {
                        self.hide(ctx, true);
                    } else {
                        self.show(ctx);
                    }
                }
                Signal::Hotkey(id) => {
                    if let Some(index) = self.commands.iter().position(|c| c.key == id) {
                        let command = &self.commands[index];
                        if command.confirm.is_some() || command.view.is_some() || command.argument.is_some() {
                            self.pop_to_root();
                            self.show(ctx);
                        }
                        let action = self.primary_of(index);
                        self.run(ctx, action, Some(id), false);
                    }
                }
            }
        }
        let requests: Vec<Async> = std::mem::take(&mut *crate::extension::ASYNC.lock().unwrap_or_else(|e| e.into_inner()));
        for request in requests {
            self.handle_async(ctx, request);
        }
        crate::flowkey_host::init(ctx);
        self.reload();
        if launcher::is_shown() && ctx.input(|i| i.viewport().focused) == Some(false) && !launcher::owns_foreground() && self.was_shown {
            self.hide(ctx, false);
        }
        self.was_shown = launcher::is_shown() && ctx.input(|i| i.viewport().focused) == Some(true);
    }

    fn selection(&mut self) -> &mut usize {
        match self.views.last_mut() {
            Some(view) => &mut view.selected,
            None => &mut self.selected,
        }
    }

    pub fn ui(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        theme::sync(&ctx, self.settings.appearance);
        crate::i18n::sync(self.settings.language);
        self.logic(&ctx);
        if self.views.last_mut().is_some_and(|v| v.view.form().is_some()) {
            self.form_ui(ui);
            if self.pending.is_some() {
                self.confirm_ui(ui, ui.max_rect());
            }
            self.tick(&ctx);
            return;
        }
        if let Some(page) = self.views.last_mut().and_then(|v| v.view.page()) {
            self.page_ui(ui, page);
            self.tick(&ctx);
            return;
        }
        let rows = self.rows(&ctx);
        if let Some(key) = self.keep_selected.take() {
            if let Some(i) = rows.iter().position(|r| r.id == key) {
                *self.selection() = i;
            }
        }
        if let Some(top) = self.views.last_mut() {
            if !top.picked_initial {
                if let Some(id) = top.view.initial_selection() {
                    if let Some(i) = rows.iter().position(|r| r.id == id) {
                        top.selected = i;
                        top.picked_initial = true;
                    }
                }
            }
        }
        let limit = rows.len().saturating_sub(1);
        let selected = (*self.selection()).min(limit);
        *self.selection() = selected;
        let current = rows.get(selected).cloned();
        let groups = current.as_ref().map(|r| self.groups(r)).unwrap_or_else(|| self.views.last().map(|v| v.view.empty_actions()).unwrap_or_default());
        let full = ui.max_rect();
        ui.painter().rect_filled(full, 0.0, theme::BG());

        if self.pending.is_some() {
            self.confirm_keys(ui);
        } else if self.panel.is_some() {
            self.panel_keys(ui);
        } else if self.dropdown.is_some() {
            self.dropdown_keys(ui);
        } else {
            let lines = self.views.last().and_then(|v| v.view.grid()).map(|columns| grid_lines(&rows, columns)).unwrap_or_default();
            self.main_keys(ui, rows.len(), current.as_ref(), &groups, &lines);
        }

        let header = Rect::from_min_size(full.min, vec2(full.width(), HEADER));
        let footer = Rect::from_min_max(pos2(full.left(), full.bottom() - FOOTER), full.max);
        let list = Rect::from_min_max(pos2(full.left(), header.bottom()), pos2(full.right(), footer.top()));
        let item_argument = current.as_ref().filter(|r| r.source == Source::Item).and_then(|r| self.views.last().and_then(|v| v.view.item_argument(&r.id)).map(|a| (r.id.clone(), a)));
        if let Some((id, (_, value))) = &item_argument {
            if *id != self.argument_for {
                self.argument_for = id.clone();
                self.argument = value.clone();
            }
        }
        let argument = match (&item_argument, current.as_ref().map(|r| r.source)) {
            (Some((_, (placeholder, _))), _) => Some(placeholder.clone()),
            (None, Some(Source::Command(i))) if self.views.is_empty() => self.commands[i].argument.as_ref().map(|a| crate::i18n::tr(&a.placeholder).into_owned()),
            _ => None,
        };
        self.header(ui, header, argument);
        if let Some((id, (_, value))) = &item_argument {
            if *value != self.argument {
                if let Some(top) = self.views.last_mut() {
                    top.view.set_argument(id, &self.argument);
                }
            }
        }
        self.list(ui, list, &rows);
        self.footer(ui, footer, current.as_ref(), &groups, None);
        if self.panel.is_some() {
            self.panel_ui(ui, footer.top());
        }
        if self.pending.is_some() {
            self.confirm_ui(ui, full);
        }
        self.tick(&ctx);
    }

    fn tick(&mut self, ctx: &egui::Context) {
        let now = ctx.input(|i| i.time);
        for notice in std::mem::take(&mut *crate::extension::NOTICES.lock().unwrap_or_else(|e| e.into_inner())) {
            self.hud = Some((notice, now));
        }
        for toast in std::mem::take(&mut *crate::extension::TOASTS.lock().unwrap_or_else(|e| e.into_inner())) {
            self.toast = Some((toast, now));
        }
        if let Some((_, at)) = &self.toast {
            let left = TOAST_SECONDS - (now - at);
            if left <= 0.0 {
                self.toast = None;
            } else {
                ctx.request_repaint_after(std::time::Duration::from_secs_f64(left));
            }
        }
        self.show_hud(ctx, now);
        self.show_notes(ctx);
        self.show_settings(ctx);
        crate::focus_ui::show(ctx);
        crate::capture_ui::show(ctx);
    }

    fn show_hud(&mut self, ctx: &egui::Context, now: f64) {
        let Some((toast, at)) = &self.hud else { return };
        let (title, color, seconds) = match toast {
            Toast::Success(title) => (title.clone(), theme::GREEN(), HUD_SECONDS),
            Toast::Failure(title, _) => (title.clone(), theme::RED(), HUD_FAILURE_SECONDS),
        };
        let left = seconds - (now - at);
        if left <= 0.0 {
            self.hud = None;
            return;
        }
        ctx.request_repaint_after(std::time::Duration::from_secs_f64(left));
        let width = ctx.fonts_mut(|f| f.layout_no_wrap(crate::i18n::tr(&title).into_owned(), theme::weighted(13.0, theme::MEDIUM), theme::FG())).size().x.ceil() + 48.0;
        let size = vec2(width.min(500.0), 46.0);
        let position = launcher::hud_position(size);
        let builder = egui::ViewportBuilder::default()
            .with_title("Openray HUD")
            .with_inner_size(size)
            .with_position(position)
            .with_decorations(false)
            .with_transparent(true)
            .with_always_on_top()
            .with_taskbar(false)
            .with_resizable(false)
            .with_active(false)
            .with_mouse_passthrough(true);
        ctx.show_viewport_immediate(egui::ViewportId::from_hash_of("hud"), builder, |ui, _| {
            let rect = ui.max_rect();
            let painter = ui.painter();
            painter.rect(rect, 8.0, theme::BG(), Stroke::new(1.0, theme::CONTROL_HOVER()), egui::StrokeKind::Inside);
            for (radius, alpha) in [(rect.width() * 0.45, 0.10), (rect.width() * 0.25, 0.10)] {
                painter.circle_filled(pos2(rect.center().x, rect.top() - 8.0), radius.min(40.0), color.gamma_multiply(alpha));
            }
            paint_text(ui, rect.center() + vec2(0.0, 1.0), Align2::CENTER_CENTER, &title, theme::weighted(13.0, theme::MEDIUM), theme::FG(), rect.width() - 48.0);
        });
    }

    fn open_settings(&mut self, ctx: &egui::Context) {
        if self.settings_window.is_none() {
            self.settings_window = Some(settings_ui::SettingsWindow::open("general"));
        }
        self.hide(ctx, false);
        ctx.send_viewport_cmd_to(egui::ViewportId::from_hash_of("settings"), egui::ViewportCommand::Focus);
        ctx.request_repaint();
    }

    fn show_notes(&mut self, ctx: &egui::Context) {
        let Some(notes) = &mut self.notes else { return };
        let builder = egui::ViewportBuilder::default().with_title("Notes").with_inner_size([440.0, 762.0]).with_min_inner_size([300.0, 200.0]).with_always_on_top();
        let close = ctx.show_viewport_immediate(egui::ViewportId::from_hash_of("notes"), builder, |ui, _| {
            notes.ui(ui);
            ui.input(|i| i.viewport().close_requested())
        });
        if close {
            self.notes = None;
        }
    }

    fn main_keys(&mut self, ui: &Ui, count: usize, current: Option<&Row>, groups: &[Vec<Action>], lines: &[std::ops::Range<usize>]) {
        let ctx = ui.ctx().clone();
        let in_view = !self.views.is_empty();
        let key_of = current.and_then(Self::row_key);
        for (key, modifiers) in key_presses(ui) {
            let text_empty = match (&self.focus, self.views.last()) {
                (Focus::Argument, _) => self.argument.is_empty(),
                (_, Some(view)) => view.query.is_empty(),
                _ => self.query.is_empty(),
            };
            let mut handled = true;
            let dropdown = self.views.last().and_then(|v| v.view.dropdown());
            if key == Key::P && modifiers.ctrl && !modifiers.alt && !modifiers.shift && dropdown.is_some() {
                self.dropdown = dropdown.map(|d| Open::new(HEADER_SLOT, d.selected, d.options.len()));
            } else if key == Key::K && modifiers.ctrl && !modifiers.alt && !modifiers.shift {
                if let Some(row) = current.filter(|_| !groups.is_empty()) {
                    self.panel = Some(Panel { title: row.title.clone(), groups: groups.to_vec(), selected: 0, filter: String::new(), command: key_of.clone() });
                }
            } else if key == Key::Enter && plain(modifiers) {
                if let Some(action) = groups.first().and_then(|g| g.first()) {
                    self.run(&ctx, action.clone(), key_of.clone(), false);
                }
            } else if let Some(action) = groups.iter().flatten().find(|a| a.shortcut.is_some_and(|s| s.matches(key, modifiers))) {
                self.run(&ctx, action.clone(), key_of.clone(), false);
            } else if (key == Key::ArrowDown || key == Key::ArrowUp) && plain(modifiers) && !lines.is_empty() {
                let next = grid_move(lines, *self.selection(), if key == Key::ArrowDown { 1 } else { -1 });
                *self.selection() = next;
                self.scroll_to_selected = true;
            } else if (key == Key::ArrowLeft || key == Key::ArrowRight) && plain(modifiers) && !lines.is_empty() && self.focus == Focus::Search {
                let next = if key == Key::ArrowRight { (*self.selection() + 1).min(count.saturating_sub(1)) } else { self.selection().saturating_sub(1) };
                *self.selection() = next;
                self.scroll_to_selected = true;
            } else if key == Key::ArrowDown && plain(modifiers) {
                let next = (*self.selection() + 1).min(count.saturating_sub(1));
                *self.selection() = next;
                self.scroll_to_selected = true;
            } else if key == Key::ArrowUp && plain(modifiers) {
                let next = self.selection().saturating_sub(1);
                *self.selection() = next;
                self.scroll_to_selected = true;
            } else if key == Key::Tab && !in_view && current.is_some_and(|r| r.card.as_ref().is_some_and(|c| matches!(c.kind, CardKind::Weather(_)))) {
                self.weather_week = !self.weather_week;
            } else if key == Key::Tab && in_view && self.views.last().is_some_and(|v| v.view.tab_cycles()) && count > 0 {
                let step = if modifiers.shift { count - 1 } else { 1 };
                let next = (*self.selection() + step) % count;
                *self.selection() = next;
                self.scroll_to_selected = true;
            } else if key == Key::Tab && (current.is_some_and(|r| matches!(r.source, Source::Command(i) if !in_view && self.commands[i].argument.is_some())) || current.is_some_and(|r| r.source == Source::Item && self.views.last().is_some_and(|v| v.view.item_argument(&r.id).is_some()))) {
                self.focus = if self.focus == Focus::Search { Focus::Argument } else { Focus::Search };
            } else if self.focus == Focus::Argument && (key == Key::Escape || (key == Key::Backspace && text_empty)) {
                self.focus = Focus::Search;
            } else if key == Key::Backspace && in_view && text_empty {
                self.pop_view();
            } else if key == Key::Escape && plain(modifiers) {
                if let Some(view) = self.views.last_mut() {
                    if view.query.is_empty() {
                        self.pop_view();
                    } else {
                        view.query.clear();
                        view.selected = 0;
                    }
                } else if !self.query.is_empty() && self.settings.escape_key_behavior != EscapeKeyBehavior::CloseAndPopToRoot {
                    self.query.clear();
                    self.selected = 0;
                } else {
                    if self.settings.escape_key_behavior == EscapeKeyBehavior::CloseAndPopToRoot {
                        self.pop_to_root();
                    }
                    self.hide(&ctx, true);
                }
            } else {
                handled = false;
            }
            if handled {
                consume(ui, key, modifiers);
            }
        }
    }

    fn dropdown_keys(&mut self, ui: &Ui) {
        for (key, modifiers) in key_presses(ui) {
            let Some(open) = &mut self.dropdown else { return };
            let count = open.count.max(1);
            match key {
                Key::Escape if !open.query.is_empty() => {
                    open.query.clear();
                    open.highlight = 0;
                    open.scroll = true;
                }
                Key::Escape => self.dropdown = None,
                Key::ArrowDown => {
                    open.highlight = (open.highlight.min(count - 1) + 1) % count;
                    open.scroll = true;
                }
                Key::ArrowUp => {
                    open.highlight = (open.highlight.min(count - 1) + count - 1) % count;
                    open.scroll = true;
                }
                Key::Enter => open.enter = true,
                _ => continue,
            }
            consume(ui, key, modifiers);
        }
    }

    fn choose(&mut self, index: usize) {
        self.dropdown = None;
        if let Some(view) = self.views.last_mut() {
            view.view.select(index);
            view.selected = 0;
            self.scroll_to_selected = true;
        }
    }

    fn dropdown_trigger(ui: &Ui, trigger: Rect, slot: usize) -> egui::Response {
        ui.interact(trigger, egui::Id::unique(("dropdown", slot)), Sense::click())
    }

    fn dropdown_ui(&mut self, ui: &mut Ui, response: egui::Response, slot: usize, label: &str, options: &[(String, Icon)], selected: usize) -> Option<usize> {
        let trigger = response.rect;
        let search_id = egui::Id::unique(("dropdown-search", slot));
        if self.dropdown.as_ref().is_none_or(|o| o.slot != slot) && ui.memory(|m| m.has_focus(search_id)) {
            response.request_focus();
        }
        let selected = selected.min(options.len().saturating_sub(1));
        let keyboard_open = response.has_focus() && self.dropdown.is_none() && ui.input(|i| i.key_pressed(Key::Enter) || i.key_pressed(Key::Space));
        let current = options.get(selected).map_or("", |o| o.0.as_str());
        response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::ComboBox, true, format!("{label}: {current}")));
        let border = if response.has_focus() { theme::FG_40() } else { theme::KEY_BORDER() };
        ui.painter().rect(trigger, 8.0, if response.hovered() { theme::fg(5) } else { Color32::TRANSPARENT }, Stroke::new(1.0, border), egui::StrokeKind::Inside);
        let option_icon = |ui: &mut Ui, icon: &Icon, rect: Rect| match icon {
            Icon::Glyph(glyph) => icons::paint_glyph(ui, glyph, rect, theme::FG()),
            other => icons::command_icon(&mut ui.new_child(UiBuilder::new().max_rect(rect)), other, "", 16.0),
        };
        if let Some((text, icon)) = options.get(selected) {
            option_icon(ui, icon, Rect::from_min_size(pos2(trigger.left() + 12.0, trigger.center().y - 8.0), vec2(16.0, 16.0)));
            paint_text(ui, pos2(trigger.left() + 36.0, trigger.center().y), Align2::LEFT_CENTER, text, theme::weighted(13.0, theme::MEDIUM), theme::FG(), trigger.width() - 64.0);
        }
        icons::paint_glyph(ui, "chevron-down", Rect::from_min_size(pos2(trigger.right() - 12.0 - 12.0, trigger.center().y - 6.0), vec2(12.0, 12.0)), theme::FG_60());
        if response.clicked() || keyboard_open {
            self.dropdown = match &self.dropdown {
                Some(open) if open.slot == slot => None,
                _ => Some(Open::new(slot, selected, options.len())),
            };
        }
        let mut open = self.dropdown.clone().filter(|o| o.slot == slot)?;
        let visible: Vec<usize> = (0..options.len()).filter(|&i| open.query.trim().is_empty() || match_score(&open.query, &options[i].0, &[]) > 0).collect();
        if open.enter {
            self.dropdown = None;
            response.request_focus();
            return visible.get(open.highlight.min(visible.len().saturating_sub(1))).copied();
        }
        let ctx = ui.ctx().clone();
        let list_height = (14.0 + visible.len().max(1) as f32 * 32.0).min(MENU_LIST_MAX);
        let size = vec2(trigger.width().clamp(197.0, 360.0), MENU_SEARCH + list_height);
        let below = trigger.bottom() + 4.0 + size.y <= ctx.content_rect().bottom();
        let frame = if below { Rect::from_min_size(pos2(trigger.right() - size.x, trigger.bottom() + 4.0), size) } else { Rect::from_min_size(pos2(trigger.right() - size.x, trigger.top() - 4.0 - size.y), size) };
        let (search, list) = if below {
            (Rect::from_min_size(frame.min, vec2(size.x, MENU_SEARCH)), Rect::from_min_max(pos2(frame.left(), frame.top() + MENU_SEARCH), frame.max))
        } else {
            (Rect::from_min_max(pos2(frame.left(), frame.bottom() - MENU_SEARCH), frame.max), Rect::from_min_size(frame.min, vec2(size.x, list_height)))
        };
        let mut chosen = None;
        let mut hovered = None;
        let mut query_changed = false;
        egui::Area::new(egui::Id::unique(("dropdown-menu", slot))).order(egui::Order::Foreground).fixed_pos(frame.min).show(&ctx, |ui| {
            ui.painter().add(egui::epaint::RectShape::filled(frame.translate(vec2(0.0, 8.0)).expand(4.0), 10.0, Color32::from_black_alpha(40)));
            ui.painter().rect(frame, 6.0, theme::PANEL(), Stroke::new(1.0, theme::PANEL_BORDER()), egui::StrokeKind::Inside);
            let line_y = if below { search.bottom() - 0.5 } else { search.top() + 0.5 };
            ui.painter().hline(frame.x_range().shrink(1.0), line_y, Stroke::new(1.0, theme::PANEL_LINE()));
            let inner = Rect::from_min_max(pos2(search.left() + 16.0, search.center().y - 9.0), pos2(search.right() - 16.0, search.center().y + 9.0));
            let hint = egui::RichText::new(crate::i18n::tr("Search...")).color(theme::FG_40()).font(theme::font(13.0));
            let edit = egui::TextEdit::singleline(&mut open.query).id(search_id).frame(egui::Frame::NONE).margin(egui::Margin::ZERO).font(theme::font(13.0)).text_color(theme::FG()).hint_text(hint).desired_width(inner.width());
            let field = ui.put(inner, edit);
            field.widget_info(|| egui::WidgetInfo::text_edit(true, "Search options", &open.query, ""));
            query_changed = field.changed();
            ui.memory_mut(|m| m.request_focus(search_id));
            let rows = list.shrink(7.0);
            ui.scope_builder(UiBuilder::new().max_rect(rows), |ui| {
                ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                egui::ScrollArea::vertical().id_salt(("dropdown-list", slot)).max_height(rows.height()).auto_shrink(false).show(ui, |ui| {
                    if visible.is_empty() {
                        let (row, _) = ui.allocate_exact_size(vec2(ui.available_width(), 32.0), Sense::hover());
                        paint_text(ui, pos2(row.left() + 8.0, row.center().y), Align2::LEFT_CENTER, "No results", theme::font(13.0), theme::FG_60(), row.width());
                    }
                    for (n, &i) in visible.iter().enumerate() {
                        let (text, icon) = &options[i];
                        let (row, option) = ui.allocate_exact_size(vec2(ui.available_width(), 32.0), Sense::click());
                        option.widget_info(|| egui::WidgetInfo::selected(egui::Role::ListBoxOption, true, i == selected, text));
                        if option.hovered() && ui.input(|i| i.pointer.delta() != egui::Vec2::ZERO) {
                            hovered = Some(n);
                        }
                        if n == open.highlight {
                            ui.painter().rect_filled(row, 6.0, theme::PANEL_LINE());
                            if open.scroll {
                                option.scroll_to_me(None);
                            }
                        }
                        option_icon(ui, icon, Rect::from_min_size(pos2(row.left() + 8.0, row.center().y - 8.0), vec2(16.0, 16.0)));
                        paint_text(ui, pos2(row.left() + 32.0, row.center().y), Align2::LEFT_CENTER, text, theme::weighted(13.0, theme::MEDIUM), theme::FG(), row.width() - 40.0);
                        if option.clicked() {
                            chosen = Some(i);
                        }
                    }
                });
            });
        });
        open.scroll = query_changed;
        open.count = visible.len();
        if query_changed {
            open.highlight = 0;
        }
        if let Some(n) = hovered {
            open.highlight = n;
        }
        self.dropdown = Some(open);
        if chosen.is_some() {
            self.dropdown = None;
            response.request_focus();
        } else if !response.clicked() && ctx.input(|i| i.pointer.any_click()) && ctx.input(|i| i.pointer.interact_pos()).is_some_and(|p| !frame.contains(p)) {
            self.dropdown = None;
        }
        chosen
    }

    fn form_ui(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        let full = ui.max_rect();
        ui.painter().rect_filled(full, 0.0, theme::BG());
        let (mut submit, mut back) = (false, false);
        let actions = self.views.last_mut().map(|v| v.view.form_actions()).unwrap_or_default();
        let has_panel = actions.iter().flatten().count() > 1;
        if self.pending.is_some() {
            self.confirm_keys(ui);
        } else if self.panel.is_some() {
            self.panel_keys(ui);
        } else if self.dropdown.is_some() {
            self.dropdown_keys(ui);
        } else {
            for (key, modifiers) in key_presses(ui) {
                if key == Key::Enter && modifiers.ctrl && !modifiers.alt && !modifiers.shift {
                    submit = true;
                } else if key == Key::K && modifiers.ctrl && !modifiers.alt && !modifiers.shift && has_panel {
                    let title = self.views.last_mut().and_then(|v| v.view.form()).map(|f| f.title.clone()).unwrap_or_default();
                    self.panel = Some(Panel { title, groups: actions.clone(), selected: 0, filter: String::new(), command: None });
                } else if let Some(action) = actions.iter().flatten().skip(1).find(|a| a.shortcut.is_some_and(|s| s.matches(key, modifiers))) {
                    self.run(&ctx, action.clone(), None, false);
                } else if key == Key::Escape && plain(modifiers) && !token_query_open(ui) && !crate::date_picker::is_open(ui) {
                    back = true;
                } else {
                    continue;
                }
                consume(ui, key, modifiers);
            }
        }
        let header = Rect::from_min_size(full.min, vec2(full.width(), HEADER));
        let footer = Rect::from_min_max(pos2(full.left(), full.bottom() - FOOTER), full.max);
        ui.painter().hline(header.x_range(), header.bottom() - 0.5, Stroke::new(1.0, theme::SEPARATOR()));
        ui.painter().hline(footer.x_range(), footer.top() + 0.5, Stroke::new(1.0, theme::SEPARATOR()));
        let lead = Rect::from_center_size(pos2(header.left() + 16.0 + 11.0, header.center().y), vec2(22.0, 22.0));
        back |= back_button(ui, lead);
        let Some(top) = self.views.last_mut() else { return };
        let Some(form) = top.view.form() else { return };
        let (title, submit_title, icon) = (form.title.clone(), form.submit, form.icon.clone());
        paint_text(ui, pos2(lead.right() + 12.0, header.center().y), Align2::LEFT_CENTER, &title, theme::weighted(13.0, theme::MEDIUM), theme::FG_60(), 400.0);
        let mut changed = Vec::new();
        let mut picks = Vec::new();
        let body = Rect::from_min_max(pos2(full.left(), header.bottom()), pos2(full.right(), footer.top()));
        let mut scroll_ui = ui.new_child(UiBuilder::new().max_rect(body));
        egui::ScrollArea::vertical().id_salt(("form", self.views.len())).auto_shrink(false).show(&mut scroll_ui, |ui| {
            let Some(form) = self.views.last_mut().and_then(|v| v.view.form()) else { return };
            let origin = ui.max_rect().min;
            let label_right = full.left() + 175.0;
            let field_left = label_right + 20.0;
            let mut y = origin.y + 6.0;
            let mut choices = Vec::new();
            let focus = form.focus.take();
            for (i, field) in form.fields.iter_mut().enumerate() {
                match field.kind {
                    crate::extension::FieldKind::Separator => {
                        ui.painter().hline((full.left() + 24.0)..=(field_left + 360.0), y + 0.5, Stroke::new(1.0, theme::SEPARATOR()));
                        y += 21.0;
                        continue;
                    }
                    crate::extension::FieldKind::Note => {
                        let galley = wrapped(ui, &field.value, theme::font(13.0), theme::FG_60(), 360.0);
                        let h = galley.size().y;
                        ui.painter().galley(pos2(field_left, y), galley, theme::FG_60());
                        y += h + 20.0;
                        continue;
                    }
                    _ => {}
                }
                paint_text(ui, pos2(label_right, y + 9.0), Align2::RIGHT_TOP, field.label, theme::font(13.0), theme::FG_60(), 170.0);
                let height = match field.kind {
                    crate::extension::FieldKind::Area(h) => h,
                    _ => 34.0,
                };
                let rect = Rect::from_min_size(pos2(field_left, y), vec2(360.0, height));
                let id = egui::Id::unique(("form-field", field.label));
                let mut bottom = rect.bottom();
                match &field.kind {
                    crate::extension::FieldKind::Tokens(spec) => {
                        let spec = spec.clone();
                        let (used, edited) = tokens_field(ui, rect.min, field, &spec, id);
                        bottom = used;
                        if edited {
                            changed.push(i);
                        }
                    }
                    crate::extension::FieldKind::Date { time } => {
                        let time = *time;
                        if crate::date_picker::field(ui, rect, field, id, time) {
                            changed.push(i);
                        }
                    }
                    crate::extension::FieldKind::Choice(options) => {
                        let selected = options.iter().position(|(value, _, _)| *value == field.value).unwrap_or(0);
                        let trigger = Self::dropdown_trigger(ui, rect, i);
                        if trigger.gained_focus() {
                            trigger.scroll_to_me(None);
                        }
                        choices.push((i, trigger, options.iter().map(|(_, label, icon)| (label.clone(), icon.clone())).collect::<Vec<_>>(), selected, field.label));
                    }
                    crate::extension::FieldKind::Toggle => {
                        let on = field.value == "true";
                        let switch = Rect::from_min_size(pos2(rect.left(), rect.center().y - 8.0), vec2(28.0, 16.0));
                        let response = ui.interact(switch, id, Sense::click());
                        response.widget_info(|| egui::WidgetInfo::selected(egui::Role::Switch, true, on, field.label));
                        if on {
                            ui.painter().rect_filled(switch, 8.0, theme::ACCENT());
                            ui.painter().circle_filled(pos2(switch.right() - 3.0 - 5.0, switch.center().y), 5.0, Color32::WHITE);
                        } else {
                            ui.painter().rect_stroke(switch, 8.0, Stroke::new(1.0, theme::fg(25)), egui::StrokeKind::Inside);
                            ui.painter().circle_filled(pos2(switch.left() + 3.0 + 4.0, switch.center().y), 4.0, theme::FG().gamma_multiply(0.9));
                        }
                        if response.has_focus() {
                            ui.painter().rect_stroke(switch.expand(2.0), 10.0, Stroke::new(1.0, theme::FG_40()), egui::StrokeKind::Outside);
                        }
                        if response.gained_focus() {
                            response.scroll_to_me(None);
                        }
                        if response.clicked() || (response.has_focus() && ui.input(|i| i.key_pressed(Key::Space))) {
                            field.value = (!on).to_string();
                            changed.push(i);
                        }
                    }
                    crate::extension::FieldKind::Path { directory, multiple } => {
                        let (directory, multiple) = (*directory, *multiple);
                        let title = if directory { "Choose Folder…" } else { "Choose File…" };
                        let button = Rect::from_min_size(rect.min, vec2(132.0, rect.height()));
                        let response = ui.interact(button, id, Sense::click());
                        response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, format!("{} {title}", field.label)));
                        ui.painter().rect_filled(button, 8.0, if response.hovered() { theme::CONTROL_HOVER() } else { theme::CONTROL() });
                        if response.has_focus() {
                            ui.painter().rect_stroke(button, 8.0, Stroke::new(1.0, theme::FG_40()), egui::StrokeKind::Inside);
                        }
                        paint_text(ui, button.center(), Align2::CENTER_CENTER, title, theme::weighted(13.0, theme::MEDIUM), theme::FG(), button.width() - 12.0);
                        let shown = if field.value.is_empty() { "Nothing selected".to_owned() } else { field.value.lines().collect::<Vec<_>>().join(", ") };
                        paint_text(ui, pos2(button.right() + 10.0, rect.center().y), Align2::LEFT_CENTER, &shown, theme::font(12.0), theme::FG_60(), rect.right() - button.right() - 10.0);
                        if response.gained_focus() {
                            response.scroll_to_me(None);
                        }
                        if response.clicked() || (response.has_focus() && ui.input(|i| i.key_pressed(Key::Space))) {
                            match crate::apps::pick_path(directory, multiple, &[]) {
                                Ok(paths) if !paths.is_empty() => {
                                    field.value = paths.join("\n");
                                    field.error = None;
                                    changed.push(i);
                                }
                                Ok(_) => {}
                                Err(error) => field.error = Some(error),
                            }
                        }
                    }
                    kind => {
                        let focused = ui.memory(|m| m.has_focus(id));
                        let border = if field.error.is_some() { theme::RED() } else if focused { theme::FG_40() } else { theme::KEY_BORDER() };
                        ui.painter().rect(rect, 8.0, theme::fg(3), Stroke::new(1.0, border), egui::StrokeKind::Inside);
                        let font = if field.mono { FontId::monospace(13.0) } else { theme::font(13.0) };
                        let hint = egui::RichText::new(field.placeholder.as_str()).color(theme::FG_40()).font(font.clone());
                        let inner = Rect::from_min_max(rect.min + vec2(12.0, 8.0), rect.max - vec2(12.0, 8.0));
                        let edit = if matches!(kind, crate::extension::FieldKind::Area(_)) { egui::TextEdit::multiline(&mut field.value).desired_rows(1) } else { egui::TextEdit::singleline(&mut field.value) };
                        let response = ui.put(inner, edit.id(id).frame(egui::Frame::NONE).margin(egui::Margin::ZERO).font(font).text_color(theme::FG()).hint_text(hint).desired_width(inner.width()).password(field.secret));
                        response.widget_info(|| egui::WidgetInfo::text_edit(true, field.label, &field.value, ""));
                        if response.changed() {
                            field.error = None;
                            changed.push(i);
                        }
                        if response.gained_focus() {
                            response.scroll_to_me(None);
                        }
                    }
                }
                if focus == Some(i) {
                    ui.memory_mut(|m| m.request_focus(id));
                }
                y = bottom;
                if let Some(note) = field.error.as_ref().or(field.help.as_ref()) {
                    let color = if field.error.is_some() { theme::RED() } else { theme::FG_60() };
                    let galley = wrapped(ui, note, theme::font(12.0), color, 360.0);
                    let h = galley.size().y;
                    ui.painter().galley(pos2(field_left, y + 6.0), galley, color);
                    y += 6.0 + h;
                }
                y += 20.0;
            }
            ui.expand_to_include_rect(Rect::from_min_max(origin, pos2(origin.x + 1.0, y)));
            for (i, rect, options, selected, label) in choices {
                if let Some(choice) = self.dropdown_ui(ui, rect, i, label, &options, selected) {
                    picks.push((i, choice));
                }
            }
        });
        let tile = Rect::from_min_size(pos2(footer.left() + 12.0, footer.center().y - 8.0), vec2(16.0, 16.0));
        icons::command_icon(&mut ui.new_child(UiBuilder::new().max_rect(tile)), &icon, &title, 16.0);
        paint_text(ui, pos2(tile.right() + 8.0, footer.center().y), Align2::LEFT_CENTER, &title, theme::weighted(13.0, theme::MEDIUM), theme::FG_60(), 300.0);
        let mut right = footer.right() - 12.0;
        if has_panel {
            let center_y = footer.center().y;
            let caps = keycaps(ui, right, center_y, &["Ctrl".into(), "K".into()]);
            let label = paint_text(ui, pos2(caps - 8.0, center_y), Align2::RIGHT_CENTER, "Actions", theme::weighted(13.0, theme::MEDIUM), theme::FG_60(), 200.0);
            let button = Rect::from_min_max(pos2(label.left() - 8.0, center_y - 13.0), pos2(footer.right() - 9.0, center_y + 13.0));
            let response = ui.interact(button, egui::Id::unique("form-actions"), Sense::click());
            response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, "Actions"));
            if self.panel.is_some() {
                ui.painter().rect_filled(button, 6.0, theme::SELECTION());
                keycaps(ui, right, center_y, &["Ctrl".into(), "K".into()]);
                paint_text(ui, pos2(caps - 8.0, center_y), Align2::RIGHT_CENTER, "Actions", theme::weighted(13.0, theme::MEDIUM), theme::FG_60(), 200.0);
            }
            ui.painter().vline(button.left() - 6.0, (center_y - 5.5)..=(center_y + 5.5), Stroke::new(1.0, theme::FOOTER_DIVIDER()));
            if response.clicked() {
                self.panel = match self.panel {
                    Some(_) => None,
                    None => Some(Panel { title: title.clone(), groups: actions.clone(), selected: 0, filter: String::new(), command: None }),
                };
            }
            right = button.left() - 13.0;
        }
        let caps_left = keycaps(ui, right, footer.center().y, &["Ctrl".into(), "↵".into()]);
        let label = paint_text(ui, pos2(caps_left - 8.0, footer.center().y), Align2::RIGHT_CENTER, submit_title, theme::weighted(13.0, theme::MEDIUM), theme::FG(), 300.0);
        let button = ui.interact(Rect::from_min_max(pos2(label.left() - 8.0, footer.center().y - 13.0), pos2(right + 3.0, footer.center().y + 13.0)), egui::Id::unique("form-submit"), Sense::click());
        button.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, submit_title));
        submit |= button.clicked();
        if let Some(form) = self.views.last_mut().and_then(|v| v.view.form()) {
            for (i, choice) in picks {
                if let Some(field) = form.fields.get_mut(i) {
                    if let crate::extension::FieldKind::Choice(options) = &field.kind {
                        if let Some((value, _, _)) = options.get(choice) {
                            field.value = value.clone();
                            changed.push(i);
                        }
                    }
                }
            }
        }
        if let Some(top) = self.views.last_mut() {
            for i in changed {
                top.view.form_changed(i);
            }
        }
        if submit {
            let mut host = Host { ctx: ctx.clone(), toast: None, requests: Vec::new() };
            let result = self.views.last_mut().map_or(Ok(false), |v| v.view.submit(&mut host));
            if let Some(toast) = host.toast {
                self.toast = Some((toast, ctx.input(|i| i.time)));
            }
            for request in host.requests {
                self.handle(&ctx, request, None);
            }
            match result {
                Ok(true) => self.pop_view(),
                Ok(false) => {}
                Err(e) => self.fail(&ctx, format!("Could not save {}", submit_title.trim_start_matches("Save ")), e),
            }
        } else if back {
            self.pop_view();
        }
        if self.panel.is_some() {
            self.panel_ui(ui, footer.top());
        }
    }

    fn filtered_panel(&self) -> Vec<Vec<Action>> {
        let Some(panel) = &self.panel else { return Vec::new() };
        if panel.filter.trim().is_empty() {
            return panel.groups.clone();
        }
        panel.groups.iter().map(|g| g.iter().filter(|a| match_score(&panel.filter, &a.title, &[]) > 0).cloned().collect::<Vec<_>>()).filter(|g| !g.is_empty()).collect()
    }

    fn panel_keys(&mut self, ui: &Ui) {
        let ctx = ui.ctx().clone();
        let actions: Vec<Action> = self.filtered_panel().into_iter().flatten().collect();
        for (key, modifiers) in key_presses(ui) {
            let Some(panel) = &mut self.panel else { return };
            let mut handled = true;
            if (key == Key::K && modifiers.ctrl && !modifiers.alt && !modifiers.shift) || (key == Key::Escape && plain(modifiers)) {
                self.panel = None;
            } else if key == Key::ArrowDown && plain(modifiers) {
                panel.selected = (panel.selected + 1).min(actions.len().saturating_sub(1));
            } else if key == Key::ArrowUp && plain(modifiers) {
                panel.selected = panel.selected.saturating_sub(1);
            } else if key == Key::Enter && plain(modifiers) {
                if let Some(action) = actions.get(panel.selected).cloned() {
                    let command = panel.command.clone();
                    self.run(&ctx, action, command, false);
                }
            } else if let Some(action) = actions.iter().find(|a| a.shortcut.is_some_and(|s| s.matches(key, modifiers))).cloned() {
                let command = panel.command.clone();
                self.run(&ctx, action, command, false);
            } else {
                handled = false;
            }
            if handled {
                consume(ui, key, modifiers);
            }
        }
    }

    fn confirm_keys(&mut self, ui: &Ui) {
        let ctx = ui.ctx().clone();
        for (key, modifiers) in key_presses(ui) {
            if key == Key::Escape {
                self.pending = None;
            } else if key == Key::Enter && plain(modifiers) {
                if let Some(p) = self.pending.take().filter(|p| p.ok_focused) {
                    self.run(&ctx, p.action, p.command, true);
                }
            } else if key == Key::Tab {
                if let Some(p) = &mut self.pending {
                    p.ok_focused = !p.ok_focused;
                }
            } else {
                continue;
            }
            consume(ui, key, modifiers);
        }
    }

    fn header(&mut self, ui: &mut Ui, rect: Rect, argument: Option<String>) {
        ui.painter().hline(rect.x_range(), rect.bottom() - 0.5, Stroke::new(1.0, theme::SEPARATOR()));
        let now = ui.input(|i| i.time);
        self.loading_since = self.views.last().is_some_and(|v| v.view.loading()).then(|| self.loading_since.unwrap_or(now));
        if let Some(since) = self.loading_since {
            if let Some((offset, alpha)) = loader_frame(now - since) {
                paint_loader(ui, Rect::from_min_max(pos2(rect.left(), rect.bottom() - 1.0), rect.max), offset, alpha);
            }
            ui.ctx().request_repaint();
        }
        let lead =Rect::from_center_size(pos2(rect.left() + 16.0 + 11.0, rect.center().y), vec2(22.0, 22.0));
        let placeholder = match self.views.last() {
            Some(view) => {
                if back_button(ui, lead) {
                    self.pop_view();
                    return;
                }
                crate::i18n::tr(&view.view.placeholder()).into_owned()
            }
            None => {
                icons::paint_glyph(ui, "openray", lead, theme::FG_60());
                crate::i18n::tr("Search for apps or commands...").into_owned()
            }
        };
        let mut input = Rect::from_min_max(pos2(lead.right() + 12.0, rect.top() + 1.0), pos2(rect.right() - 16.0, rect.bottom() - 1.0));
        if let Some(dropdown) = self.views.last().and_then(|v| v.view.dropdown()) {
            let trigger = Rect::from_min_size(pos2(rect.right() - 16.0 - DROPDOWN_WIDTH, rect.center().y - 17.0), vec2(DROPDOWN_WIDTH, 34.0));
            input.max.x = trigger.left() - 12.0;
            let response = Self::dropdown_trigger(ui, trigger, HEADER_SLOT);
            if let Some(index) = self.dropdown_ui(ui, response, HEADER_SLOT, dropdown.label, &dropdown.options, dropdown.selected) {
                self.choose(index);
            }
        }
        let search_id = egui::Id::unique(("launcher-search", self.views.len()));
        let argument_id = egui::Id::unique("launcher-argument");
        if argument.is_none() && self.focus == Focus::Argument {
            self.focus = Focus::Search;
        }
        if let Some(placeholder) = argument.as_deref() {
            let current = self.views.last().map_or(self.query.as_str(), |v| v.query.as_str());
            let shown = if current.is_empty() { placeholder_owned(&self.views) } else { current.to_owned() };
            let shown = shown.as_str();
            let query_width = ui.ctx().fonts_mut(|f| f.layout_no_wrap(shown.to_owned(), theme::font(18.0), theme::FG())).size().x + 12.0;
            let query_width = query_width.min(input.width() * 0.6);
            let arg_text = if self.argument.is_empty() { placeholder } else { self.argument.as_str() };
            let arg_width = (ui.ctx().fonts_mut(|f| f.layout_no_wrap(arg_text.to_owned(), theme::font(13.0), theme::FG())).size().x + 20.0).min(200.0);
            let field = Rect::from_min_size(pos2(input.left() + query_width + 12.0, rect.center().y - 12.0), vec2(arg_width, 24.0));
            let focused = self.focus == Focus::Argument;
            ui.painter().rect_filled(field, 5.0, if focused { theme::CONTROL_HOVER() } else { theme::CONTROL() });
            let edit = egui::TextEdit::singleline(&mut self.argument)
                .id(argument_id)
                .frame(egui::Frame::NONE)
                .margin(egui::Margin::ZERO)
                .font(theme::font(13.0))
                .text_color(theme::FG())
                .hint_text(egui::RichText::new(placeholder).color(theme::FG_60()).font(theme::font(13.0)))
                .desired_width(field.width() - 16.0);
            let response = ui.put(Rect::from_center_size(field.center(), vec2(field.width() - 16.0, 18.0)), edit);
            response.widget_info(|| egui::WidgetInfo::text_edit(true, placeholder, &self.argument, ""));
            if response.clicked() {
                self.focus = Focus::Argument;
            }
            input.max.x = input.left() + query_width;
        } else {
            self.argument.clear();
        }
        let text = match self.views.last_mut() {
            Some(view) => &mut view.query,
            None => &mut self.query,
        };
        let before = text.clone();
        if std::mem::take(&mut self.cursor_to_end) {
            let mut state = egui::text_edit::TextEditState::load(ui.ctx(), search_id).unwrap_or_default();
            state.cursor.set_char_range(Some(egui::text::CCursorRange::one(egui::text::CCursor::new(text.chars().count()))));
            state.store(ui.ctx(), search_id);
        }
        let edit = egui::TextEdit::singleline(text)
            .id(search_id)
            .frame(egui::Frame::NONE)
            .margin(egui::Margin::ZERO)
            .font(theme::font(18.0))
            .text_color(theme::FG())
            .hint_text(egui::RichText::new(placeholder.as_str()).color(theme::FG_40()).font(theme::font(18.0)))
            .desired_width(input.width());
        let response = ui.put(Rect::from_center_size(input.center(), vec2(input.width(), 24.0)), edit);
        let label = placeholder.trim_end_matches(['.', '…']).to_string();
        let changed = *text != before;
        let value = text.clone();
        response.widget_info(|| egui::WidgetInfo::text_edit(true, &label, &value, ""));
        if response.clicked() {
            self.focus = Focus::Search;
        }
        if self.panel.is_none() && self.pending.is_none() && self.dropdown.is_none() {
            let id = if self.focus == Focus::Argument { argument_id } else { search_id };
            ui.memory_mut(|m| m.request_focus(id));
        }
        if changed {
            *self.selection() = 0;
            self.scroll_to_selected = true;
        }
    }

    fn list(&mut self, ui: &mut Ui, rect: Rect, rows: &[Row]) {
        let ctx = ui.ctx().clone();
        let detail = self.views.last().is_some_and(|v| v.view.has_detail()).then(|| Rect::from_min_max(pos2(rect.left() + DETAIL_LIST, rect.top()), rect.max));
        let rect = if detail.is_some() { Rect::from_min_size(rect.min, vec2(DETAIL_LIST, rect.height())) } else { rect };
        if let Some(pane) = detail {
            ui.painter().vline(pane.left() - 0.5, pane.y_range(), Stroke::new(1.0, theme::SEPARATOR()));
            let selected = (*self.selection()).min(rows.len().saturating_sub(1));
            if let (Some(row), Some(top)) = (rows.get(selected), self.views.last_mut()) {
                top.view.detail(&mut ui.new_child(UiBuilder::new().max_rect(pane)), pane, &row.id);
            }
        }
        if rows.is_empty() {
            match self.views.last() {
                Some(view) if view.view.loading() => {}
                Some(view) => {
                    let empty = view.view.empty();
                    let (font, color) = if empty.description.is_some() { (theme::weighted(13.0, theme::MEDIUM), theme::FG()) } else { (theme::font(13.0), theme::FG_60()) };
                    let mut job = LayoutJob::single_section(empty.title.clone(), TextFormat { font_id: font, color, ..Default::default() });
                    job.wrap.max_width = rect.width() - 48.0;
                    job.halign = egui::Align::Center;
                    let galley = ctx.fonts_mut(|f| f.layout_job(job));
                    let top = rect.center().y - galley.size().y / 2.0 - if empty.description.is_some() { 10.0 } else { 0.0 };
                    let title = Rect::from_min_size(pos2(rect.center().x - galley.size().x / 2.0, top), galley.size());
                    ui.painter().galley(pos2(rect.center().x, top), galley, color);
                    if let Some(description) = empty.description {
                        let mut job = LayoutJob::single_section(description, TextFormat { font_id: theme::font(13.0), color: theme::FG_60(), ..Default::default() });
                        job.wrap.max_width = 420.0_f32.min(rect.width() - 48.0);
                        job.halign = egui::Align::Center;
                        let galley = ctx.fonts_mut(|f| f.layout_job(job));
                        ui.painter().galley(pos2(rect.center().x, title.bottom() + 4.0), galley, theme::FG_60());
                    }
                }
                None => {
                    let text = format!("No results for “{}”", self.query.trim());
                    paint_text(ui, rect.center(), Align2::CENTER_CENTER, &text, theme::font(13.0), theme::FG_60(), rect.width() - 32.0);
                }
            }
            return;
        }
        let inner = Rect::from_min_max(pos2(rect.left() + INSET, rect.top()), pos2(rect.right() - INSET, rect.bottom()));
        if let Some(columns) = self.views.last().and_then(|v| v.view.grid()) {
            return self.grid(ui, inner, rows, columns);
        }
        let mut clicked = None;
        let selected_index = *self.selection();
        let mut hovered_index = None;
        let pointer_moved = ui.input(|i| i.pointer.delta() != egui::Vec2::ZERO);
        let scroll = self.scroll_to_selected;
        ui.scope_builder(UiBuilder::new().max_rect(inner), |ui| {
            egui::ScrollArea::vertical().id_salt(("results", self.views.len())).auto_shrink(false).scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded).show(ui, |ui| {
                let mut previous_section: Option<&str> = None;
                for (index, row) in rows.iter().enumerate() {
                    if previous_section != Some(row.section.as_str()) {
                        previous_section = Some(row.section.as_str());
                        if row.section.is_empty() {
                            ui.add_space(8.0);
                        } else {
                            let (head, _) = ui.allocate_exact_size(vec2(ui.available_width(), SECTION), Sense::hover());
                            let title = paint_text(ui, pos2(head.left() + 8.0, head.bottom() - 5.0), Align2::LEFT_BOTTOM, &row.section, theme::weighted(11.0, theme::SEMIBOLD), theme::FG_60(), head.width());
                            if row.card.as_ref().is_some_and(|c| matches!(c.kind, CardKind::Weather(_))) {
                                let mut x = title.right() + 16.0;
                                for (week, label) in [(false, "Today"), (true, "This Week")] {
                                    let color = if self.weather_week == week { theme::FG() } else { theme::FG_40() };
                                    let tab = paint_text(ui, pos2(x, title.bottom()), Align2::LEFT_BOTTOM, label, theme::weighted(11.0, theme::SEMIBOLD), color, 120.0);
                                    let response = ui.interact(tab.expand(4.0), egui::Id::unique(("weather-tab", week)), Sense::click());
                                    response.widget_info(|| egui::WidgetInfo::selected(egui::Role::Tab, true, self.weather_week == week, label));
                                    if response.clicked() {
                                        self.weather_week = week;
                                    }
                                    x = tab.right() + 10.0;
                                }
                            } else if row.section.starts_with("Use “") {
                                icons::paint_glyph(ui, "settings", Rect::from_min_size(pos2(title.right() + 6.0, title.center().y - 6.0), vec2(12.0, 12.0)), theme::FG_40());
                            }
                        }
                    }
                    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), if row.card.is_some() { CARD } else { ROW }), Sense::click());
                    let selected = index == selected_index;
                    let label = match (&row.card, row.kind) {
                        (Some(card), _) => format!("{} = {}", card.input.text, row.title),
                        (None, Some(kind)) => format!("{}, {kind}", row.title),
                        (None, None) => row.title.clone(),
                    };
                    response.widget_info(|| egui::WidgetInfo::selected(egui::Role::ListBoxOption, true, selected, &label));
                    if selected {
                        ui.painter().rect_filled(rect, 8.0, theme::SELECTION());
                        if scroll {
                            response.scroll_to_me(None);
                        }
                    }
                    if response.hovered() && pointer_moved && !selected {
                        hovered_index = Some(index);
                    }
                    if response.clicked() {
                        clicked = Some(index);
                    }
                    if let Some(card) = &row.card {
                        paint_card(ui, rect, card, selected, self.weather_week);
                        continue;
                    }
                    let icon_width = if matches!(row.icon, Icon::Time(..)) { icons::TIME_WIDTH } else { 22.0 };
                    let icon_rect = Rect::from_min_size(pos2(rect.left() + 8.0, rect.center().y - 11.0), vec2(icon_width, 22.0));
                    icons::command_icon(&mut ui.new_child(UiBuilder::new().max_rect(icon_rect)), &row.icon, &row.title, 22.0);
                    let mut right = rect.right() - 8.0;
                    if let Some(kind) = row.kind {
                        right = paint_text(ui, pos2(right, rect.center().y), Align2::RIGHT_CENTER, kind, theme::font(13.0), theme::FG_60(), 200.0).left() - 12.0;
                    }
                    if !row.accessories.is_empty() {
                        right = accessories(ui, right, rect.center().y, &row.accessories) - 12.0;
                    }
                    let mut x = icon_rect.right() + 12.0;
                    let title = paint_text(ui, pos2(x, rect.center().y), Align2::LEFT_CENTER, &row.title, theme::weighted(13.0, theme::MEDIUM), theme::FG(), right - x);
                    x = title.right() + 12.0;
                    if let Some(subtitle) = &row.subtitle {
                        if x < right {
                            paint_text(ui, pos2(x, rect.center().y), Align2::LEFT_CENTER, subtitle, theme::font(13.0), theme::FG_60(), right - x);
                        }
                    }
                }
                ui.add_space(INSET);
            });
        });
        self.scroll_to_selected = false;
        if let Some(index) = hovered_index {
            *self.selection() = index;
        }
        if let Some(index) = clicked {
            *self.selection() = index;
            let row = &rows[index];
            if let Some(action) = self.groups(row).first().and_then(|g| g.first()).cloned() {
                self.run(&ctx, action, Self::row_key(row), false);
            }
        }
    }

    fn grid(&mut self, ui: &mut Ui, inner: Rect, rows: &[Row], columns: usize) {
        let ctx = ui.ctx().clone();
        let (selected_index, scroll) = (*self.selection(), self.scroll_to_selected);
        let pointer_moved = ui.input(|i| i.pointer.delta() != egui::Vec2::ZERO);
        let (mut clicked, mut hovered) = (None, None);
        ui.scope_builder(UiBuilder::new().max_rect(inner), |ui| {
            egui::ScrollArea::vertical().id_salt(("results", self.views.len())).auto_shrink(false).scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded).show(ui, |ui| {
                let width = ui.available_width();
                let cell = ((width - GRID_GAP * (columns as f32 - 1.0)) / columns as f32).floor();
                for range in section_ranges(rows) {
                    let title = rows[range.start].section.as_str();
                    if title.is_empty() {
                        ui.add_space(8.0);
                    } else {
                        let (head, _) = ui.allocate_exact_size(vec2(width, SECTION), Sense::hover());
                        let label = paint_text(ui, pos2(head.left() + 8.0, head.bottom() - 5.0), Align2::LEFT_BOTTOM, title, theme::weighted(11.0, theme::SEMIBOLD), theme::FG_60(), head.width());
                        paint_text(ui, pos2(label.right() + 8.0, head.bottom() - 5.0), Align2::LEFT_BOTTOM, &range.len().to_string(), theme::weighted(11.0, theme::MEDIUM), theme::FG_40(), 80.0);
                    }
                    let starts: Vec<usize> = range.clone().step_by(columns.max(1)).collect();
                    for (n, start) in starts.iter().enumerate() {
                        let (line, _) = ui.allocate_exact_size(vec2(width, cell), Sense::hover());
                        for index in *start..(*start + columns).min(range.end) {
                            let row = &rows[index];
                            let r = Rect::from_min_size(pos2(line.left() + (index - start) as f32 * (cell + GRID_GAP), line.top()), vec2(cell, cell));
                            let response = ui.interact(r, ui.scope_id().with(("cell", index)), Sense::click());
                            let selected = index == selected_index;
                            response.widget_info(|| egui::WidgetInfo::selected(egui::Role::ListBoxOption, true, selected, &row.title));
                            ui.painter().rect_filled(r, 8.0, if selected { theme::SELECTION() } else { theme::fg(5) });
                            if selected {
                                ui.painter().rect_stroke(r, 8.0, Stroke::new(2.0, theme::FG_60()), egui::StrokeKind::Inside);
                                if scroll {
                                    response.scroll_to_me(None);
                                }
                            }
                            if response.hovered() && pointer_moved && !selected {
                                hovered = Some(index);
                            }
                            if response.clicked() {
                                clicked = Some(index);
                            }
                            if ui.is_rect_visible(r) {
                                if matches!(row.icon, Icon::Thumbnail(_)) {
                                    let size = cell - 36.0;
                                    let icon = Rect::from_center_size(pos2(r.center().x, r.top() + 8.0 + size / 2.0), vec2(size, size));
                                    icons::command_icon(&mut ui.new_child(UiBuilder::new().max_rect(icon)), &row.icon, &row.title, size);
                                    paint_text(ui, pos2(r.center().x, r.bottom() - 7.0), Align2::CENTER_BOTTOM, &row.title, theme::font(11.0), theme::FG_60(), cell - 12.0);
                                } else {
                                    let icon = Rect::from_center_size(r.center(), vec2(GRID_ICON, GRID_ICON));
                                    icons::command_icon(&mut ui.new_child(UiBuilder::new().max_rect(icon)), &row.icon, &row.title, GRID_ICON);
                                }
                            }
                        }
                        if n + 1 < starts.len() {
                            ui.add_space(GRID_GAP);
                        }
                    }
                }
                ui.add_space(INSET);
            });
        });
        self.scroll_to_selected = false;
        if let Some(index) = hovered {
            *self.selection() = index;
        }
        if let Some(index) = clicked {
            let was_selected = index == selected_index;
            *self.selection() = index;
            if was_selected {
                if let Some(action) = self.groups(&rows[index]).first().and_then(|g| g.first()).cloned() {
                    self.run(&ctx, action, Self::row_key(&rows[index]), false);
                }
            }
        }
    }

    fn page_ui(&mut self, ui: &mut Ui, page: Page) {
        let ctx = ui.ctx().clone();
        let full = ui.max_rect();
        ui.painter().rect_filled(full, 0.0, theme::BG());
        let title = if page.title.is_empty() { self.views.last().map(|v| v.view.footer().0).unwrap_or_default() } else { page.title.clone() };
        let row = Row { id: "page".into(), section: String::new(), title, subtitle: None, icon: Icon::Glyph("file"), accessories: Vec::new(), kind: None, source: Source::Item, actions: page.actions.clone(), card: None };
        let groups = self.groups(&row);
        if self.pending.is_some() {
            self.confirm_keys(ui);
        } else if self.panel.is_some() {
            self.panel_keys(ui);
        } else if self.dropdown.is_some() {
            self.dropdown_keys(ui);
        } else {
            let mut back = false;
            for (key, modifiers) in key_presses(ui) {
                if key == Key::K && modifiers.ctrl && !modifiers.alt && !modifiers.shift {
                    if !groups.is_empty() {
                        self.panel = Some(Panel { title: row.title.clone(), groups: groups.clone(), selected: 0, filter: String::new(), command: None });
                    }
                } else if key == Key::Enter && plain(modifiers) && page.enter_runs_primary {
                    if let Some(action) = groups.first().and_then(|g| g.first()) {
                        self.run(&ctx, action.clone(), None, false);
                    }
                } else if let Some(action) = groups.iter().flatten().find(|a| a.shortcut.is_some_and(|s| s.matches(key, modifiers))) {
                    self.run(&ctx, action.clone(), None, false);
                } else if key == Key::Escape && plain(modifiers) {
                    back = true;
                } else if key == Key::Backspace && plain(modifiers) && page.backspace_pops {
                    back = true;
                } else {
                    continue;
                }
                consume(ui, key, modifiers);
            }
            if back {
                self.pop_view();
                return;
            }
        }
        let header_height = if page.dropdowns.is_empty() { HEADER } else { 54.0 };
        let header = Rect::from_min_size(full.min, vec2(full.width(), header_height));
        let footer = Rect::from_min_max(pos2(full.left(), full.bottom() - FOOTER), full.max);
        let content = Rect::from_min_max(pos2(full.left(), if page.dropdowns.is_empty() { header.bottom() } else { full.top() }), pos2(full.right(), footer.top()));
        if let Some(top) = self.views.last_mut() {
            top.view.page_ui(&mut ui.new_child(UiBuilder::new().max_rect(content)), content);
        }
        let lead = Rect::from_center_size(pos2(header.left() + 16.0 + 11.0, header.center().y), vec2(22.0, 22.0));
        if page.dropdowns.is_empty() {
            ui.painter().hline(header.x_range(), header.bottom() - 0.5, Stroke::new(1.0, theme::SEPARATOR()));
            paint_text(ui, pos2(lead.right() + 12.0, header.center().y), Align2::LEFT_CENTER, &page.title, theme::weighted(13.0, theme::MEDIUM), theme::FG_60(), header.width() - 80.0);
        }
        let mut picks = Vec::new();
        let count = page.dropdowns.len() as f32;
        for (slot, dropdown) in page.dropdowns.iter().enumerate() {
            let center = pos2(full.left() + full.width() / count * (slot as f32 + 0.5), header.center().y);
            let trigger = Rect::from_center_size(center, vec2(DROPDOWN_WIDTH, 34.0));
            let response = Self::dropdown_trigger(ui, trigger, slot);
            if let Some(choice) = self.dropdown_ui(ui, response, slot, dropdown.label, &dropdown.options, dropdown.selected) {
                picks.push((slot, choice));
            }
        }
        if let Some(top) = self.views.last_mut() {
            for (slot, choice) in picks {
                top.view.page_select(slot, choice);
            }
        }
        if back_button(ui, lead) {
            self.pop_view();
            return;
        }
        let keys = (!page.enter_runs_primary).then(|| groups.first().and_then(|g| g.first()).and_then(|a| a.shortcut).map(|s| s.labels())).flatten();
        self.footer(ui, footer, Some(&row), &groups, keys);
        if self.panel.is_some() {
            self.panel_ui(ui, footer.top());
        }
        if self.pending.is_some() {
            self.confirm_ui(ui, full);
        }
    }

    fn footer(&mut self, ui: &mut Ui, rect: Rect, current: Option<&Row>, groups: &[Vec<Action>], primary_keys: Option<Vec<String>>) {
        ui.painter().hline(rect.x_range(), rect.top() + 0.5, Stroke::new(1.0, theme::SEPARATOR()));
        let center_y = rect.center().y;
        if let Some((toast, _)) = &self.toast {
            let (color, title, message) = match toast {
                Toast::Success(title) => (theme::GREEN(), title.as_str(), None),
                Toast::Failure(title, message) => (theme::RED(), title.as_str(), Some(message.as_str())),
            };
            let dot = pos2(rect.left() + 22.0, center_y);
            for (radius, alpha) in [(10.0, 0.06), (6.0, 0.12)] {
                ui.painter().circle_filled(dot, radius, color.gamma_multiply(alpha));
            }
            ui.painter().circle_filled(dot, 2.5, color);
            let title_rect = paint_text(ui, pos2(rect.left() + 37.0, center_y), Align2::LEFT_CENTER, title, theme::font(12.0), theme::FG(), 360.0);
            if let Some(message) = message.filter(|m| !m.is_empty()) {
                paint_text(ui, pos2(title_rect.right() + 7.0, center_y), Align2::LEFT_CENTER, message, theme::font(13.0), theme::FG_60(), 300.0);
            }
        } else if let Some(view) = self.views.last() {
            let (mut title, icon) = view.view.footer();
            if let Some(row) = current.filter(|_| view.view.grid().is_some()) {
                title = format!("{title} – {}", row.title);
            }
            let tile = Rect::from_min_size(pos2(rect.left() + 12.0, center_y - 8.0), vec2(16.0, 16.0));
            icons::command_icon(&mut ui.new_child(UiBuilder::new().max_rect(tile)), &icon, &title, 16.0);
            paint_text(ui, pos2(tile.right() + 8.0, center_y), Align2::LEFT_CENTER, &title, theme::weighted(13.0, theme::MEDIUM), theme::FG(), 300.0);
        } else if let Some(error) = &self.hotkey_error {
            paint_text(ui, pos2(rect.left() + 12.0, center_y), Align2::LEFT_CENTER, &format!("Hotkey unavailable: {error}"), theme::font(12.0), theme::RED(), 380.0);
        } else {
            icons::paint_glyph(ui, "openray", Rect::from_center_size(pos2(rect.left() + 12.0 + 11.0, center_y), vec2(22.0, 22.0)), theme::FG_60());
        }
        let (Some(row), Some(primary)) = (current, groups.first().and_then(|g| g.first())) else { return };
        let right = rect.right() - 9.0;
        let mut x = right;
        if groups.iter().flatten().count() > 1 {
            let caps_left = keycaps(ui, right - 3.0, center_y, &["Ctrl".into(), "K".into()]);
            let label = paint_text(ui, pos2(caps_left - 8.0, center_y), Align2::RIGHT_CENTER, "Actions", theme::weighted(13.0, theme::MEDIUM), theme::FG_60(), 200.0);
            let button = Rect::from_min_max(pos2(label.left() - 8.0, center_y - 13.0), pos2(right, center_y + 13.0));
            let response = ui.interact(button, egui::Id::unique("actions-button"), Sense::click());
            response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, "Actions"));
            if self.panel.is_some() {
                ui.painter().rect_filled(button, 6.0, theme::SELECTION());
                keycaps(ui, right - 3.0, center_y, &["Ctrl".into(), "K".into()]);
                paint_text(ui, pos2(caps_left - 8.0, center_y), Align2::RIGHT_CENTER, "Actions", theme::weighted(13.0, theme::MEDIUM), theme::FG_60(), 200.0);
            } else {
                ui.painter().vline(button.left() - 6.0, (center_y - 5.5)..=(center_y + 5.5), Stroke::new(1.0, theme::FOOTER_DIVIDER()));
            }
            if response.clicked() {
                self.panel = match self.panel {
                    Some(_) => None,
                    None => Some(Panel { title: row.title.clone(), groups: groups.to_vec(), selected: 0, filter: String::new(), command: Self::row_key(row) }),
                };
            }
            x = button.left() - 13.0;
        }
        let caps_left = keycaps(ui, x, center_y, &primary_keys.unwrap_or_else(|| vec!["↵".into()]));
        paint_text(ui, pos2(caps_left - 8.0, center_y), Align2::RIGHT_CENTER, &primary.title, theme::weighted(13.0, theme::MEDIUM), theme::FG(), 300.0);
    }

    fn panel_ui(&mut self, ui: &mut Ui, footer_top: f32) {
        let ctx = ui.ctx().clone();
        let groups = self.filtered_panel();
        let Some(panel) = &mut self.panel else { return };
        let full = ui.max_rect();
        let rows: usize = groups.iter().map(Vec::len).sum();
        let list_height = (24.0 + rows as f32 * 32.0 + groups.len().saturating_sub(1) as f32 * 16.0 + 15.0).min(PANEL_MAX_LIST + 15.0);
        let height = list_height + 38.0 + 2.0;
        let frame = Rect::from_min_size(pos2(full.right() - 8.0 - PANEL_WIDTH, footer_top - 9.0 - height), vec2(PANEL_WIDTH, height));
        panel.selected = panel.selected.min(rows.saturating_sub(1));
        let mut run = None;
        let mut close = false;
        egui::Area::new(egui::Id::unique("action-panel")).order(egui::Order::Foreground).fixed_pos(frame.min).show(&ctx, |ui| {
            let painter = ui.painter();
            painter.add(egui::epaint::RectShape::filled(frame.translate(vec2(0.0, 8.0)).expand(4.0), 10.0, Color32::from_black_alpha(40)));
            painter.rect(frame, 6.0, theme::PANEL(), Stroke::new(1.0, theme::PANEL_BORDER()), egui::StrokeKind::Inside);
            let list = Rect::from_min_max(frame.min + vec2(1.0, 1.0), pos2(frame.right() - 1.0, frame.top() + 1.0 + list_height));
            ui.scope_builder(UiBuilder::new().max_rect(list), |ui| {
                egui::ScrollArea::vertical().id_salt("panel-list").scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden).max_height(list_height).show(ui, |ui| {
                    ui.add_space(7.0);
                    let width = ui.available_width();
                    let (title_row, _) = ui.allocate_exact_size(vec2(width, 24.0), Sense::hover());
                    paint_text(ui, pos2(title_row.left() + 7.0 + 8.0, title_row.center().y), Align2::LEFT_CENTER, &panel.title, theme::weighted(11.0, theme::SEMIBOLD), theme::FG_60(), width - 30.0);
                    let mut index = 0;
                    let pointer_moved = ui.input(|i| i.pointer.delta() != egui::Vec2::ZERO);
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
                            let is_selected = index == panel.selected;
                            response.widget_info(|| egui::WidgetInfo::selected(egui::Role::ListBoxOption, true, is_selected, &action.title));
                            if is_selected {
                                ui.painter().rect_filled(row, 6.0, theme::PANEL_LINE());
                            }
                            if response.hovered() && pointer_moved {
                                panel.selected = index;
                            }
                            if response.clicked() {
                                run = Some(action.clone());
                            }
                            let color = if action.destructive { theme::RED() } else { theme::FG() };
                            if let Some(icon) = action.icon {
                                icons::paint_glyph(ui, icon, Rect::from_min_size(pos2(row.left() + 8.0, row.center().y - 8.0), vec2(16.0, 16.0)), color);
                            }
                            let mut end = action.shortcut.map(|s| keycaps(ui, row.right() - 8.0, row.center().y, &s.labels())).unwrap_or(row.right() - 8.0);
                            if let Some(accessory) = &action.accessory {
                                end = paint_text(ui, pos2(end - 8.0, row.center().y), Align2::RIGHT_CENTER, accessory, theme::font(12.0), theme::FG_60(), 120.0).left();
                            }
                            paint_text(ui, pos2(row.left() + 32.0, row.center().y), Align2::LEFT_CENTER, &action.title, theme::font(13.0), color, end - row.left() - 40.0);
                            index += 1;
                        }
                    }
                    ui.add_space(8.0);
                });
            });
            let search = Rect::from_min_max(pos2(frame.left() + 1.0, frame.bottom() - 39.0), pos2(frame.right() - 1.0, frame.bottom() - 1.0));
            ui.painter().hline(search.x_range(), search.top() + 0.5, Stroke::new(1.0, theme::PANEL_LINE()));
            let id = egui::Id::unique("panel-search");
            let before = panel.filter.clone();
            let field = search.shrink2(vec2(15.0, 0.0));
            let edit = egui::TextEdit::singleline(&mut panel.filter).id(id).frame(egui::Frame::NONE).margin(egui::Margin::ZERO).font(theme::font(13.0)).hint_text(egui::RichText::new(crate::i18n::tr("Search for actions...")).color(theme::FG_40())).desired_width(field.width());
            ui.put(Rect::from_center_size(field.center(), vec2(field.width(), 18.0)), edit);
            ui.memory_mut(|m| m.request_focus(id));
            if panel.filter != before {
                panel.selected = 0;
            }
        });
        if ctx.input(|i| i.pointer.any_click()) && ctx.input(|i| i.pointer.interact_pos()).is_some_and(|p| !frame.contains(p) && p.y < footer_top) {
            close = true;
        }
        let command = panel.command.clone();
        if let Some(action) = run {
            self.run(&ctx, action, command, false);
        } else if close {
            self.panel = None;
        }
    }

    fn confirm_ui(&mut self, ui: &mut Ui, full: Rect) {
        let ctx = ui.ctx().clone();
        let Some(pending) = &self.pending else { return };
        let target = egui::Id::unique(if pending.ok_focused { "confirm-ok" } else { "confirm-cancel" });
        let (title, message, confirm_title) = (pending.confirmation.title.clone(), pending.confirmation.message.clone(), pending.confirmation.confirm_title.clone());
        let mut answer = None;
        egui::Area::new(egui::Id::unique("confirm")).order(egui::Order::Foreground).fixed_pos(full.min).show(&ctx, |ui| {
            ui.painter().rect_filled(full, 0.0, Color32::from_black_alpha(102));
            let width = 300.0;
            let message_galley = {
                let mut job = LayoutJob::single_section(message.clone(), TextFormat { font_id: theme::font(13.0), color: theme::FG_60(), ..Default::default() });
                job.wrap.max_width = width - 32.0;
                job.halign = egui::Align::Center;
                ctx.fonts_mut(|f| f.layout_job(job))
            };
            let height = 16.0 + 18.0 + 4.0 + message_galley.size().y + 16.0 + 32.0 + 8.0 + 32.0 + 16.0;
            let panel = Rect::from_center_size(full.center(), vec2(width, height));
            ui.painter().rect(panel, 6.0, theme::PANEL(), Stroke::new(1.0, theme::PANEL_BORDER()), egui::StrokeKind::Inside);
            paint_text(ui, pos2(panel.center().x, panel.top() + 16.0), Align2::CENTER_TOP, &title, theme::weighted(13.0, theme::SEMIBOLD), theme::FG(), width - 32.0);
            ui.painter().galley(pos2(panel.center().x, panel.top() + 38.0), message_galley.clone(), theme::FG_60());
            let mut y = panel.top() + 38.0 + message_galley.size().y + 16.0;
            for (id, label, color, ok) in [("confirm-ok", confirm_title.as_str(), theme::RED(), true), ("confirm-cancel", "Cancel", theme::FG(), false)] {
                let rect = Rect::from_min_size(pos2(panel.left() + 16.0, y), vec2(width - 32.0, 32.0));
                let response = ui.interact(rect, egui::Id::unique(id), Sense::click());
                response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, label));
                ui.painter().rect_filled(rect, 6.0, if response.hovered() { theme::CONTROL_HOVER() } else { theme::CONTROL() });
                if response.has_focus() {
                    ui.painter().rect_stroke(rect.expand(2.0), 8.0, Stroke::new(2.0, theme::ACCENT()), egui::StrokeKind::Outside);
                }
                paint_text(ui, rect.center(), Align2::CENTER_CENTER, label, theme::weighted(13.0, theme::MEDIUM), color, rect.width());
                if response.clicked() {
                    answer = Some(ok);
                }
                y += 40.0;
            }
            ctx.memory_mut(|m| m.request_focus(target));
        });
        match answer {
            Some(true) => {
                if let Some(p) = self.pending.take() {
                    self.run(&ctx, p.action, p.command, true);
                }
            }
            Some(false) => self.pending = None,
            None => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loader_waits_fades_in_and_sweeps_back_and_forth() {
        assert_eq!(loader_frame(0.2), None);
        let (start, faint) = loader_frame(LOADER_DELAY).unwrap();
        assert!((start + 0.35).abs() < 1e-6 && faint == 0.0);
        let (middle, full) = loader_frame(LOADER_DELAY + LOADER_SWEEP / 2.0).unwrap();
        assert!(middle.abs() < 1e-6 && full == 1.0);
        assert!((loader_frame(LOADER_DELAY + LOADER_SWEEP).unwrap().0 - 0.35).abs() < 1e-6);
        assert!((loader_frame(LOADER_DELAY + 2.0 * LOADER_SWEEP).unwrap().0 + 0.35).abs() < 1e-6);
        assert!((loader_frame(LOADER_DELAY + LOADER_FADE / 2.0).unwrap().1 - 0.5).abs() < 1e-6);
    }
}
