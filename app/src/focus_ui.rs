use crate::extension::{Icon, Tone};
use crate::focus::{self, BlockInfo, Notice, Session, Status};
use crate::root::Signal;
use crate::{icons, launcher, theme};
use egui::{pos2, vec2, Align2, Color32, Pos2, Rect, Sense, Stroke, Ui, UiBuilder};
use std::sync::mpsc::Sender;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{HWND, POINT, RECT};
use windows::Win32::Graphics::Dwm::{DWMWA_BORDER_COLOR, DWMWA_TRANSITIONS_FORCEDISABLED, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND};
use windows::Win32::Graphics::Gdi::{GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTONEAREST};
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowW, GetCursorPos, GetWindowLongPtrW, GetWindowRect, GetWindowThreadProcessId, SetWindowLongPtrW, SetWindowPos, GWL_EXSTYLE, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOSIZE, SWP_NOZORDER,
    SWP_SHOWWINDOW, WS_EX_APPWINDOW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
};

pub const ICON: Icon = Icon::Tile("target", Tone::Green);
const PANEL_SIZE: egui::Vec2 = vec2(265.0, 40.0);
const TOAST_SIZE: egui::Vec2 = vec2(392.0, 66.0);
const PANEL_BOTTOM_MARGIN: f32 = 24.0;
const TOAST_TOP_MARGIN: f32 = 24.0;
const TOAST_SECONDS: u64 = 6;
const YELLOW: Color32 = Color32::from_rgb(0xFF, 0xC5, 0x31);
const OVERLAY: Color32 = Color32::from_rgb(0x05, 0x05, 0x05);
const PANEL_TITLE: &str = "Openray Focus";
const TOAST_TITLE: &str = "Openray Focus Notice";
const BLOCK_TITLE: &str = "Openray Focus Block";

pub static SIGNALS: Mutex<Option<Sender<Signal>>> = Mutex::new(None);

#[derive(Default)]
struct Windows {
    panel: bool,
    panel_at: Option<Pos2>,
    panel_saved: Option<Pos2>,
    panel_styled: bool,
    panel_drag: (i32, i32),
    toast: Option<(Notice, Instant)>,
    toast_styled: bool,
    block: Option<(BlockInfo, crate::window_layout::Rect)>,
    block_placed: bool,
    block_focused: bool,
    raise: bool,
}

static WINDOWS: Mutex<Option<Windows>> = Mutex::new(None);

fn with<T>(f: impl FnOnce(&mut Windows) -> T) -> T {
    let mut guard = WINDOWS.lock().unwrap_or_else(|e| e.into_inner());
    f(guard.get_or_insert_with(Windows::default))
}

#[derive(Clone)]
pub struct EguiHost(pub egui::Context);

pub fn host(ctx: &egui::Context) -> EguiHost {
    EguiHost(ctx.clone())
}

impl focus::Host for EguiHost {
    fn changed(&self, _session: Option<&Session>) {
        crate::extensions::session_changed();
        self.0.request_repaint();
    }

    fn main(&self, work: impl FnOnce(&Self) + Send + 'static) {
        work(self);
        self.0.request_repaint();
    }

    fn show_panel(&self) {
        with(|w| {
            if !w.panel {
                w.panel = true;
                w.panel_styled = false;
                w.panel_at = Some(w.panel_saved.unwrap_or_else(default_panel_position));
            }
        });
        self.0.request_repaint();
    }

    fn hide_panel(&self) {
        with(|w| w.panel = false);
        self.0.request_repaint();
    }

    fn show_toast(&self, notice: Notice) {
        with(|w| {
            w.toast = Some((notice, Instant::now()));
            w.toast_styled = false;
        });
        self.0.request_repaint();
    }

    fn hide_toast(&self) {
        with(|w| w.toast = None);
        self.0.request_repaint();
    }

    fn show_block(&self, block: &BlockInfo, area: crate::window_layout::Rect) {
        with(|w| {
            w.block = Some((block.clone(), area));
            w.block_placed = false;
            w.block_focused = false;
        });
        self.0.request_repaint();
    }

    fn hide_block(&self) {
        with(|w| w.block = None);
        self.0.request_repaint();
    }

    fn raise_block(&self) {
        with(|w| w.raise = true);
        self.0.request_repaint();
    }
}

pub fn cursor_area() -> (Rect, f32) {
    unsafe {
        let mut cursor = POINT::default();
        let _ = GetCursorPos(&mut cursor);
        let monitor = MonitorFromPoint(cursor, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
        let _ = GetMonitorInfoW(monitor, &mut info);
        let (mut dpi_x, mut dpi_y) = (96, 96);
        let _ = GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y);
        let scale = dpi_x as f32 / 96.0;
        let area = info.rcWork;
        (Rect::from_min_max(pos2(area.left as f32 / scale, area.top as f32 / scale), pos2(area.right as f32 / scale, area.bottom as f32 / scale)), scale)
    }
}

fn default_panel_position() -> Pos2 {
    let (area, _) = cursor_area();
    pos2(area.center().x - PANEL_SIZE.x / 2.0, area.bottom() - PANEL_SIZE.y - PANEL_BOTTOM_MARGIN)
}

pub fn own_window(title: &str) -> Option<HWND> {
    let text: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
    let hwnd = unsafe { FindWindowW(PCWSTR::null(), PCWSTR(text.as_ptr())) }.ok()?;
    let mut pid = 0;
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    (pid == std::process::id()).then_some(hwnd)
}

fn popup_styles(hwnd: HWND, activatable: bool) {
    launcher::set_dwm(hwnd, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND);
    launcher::set_dwm(hwnd, DWMWA_BORDER_COLOR, theme::WINDOW_BORDER());
    launcher::set_dwm(hwnd, DWMWA_TRANSITIONS_FORCEDISABLED, 1i32);
    unsafe {
        let mut ex = (GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32 & !WS_EX_APPWINDOW.0) | WS_EX_TOOLWINDOW.0;
        if !activatable {
            ex |= WS_EX_NOACTIVATE.0;
        }
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex as isize);
    }
}

pub fn popup(title: &str, size: egui::Vec2, position: Pos2, active: bool) -> egui::ViewportBuilder {
    egui::ViewportBuilder::default()
        .with_title(title)
        .with_inner_size(size)
        .with_position(position)
        .with_decorations(false)
        .with_always_on_top()
        .with_taskbar(false)
        .with_resizable(false)
        .with_active(active)
}

pub fn paint_text(ui: &Ui, anchor: Pos2, align: Align2, text: &str, font: egui::FontId, color: Color32, max_width: f32) -> Rect {
    let mut job = egui::text::LayoutJob::simple_singleline(crate::i18n::tr(text).into_owned(), font, color);
    job.wrap = egui::text::TextWrapping::truncate_at_width(max_width);
    let galley = ui.fonts_mut(|f| f.layout_job(job));
    let rect = align.anchor_size(anchor, galley.size());
    ui.painter().galley(rect.min, galley, color);
    rect
}

fn button(ui: &mut Ui, rect: Rect, id: &str, text: &str, glyph: Option<&str>, font_size: f32) -> egui::Response {
    let response = ui.interact(rect, egui::Id::unique(("focus-button", id)), Sense::click());
    let fill = if response.hovered() || response.is_pointer_button_down_on() { theme::CONTROL_HOVER() } else { theme::CONTROL() };
    ui.painter().rect_filled(rect, 6.0, fill);
    if response.has_focus() {
        ui.painter().rect_stroke(rect.expand(2.0), 8.0, Stroke::new(1.0, theme::FG_40()), egui::StrokeKind::Outside);
    }
    let font = theme::weighted(font_size, theme::MEDIUM);
    match glyph {
        Some(name) if text.is_empty() => icons::paint_glyph(ui, name, Rect::from_center_size(rect.center(), vec2(14.0, 14.0)), theme::FG()),
        Some(name) => {
            let label = paint_text(ui, pos2(rect.left() + 8.0, rect.center().y), Align2::LEFT_CENTER, text, font, theme::FG(), rect.width());
            icons::paint_glyph(ui, name, Rect::from_center_size(pos2(label.right() + 6.0 + 7.0, rect.center().y), vec2(14.0, 14.0)), theme::FG());
        }
        None => {
            paint_text(ui, rect.center(), Align2::CENTER_CENTER, text, font, theme::FG(), rect.width());
        }
    }
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, if text.is_empty() { id } else { text }));
    response
}

fn button_width(ui: &Ui, text: &str, glyph: bool, font_size: f32) -> f32 {
    let width = ui.fonts_mut(|f| f.layout_no_wrap(crate::i18n::tr(&text).into_owned(), theme::weighted(font_size, theme::MEDIUM), theme::FG())).size().x;
    width + if glyph { 8.0 + 6.0 + 14.0 + 6.0 } else { 24.0 }
}

pub fn format_timer(ms: i64) -> String {
    let seconds = (ms.max(0) as u64).div_ceil(1000);
    let (hours, minutes) = (seconds / 3600, (seconds % 3600) / 60);
    if hours > 0 {
        format!("{hours:02}:{minutes:02}")
    } else {
        format!("{minutes:02}:{:02}", seconds % 60)
    }
}

pub fn timer_step(ms: u64) -> u64 {
    if ms <= 600_000 {
        1000
    } else if ms >= 3_600_000 {
        60_000
    } else {
        5000
    }
}

pub fn remaining_ms(session: &Session, now: u64) -> u64 {
    match (session.duration_ms, session.status, session.end_at) {
        (None, _, _) => 0,
        (_, Status::Paused, _) | (_, _, None) => session.remaining_ms,
        (_, _, Some(end)) => end.saturating_sub(now),
    }
}

pub fn pause_remaining_ms(session: &Session, now: u64) -> Option<u64> {
    (session.status == Status::Paused).then_some(session.pause_ends_at?.saturating_sub(now))
}

pub fn elapsed_fraction(session: &Session, now: u64) -> f32 {
    match session.duration_ms {
        Some(total) if total > 0 => ((total.saturating_sub(remaining_ms(session, now))) as f32 / total as f32).clamp(0.0, 1.0),
        _ => 0.0,
    }
}

pub fn shown_timer(session: &Session, now: u64) -> Option<String> {
    let pause_left = pause_remaining_ms(session, now);
    if session.duration_ms.is_none() && pause_left.is_none_or(|left| left == 0) {
        return None;
    }
    let shown = pause_left.unwrap_or_else(|| remaining_ms(session, now));
    let step = timer_step(shown);
    Some(format_timer(if shown == 0 { 0 } else { shown.div_ceil(step) * step } as i64))
}

fn send(signal: Signal, ctx: &egui::Context) {
    if let Some(sender) = SIGNALS.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
        let _ = sender.send(signal);
    }
    ctx.request_repaint();
}

fn menu(ctx: &egui::Context, which: &str, anchor: Rect, pixels_per_point: f32) {
    let Some(hwnd) = own_window(PANEL_TITLE) else { return };
    let mut window = RECT::default();
    unsafe {
        let _ = GetWindowRect(hwnd, &mut window);
    }
    let point = POINT { x: window.left + (anchor.left() * pixels_per_point).round() as i32, y: window.top + ((anchor.top() - 6.0) * pixels_per_point).round() as i32 };
    let Ok(choice) = focus::track_menu(hwnd, point, &focus::menu_items(which)) else { return };
    let host = host(ctx);
    let result = match (which, choice) {
        (_, 0) => Ok(()),
        ("pause", 99) => focus::pause(&host, None),
        ("pause", minutes) => focus::pause(&host, Some(u64::from(minutes) * 60_000)),
        (_, 1) => {
            send(Signal::Hotkey("focus/edit".into()), ctx);
            Ok(())
        }
        (_, 2) => focus::cancel(&host),
        (_, 3) => {
            reset_panel(ctx);
            Ok(())
        }
        _ => Ok(()),
    };
    if let Err(e) = result {
        crate::extension::notify(ctx, crate::extension::Toast::Failure(e, String::new()));
    }
}

fn reset_panel(ctx: &egui::Context) {
    let position = default_panel_position();
    with(|w| w.panel_saved = None);
    ctx.send_viewport_cmd_to(egui::ViewportId::from_hash_of("focus-panel"), egui::ViewportCommand::OuterPosition(position));
}

pub fn show(ctx: &egui::Context) {
    show_panel(ctx);
    show_toast(ctx);
    show_block(ctx);
}

fn show_panel(ctx: &egui::Context) {
    let Some(position) = with(|w| w.panel.then_some(w.panel_at).flatten()) else { return };
    let Some(session) = focus::get() else { return };
    let now = focus::now_ms();
    crate::extensions::session_tick(&session, now);
    ctx.request_repaint_after(Duration::from_millis(1000 - now % 1000));
    ctx.show_viewport_immediate(egui::ViewportId::from_hash_of("focus-panel"), popup(PANEL_TITLE, PANEL_SIZE, position, false), |ui, _| {
        if !with(|w| w.panel_styled) {
            if let Some(hwnd) = own_window(PANEL_TITLE) {
                popup_styles(hwnd, false);
                with(|w| w.panel_styled = true);
            }
        }
        if let Some(outer) = ui.input(|i| i.viewport().outer_rect) {
            with(|w| w.panel_saved = Some(outer.min));
        }
        let ctx = ui.ctx().clone();
        let rect = ui.max_rect();
        let paused = session.status == Status::Paused;
        ui.painter().rect_filled(rect, 0.0, theme::PANEL());
        let progress = elapsed_fraction(&session, now);
        if progress > 0.0 {
            let fill = Rect::from_min_size(rect.min, vec2((rect.width() * progress).round(), rect.height()));
            ui.painter().rect_filled(fill, 0.0, (if paused { YELLOW } else { theme::GREEN() }).gamma_multiply(0.28));
        }
        let hovered = ui.input(|i| i.pointer.any_down() || i.pointer.hover_pos().is_some_and(|p| rect.contains(p))) || focus::MENU_OPEN.load(std::sync::atomic::Ordering::SeqCst);
        let y = rect.center().y;
        if !hovered {
            let mut x = rect.left() + 8.0;
            match shown_timer(&session, now) {
                Some(timer) => {
                    let width = ui.fonts_mut(|f| f.layout_no_wrap(crate::i18n::tr(&timer).into_owned(), theme::mono(12.0), theme::FG_60())).size().x + 24.0;
                    let pill = Rect::from_min_size(pos2(x, y - 12.0), vec2(width, 24.0));
                    ui.painter().rect_filled(pill, 6.0, theme::CONTROL());
                    paint_text(ui, pill.center(), Align2::CENTER_CENTER, &timer, theme::mono(12.0), theme::FG_60(), width);
                    x = pill.right() + 8.0;
                }
                None => {
                    let tile = Rect::from_min_size(pos2(x, y - 11.0), vec2(22.0, 22.0));
                    icons::command_icon(&mut ui.new_child(UiBuilder::new().max_rect(tile)), &ICON, "Focus", 22.0);
                    x = tile.right() + 8.0;
                }
            }
            paint_text(ui, pos2(x, y), Align2::LEFT_CENTER, &session.title, theme::font(13.0), theme::FG(), rect.right() - 8.0 - x);
            return;
        }
        let ppp = ui.ctx().pixels_per_point();
        let mut x = rect.left() + 8.0;
        let (pause_text, pause_glyph) = if paused { ("Resume", "circle-play") } else { ("Pause", "circle-pause") };
        let width = button_width(ui, pause_text, true, 12.0);
        let pause = Rect::from_min_size(pos2(x, y - 12.0), vec2(width, 24.0));
        if button(ui, pause, "pause", pause_text, Some(pause_glyph), 12.0).clicked() {
            if paused {
                let _ = focus::resume(&host(&ctx));
            } else {
                menu(&ctx, "pause", pause, ppp);
            }
        }
        x = pause.right() + 4.0;
        let width = button_width(ui, "Complete", true, 12.0);
        let complete = Rect::from_min_size(pos2(x, y - 12.0), vec2(width, 24.0));
        if button(ui, complete, "complete", "Complete", Some("circle-check"), 12.0).clicked() {
            let _ = focus::complete(&host(&ctx));
        }
        x = complete.right() + 4.0;
        let more = Rect::from_min_size(pos2(x, y - 12.0), vec2(24.0, 24.0));
        if button(ui, more, "More", "", Some("ellipsis"), 12.0).clicked() {
            menu(&ctx, "more", more, ppp);
        }
        let grip = Rect::from_min_size(pos2(rect.right() - 8.0 - 20.0, y - 12.0), vec2(20.0, 24.0));
        let drag = ui.interact(grip, egui::Id::unique("focus-grip"), Sense::click_and_drag());
        drag.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, "Drag Focus Panel"));
        icons::paint_glyph(ui, "grip-vertical", Rect::from_center_size(grip.center(), vec2(16.0, 16.0)), theme::FG_60());
        if drag.double_clicked() {
            reset_panel(&ctx);
        } else if drag.drag_started() {
            with(|w| w.panel_drag = crate::capture::cursor());
        } else if drag.dragged() {
            let now = crate::capture::cursor();
            let from = with(|w| std::mem::replace(&mut w.panel_drag, now));
            if let Some(hwnd) = own_window(PANEL_TITLE) {
                let mut window = RECT::default();
                unsafe {
                    let _ = GetWindowRect(hwnd, &mut window);
                    let _ = SetWindowPos(hwnd, None, window.left + now.0 - from.0, window.top + now.1 - from.1, 0, 0, SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE);
                }
            }
        }
        if drag.hovered() {
            ctx.set_cursor_icon(egui::CursorIcon::Grab);
        }
    });
}

fn show_toast(ctx: &egui::Context) {
    let Some((notice, at)) = with(|w| w.toast.clone()) else { return };
    if notice.snooze.is_none() {
        let left = Duration::from_secs(TOAST_SECONDS).saturating_sub(at.elapsed());
        if left.is_zero() {
            with(|w| w.toast = None);
            return;
        }
        ctx.request_repaint_after(left);
    }
    let (area, _) = cursor_area();
    let position = pos2(area.center().x - TOAST_SIZE.x / 2.0, area.top() + TOAST_TOP_MARGIN);
    ctx.show_viewport_immediate(egui::ViewportId::from_hash_of("focus-toast"), popup(TOAST_TITLE, TOAST_SIZE, position, false), |ui, _| {
        if !with(|w| w.toast_styled) {
            if let Some(hwnd) = own_window(TOAST_TITLE) {
                popup_styles(hwnd, false);
                with(|w| w.toast_styled = true);
            }
        }
        let ctx = ui.ctx().clone();
        let rect = ui.max_rect();
        ui.painter().rect_filled(rect, 0.0, theme::PANEL());
        let tile = Rect::from_min_size(pos2(rect.left() + 16.0, rect.center().y - 11.0), vec2(22.0, 22.0));
        icons::command_icon(&mut ui.new_child(UiBuilder::new().max_rect(tile)), &ICON, "Focus", 22.0);
        let mut right = rect.right() - 36.0;
        if notice.snooze == Some(true) {
            let width = button_width(ui, "Snooze", false, 12.0);
            let snooze = Rect::from_min_size(pos2(right - width, rect.center().y - 14.0), vec2(width, 28.0));
            if button(ui, snooze, "snooze", "Snooze", None, 12.0).clicked() {
                if let Err(e) = focus::snooze(&host(&ctx)) {
                    crate::extension::notify(&ctx, crate::extension::Toast::Failure(e, String::new()));
                }
            }
            right = snooze.left() - 12.0;
        }
        let left = tile.right() + 12.0;
        let width = right - left;
        if notice.message.is_empty() {
            paint_text(ui, pos2(left, rect.center().y), Align2::LEFT_CENTER, &notice.title, theme::weighted(13.0, theme::SEMIBOLD), theme::FG(), width);
        } else {
            paint_text(ui, pos2(left, rect.center().y - 1.0), Align2::LEFT_BOTTOM, &notice.title, theme::weighted(13.0, theme::SEMIBOLD), theme::FG(), width);
            paint_text(ui, pos2(left, rect.center().y + 1.0), Align2::LEFT_TOP, &notice.message, theme::font(12.0), theme::FG_60(), width);
        }
        let close = Rect::from_min_size(pos2(rect.right() - 8.0 - 20.0, rect.top() + 8.0), vec2(20.0, 20.0));
        let response = ui.interact(close, egui::Id::unique("focus-toast-close"), Sense::click());
        response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, "Close"));
        if response.hovered() {
            ui.painter().rect_filled(close, 4.0, theme::CONTROL());
        }
        icons::paint_glyph(ui, "x", Rect::from_center_size(close.center(), vec2(14.0, 14.0)), if response.hovered() { theme::FG() } else { theme::FG_60() });
        if response.clicked() {
            with(|w| w.toast = None);
        }
    });
}

fn show_block(ctx: &egui::Context) {
    let Some((block, area)) = with(|w| w.block.clone()) else { return };
    let scale = cursor_area().1;
    let size = vec2((area.right - area.left) as f32 / scale, (area.bottom - area.top) as f32 / scale);
    let position = pos2(area.left as f32 / scale, area.top as f32 / scale);
    let id = egui::ViewportId::from_hash_of(("focus-block", &block.key));
    ctx.show_viewport_immediate(id, popup(BLOCK_TITLE, size, position, true), |ui, _| {
        let ctx = ui.ctx().clone();
        let (placed, raise) = with(|w| (w.block_placed, std::mem::take(&mut w.raise)));
        if !placed || raise {
            if let Some(hwnd) = own_window(BLOCK_TITLE) {
                if !placed {
                    popup_styles(hwnd, true);
                    unsafe {
                        let _ = SetWindowPos(hwnd, Some(HWND_TOPMOST), area.left, area.top, area.right - area.left, area.bottom - area.top, SWP_SHOWWINDOW);
                    }
                }
                launcher::force_foreground(hwnd);
                with(|w| w.block_placed = true);
            } else {
                ctx.request_repaint();
            }
        }
        let focused = ui.input(|i| i.viewport().focused) == Some(true);
        let was = with(|w| std::mem::replace(&mut w.block_focused, focused));
        if was && !focused {
            focus::block_left(&host(&ctx));
        }
        let rect = ui.max_rect();
        ui.painter().rect_filled(rect, 0.0, OVERLAY);
        let minutes = block.snooze_ms.map(|ms| (ms as f64 / 60_000.0).round() as u64).unwrap_or(0);
        let mut buttons: Vec<(&str, String)> = Vec::new();
        if minutes > 0 {
            buttons.push(("snooze", if minutes == 1 { "Snooze for 1 minute".into() } else { format!("Snooze for {minutes} minutes") }));
        }
        if block.website {
            buttons.push(("close-tab", "Close Tab".into()));
        }
        let center = rect.center();
        let content = 16.0 + 12.0 + 20.0 + if buttons.is_empty() { 0.0 } else { 16.0 + 30.0 } + 20.0 + 14.0;
        let mut y = center.y - content / 2.0;
        icons::paint_glyph(ui, "target", Rect::from_min_size(pos2(center.x - 8.0, y), vec2(16.0, 16.0)), theme::FG_60());
        y += 16.0 + 12.0;
        paint_text(ui, pos2(center.x, y), Align2::CENTER_TOP, &format!("{} has been blocked by Openray", block.name), theme::weighted(15.0, theme::SEMIBOLD), theme::FG(), rect.width() * 0.8);
        y += 20.0;
        if !buttons.is_empty() {
            y += 16.0;
            let widths: Vec<f32> = buttons.iter().map(|(id, text)| button_width(ui, text, false, 12.0).max(if *id == "snooze" { 180.0 } else { 0.0 }) + 8.0).collect();
            let total = widths.iter().sum::<f32>() + 8.0 * (buttons.len() - 1) as f32;
            let mut x = center.x - total / 2.0;
            for ((id, text), width) in buttons.iter().zip(widths) {
                let r = Rect::from_min_size(pos2(x, y), vec2(width, 30.0));
                let response = button(ui, r, id, text, None, 12.0);
                if !placed && *id == buttons[0].0 {
                    response.request_focus();
                }
                let pressed = response.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                if response.clicked() || pressed {
                    let host = host(&ctx);
                    let result = if *id == "snooze" { focus::snooze(&host) } else { focus::close_tab(&host) };
                    if let Err(e) = result {
                        crate::extension::notify(&ctx, crate::extension::Toast::Failure(e, String::new()));
                    }
                }
                x += width + 8.0;
            }
            y += 30.0;
        }
        y += 20.0;
        let note = if block.website { "Switching focus to another window will hide this page." } else { "Switching focus to another window will close this app." };
        paint_text(ui, pos2(center.x, y), Align2::CENTER_TOP, note, theme::font(11.0), theme::FG_60(), rect.width() * 0.8);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(status: Status, duration: Option<u64>, end: Option<u64>, remaining: u64, pause_end: Option<u64>) -> Session {
        crate::focus::test_session(status, duration, end, remaining, pause_end)
    }

    #[test]
    fn formats_the_timer_like_the_reference_panel() {
        assert_eq!(format_timer(25 * 60_000), "25:00");
        assert_eq!(format_timer(59_001), "01:00");
        assert_eq!(format_timer(90 * 60_000), "01:30");
        assert_eq!(format_timer(-5), "00:00");
        assert_eq!(timer_step(300_000), 1000);
        assert_eq!(timer_step(1_800_000), 5000);
        assert_eq!(timer_step(7_200_000), 60_000);
    }

    #[test]
    fn tracks_remaining_paused_and_elapsed_time() {
        let running = session(Status::Running, Some(600_000), Some(600_000), 600_000, None);
        assert_eq!(remaining_ms(&running, 150_000), 450_000);
        assert_eq!(elapsed_fraction(&running, 150_000), 0.25);
        let paused = session(Status::Paused, Some(600_000), None, 200_000, Some(400_000));
        assert_eq!(remaining_ms(&paused, 999_999), 200_000);
        assert_eq!(pause_remaining_ms(&paused, 100_000), Some(300_000));
        let open = session(Status::Running, None, None, 0, None);
        assert_eq!(remaining_ms(&open, 1), 0);
        assert_eq!(elapsed_fraction(&open, 1), 0.0);
        assert_eq!(shown_timer(&open, 1), None);
        assert_eq!(shown_timer(&running, 150_000).as_deref(), Some("07:30"));
    }
}
