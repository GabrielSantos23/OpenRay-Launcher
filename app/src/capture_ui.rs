use crate::capture::{self, Frame, Rect, FROZEN, LAST_REGION};
use crate::extension::{notify, Toast};
use crate::focus_ui::{cursor_area, own_window, paint_text, popup};
use crate::{launcher, theme};
use egui::{pos2, vec2, Align2, Color32, Pos2, Sense, Stroke, Ui};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Duration;
use windows::Win32::Graphics::Dwm::{DWMWA_CLOAK, DWMWA_TRANSITIONS_FORCEDISABLED};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass};
use windows::Win32::UI::WindowsAndMessaging::{SetForegroundWindow, SetWindowPos, HTCLIENT, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SWP_SHOWWINDOW, WM_NCHITTEST};

pub static VERSION: AtomicU64 = AtomicU64::new(0);
const HIDE_DELAY: Duration = Duration::from_millis(180);
const ZOOM_PIXELS: i32 = 15;
const LOUPE: f32 = 120.0;
const OVERLAY_TITLE: &str = "Openray Capture";
const DIM: Color32 = Color32::from_black_alpha(115);
const PIN_HELP: &str = "Pinned screenshot. Drag to move, scroll to zoom, Ctrl+C to copy, Esc or double-click to close.";

static BUSY: AtomicBool = AtomicBool::new(false);
static NEXT_PIN: AtomicU32 = AtomicU32::new(1);
static OVERLAY: Mutex<Option<Overlay>> = Mutex::new(None);
static PINS: Mutex<Vec<Pin>> = Mutex::new(Vec::new());

struct Overlay {
    mode: &'static str,
    app: String,
    previous: isize,
    screen: Rect,
    windows: Vec<Rect>,
    monitors: Vec<Rect>,
    cursor: (i32, i32),
    anchor: Option<(i32, i32)>,
    ruler: Option<Rect>,
    texture: egui::TextureHandle,
    stage: u8,
}

struct Pin {
    id: u32,
    rect: Rect,
    png: Vec<u8>,
    texture: egui::TextureHandle,
    zoom: f32,
    stage: u8,
    drag_from: (i32, i32),
}

enum Outcome {
    Keep,
    Cancel,
    Finish(Rect),
}

pub fn rect_from(a: (i32, i32), b: (i32, i32)) -> Rect {
    Rect { x: a.0.min(b.0), y: a.1.min(b.1), w: (a.0 - b.0).abs() + 1, h: (a.1 - b.1).abs() + 1 }
}

pub fn target_at(point: (i32, i32), windows: &[Rect], monitors: &[Rect]) -> Option<Rect> {
    windows.iter().chain(monitors).find(|r| r.contains(point.0, point.1)).copied()
}

pub fn measure(rect: Rect) -> String {
    format!("{} × {}  ·  {} px", rect.w, rect.h, f64::from(rect.w).hypot(f64::from(rect.h)).round() as i64)
}

pub fn fit(size: (i32, i32), bounds: (i32, i32)) -> (i32, i32) {
    let scale = (f64::from(bounds.0) / f64::from(size.0)).min(f64::from(bounds.1) / f64::from(size.1)).min(1.0);
    ((f64::from(size.0) * scale).round() as i32, (f64::from(size.1) * scale).round() as i32)
}

fn hint(mode: &str) -> &'static str {
    match mode {
        "color" => "Click to copy the color under the cursor",
        "ocr" => "Select the text to copy",
        "qr" => "Select the QR code to read",
        "pin" => "Select the area to pin to the screen",
        "ruler" => "Drag to measure",
        _ => "Drag to select an area, or click a window",
    }
}

fn success(ctx: &egui::Context, title: impl Into<String>) {
    notify(ctx, Toast::Success(title.into()));
}

fn failure(ctx: &egui::Context, error: impl Into<String>) {
    notify(ctx, Toast::Failure(error.into(), String::new()));
}

fn app_of(hwnd: HWND) -> String {
    crate::clipboard::window_app(hwnd).map(|(_, path)| std::path::Path::new(&path).file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()).unwrap_or_default()
}

fn year_month() -> (i32, u32) {
    let time = unsafe { windows::Win32::System::SystemInformation::GetLocalTime() };
    (i32::from(time.wYear), u32::from(time.wMonth))
}

fn deliver(ctx: &egui::Context, frame: &Frame, app: &str) -> Result<(), String> {
    let (_, png) = capture::save(frame, app, year_month())?;
    crate::clipboard::write_image(&png)?;
    VERSION.fetch_add(1, Ordering::SeqCst);
    success(ctx, "Screenshot copied to clipboard");
    Ok(())
}

pub fn start(ctx: &egui::Context, mode: &'static str) {
    if BUSY.swap(true, Ordering::SeqCst) {
        return;
    }
    let target = launcher::active_window();
    let (app, previous, shown, ctx) = (app_of(target), target.0 as isize, launcher::is_shown(), ctx.clone());
    std::thread::spawn(move || {
        if shown {
            std::thread::sleep(HIDE_DELAY);
        }
        let screen = capture::virtual_screen();
        let frame = match capture::grab(screen) {
            Ok(frame) => frame,
            Err(e) => {
                BUSY.store(false, Ordering::SeqCst);
                return failure(&ctx, e);
            }
        };
        let relative = |r: Rect| Rect { x: r.x - screen.x, y: r.y - screen.y, ..r };
        let (x, y) = capture::cursor();
        let image = egui::ColorImage::from_rgba_unmultiplied([screen.w as usize, screen.h as usize], &frame.rgba());
        let texture = ctx.load_texture("capture-frame", image, egui::TextureOptions::NEAREST);
        *FROZEN.lock().unwrap_or_else(|e| e.into_inner()) = Some(frame);
        *OVERLAY.lock().unwrap_or_else(|e| e.into_inner()) = Some(Overlay {
            mode,
            app,
            previous,
            screen,
            windows: capture::windows().into_iter().filter_map(|r| r.intersect(&screen)).map(relative).collect(),
            monitors: capture::monitors().into_iter().map(relative).collect(),
            cursor: (x - screen.x, y - screen.y),
            anchor: None,
            ruler: None,
            texture,
            stage: 0,
        });
        ctx.request_repaint();
    });
}

pub fn direct(ctx: &egui::Context, kind: &'static str) {
    let target = launcher::active_window();
    let (app, hwnd, shown, ctx) = (app_of(target), target.0 as isize, launcher::is_shown(), ctx.clone());
    std::thread::spawn(move || {
        if shown {
            std::thread::sleep(HIDE_DELAY);
        }
        if let Err(e) = capture_direct(&ctx, kind, HWND(hwnd as _), &app) {
            failure(&ctx, e);
        }
    });
}

fn capture_direct(ctx: &egui::Context, kind: &str, target: HWND, app: &str) -> Result<(), String> {
    let screen = capture::virtual_screen();
    let rect = match kind {
        "fullscreen" => screen,
        "window" => capture::window_frame(target).ok_or("There is no active window to capture")?,
        "monitor" => {
            let (x, y) = capture::cursor();
            capture::monitors().into_iter().find(|m| m.contains(x, y)).unwrap_or(screen)
        }
        _ => LAST_REGION.lock().unwrap_or_else(|e| e.into_inner()).ok_or("No region captured yet. Use Capture Region first.")?,
    };
    let rect = rect.intersect(&screen).ok_or("The capture area is off screen")?;
    if kind == "last-region" {
        *LAST_REGION.lock().unwrap_or_else(|e| e.into_inner()) = Some(rect);
    }
    deliver(ctx, &capture::grab(rect)?, app)
}

fn restore_focus(previous: isize) {
    if previous != 0 {
        unsafe {
            let _ = SetForegroundWindow(HWND(previous as _));
        }
    }
}

fn cancel(overlay: &Overlay) {
    restore_focus(overlay.previous);
    *FROZEN.lock().unwrap_or_else(|e| e.into_inner()) = None;
    BUSY.store(false, Ordering::SeqCst);
}

fn finish(ctx: &egui::Context, overlay: Overlay, rect: Rect) {
    let ctx = ctx.clone();
    restore_focus(overlay.previous);
    std::thread::spawn(move || {
        if let Err(e) = complete(&ctx, &overlay, rect) {
            failure(&ctx, e);
        }
        BUSY.store(false, Ordering::SeqCst);
    });
}

fn complete(ctx: &egui::Context, overlay: &Overlay, rect: Rect) -> Result<(), String> {
    let frozen = FROZEN.lock().unwrap_or_else(|e| e.into_inner()).take().ok_or("No capture in progress")?;
    let absolute = Rect { x: rect.x + frozen.rect.x, y: rect.y + frozen.rect.y, ..rect };
    let frame = frozen.crop(absolute).ok_or("The selection is empty")?;
    match overlay.mode {
        "color" => {
            let hex = format!("#{:02X}{:02X}{:02X}", frame.bgra[2], frame.bgra[1], frame.bgra[0]);
            crate::clipboard::write_text(&hex)?;
            success(ctx, format!("Copied {hex}"));
        }
        "ocr" => {
            let text = capture::recognize_text(&frame)?;
            if text.trim().is_empty() {
                return Err("No text found".into());
            }
            crate::clipboard::write_text(&text)?;
            success(ctx, format!("Copied {} characters", text.chars().count()));
        }
        "qr" => {
            let codes = capture::decode_qr(&frame);
            if codes.is_empty() {
                return Err("No QR code found".into());
            }
            crate::clipboard::write_text(&codes.join("\n"))?;
            success(ctx, "Copied QR code content");
        }
        "pin" => {
            let (png, rgba) = (frame.png()?, frame.rgba());
            pin(ctx, png, [frame.rect.w as usize, frame.rect.h as usize], &rgba, frame.rect);
        }
        _ => {
            *LAST_REGION.lock().unwrap_or_else(|e| e.into_inner()) = Some(absolute);
            deliver(ctx, &frame, &overlay.app)?;
        }
    }
    Ok(())
}

fn pin(ctx: &egui::Context, png: Vec<u8>, size: [usize; 2], rgba: &[u8], rect: Rect) {
    let id = NEXT_PIN.fetch_add(1, Ordering::Relaxed);
    let texture = ctx.load_texture(format!("pin-{id}"), egui::ColorImage::from_rgba_unmultiplied(size, rgba), egui::TextureOptions::LINEAR);
    PINS.lock().unwrap_or_else(|e| e.into_inner()).push(Pin { id, rect, png, texture, zoom: 1.0, stage: 0, drag_from: (0, 0) });
    ctx.request_repaint();
}

pub fn decode_png(bytes: &[u8]) -> Result<(u32, u32, Vec<u8>), String> {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().map_err(|_| "Only PNG screenshots can be read".to_string())?;
    let mut buffer = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buffer).map_err(|e| e.to_string())?;
    let channels = info.color_type.samples();
    let rgba = buffer[..info.buffer_size()]
        .chunks_exact(channels)
        .flat_map(|p| match channels {
            1 | 2 => [p[0], p[0], p[0], if channels == 2 { p[1] } else { 255 }],
            3 => [p[0], p[1], p[2], 255],
            _ => [p[0], p[1], p[2], p[3]],
        })
        .collect();
    Ok((info.width, info.height, rgba))
}

pub fn pin_file(ctx: &egui::Context, path: &str) -> Result<(), String> {
    if !path.to_lowercase().ends_with(".png") {
        return Err("Only PNG screenshots can be pinned".into());
    }
    let png = std::fs::read(path).map_err(|e| e.to_string())?;
    let (w, h, rgba) = decode_png(&png)?;
    let (cx, cy) = capture::cursor();
    let monitor = capture::monitors().into_iter().find(|m| m.contains(cx, cy)).unwrap_or_else(capture::virtual_screen);
    let (fw, fh) = fit((w as i32, h as i32), (monitor.w * 2 / 3, monitor.h * 2 / 3));
    pin(ctx, png, [w as usize, h as usize], &rgba, Rect { x: monitor.x + (monitor.w - fw) / 2, y: monitor.y + (monitor.h - fh) / 2, w: fw, h: fh });
    Ok(())
}

pub fn file_text(path: &str) -> Result<String, String> {
    let (w, h, rgba) = decode_png(&std::fs::read(path).map_err(|e| e.to_string())?)?;
    let bgra = rgba.chunks_exact(4).flat_map(|p| [p[2], p[1], p[0], 255]).collect();
    capture::recognize_text(&Frame { rect: Rect { x: 0, y: 0, w: w as i32, h: h as i32 }, bgra })
}

pub fn show(ctx: &egui::Context) {
    show_overlay(ctx);
    show_pins(ctx);
}

fn show_overlay(ctx: &egui::Context) {
    let Some(mut overlay) = OVERLAY.lock().unwrap_or_else(|e| e.into_inner()).take() else { return };
    let scale = cursor_area().1;
    let builder = popup(OVERLAY_TITLE, vec2(overlay.screen.w as f32, overlay.screen.h as f32) / scale, pos2(overlay.screen.x as f32, overlay.screen.y as f32) / scale, true).with_visible(false);
    let outcome = ctx.show_viewport_immediate(egui::ViewportId::from_hash_of("capture-overlay"), builder, |ui, _| overlay_ui(ui, &mut overlay));
    match outcome {
        Outcome::Keep => *OVERLAY.lock().unwrap_or_else(|e| e.into_inner()) = Some(overlay),
        Outcome::Cancel => cancel(&overlay),
        Outcome::Finish(rect) => finish(ctx, overlay, rect),
    }
}

fn reveal(ctx: &egui::Context, title: &str, stage: &mut u8, r: Rect) {
    if *stage >= 3 {
        return;
    }
    if let Some(hwnd) = own_window(title) {
        match *stage {
            0 => {
                launcher::set_dwm(hwnd, DWMWA_TRANSITIONS_FORCEDISABLED, 1i32);
                launcher::set_dwm(hwnd, DWMWA_CLOAK, 1i32);
                unsafe {
                    let _ = SetWindowSubclass(hwnd, Some(client_everywhere), 1, 0);
                    let _ = SetWindowPos(hwnd, Some(HWND_TOPMOST), r.x, r.y, r.w, r.h, SWP_NOACTIVATE | SWP_SHOWWINDOW);
                }
            }
            2 => {
                launcher::set_dwm(hwnd, DWMWA_CLOAK, 0i32);
                launcher::force_foreground(hwnd);
            }
            _ => {}
        }
        *stage += 1;
    }
    ctx.request_repaint();
}

unsafe extern "system" fn client_everywhere(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM, _: usize, _: usize) -> LRESULT {
    if msg == WM_NCHITTEST {
        return LRESULT(HTCLIENT as isize);
    }
    unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
}

fn overlay_ui(ui: &mut Ui, overlay: &mut Overlay) -> Outcome {
    reveal(ui.ctx(), OVERLAY_TITLE, &mut overlay.stage, overlay.screen);
    let ctx = ui.ctx().clone();
    let ppp = ctx.pixels_per_point();
    let full = ui.max_rect();
    let physical = |p: Pos2| ((p.x * ppp).round() as i32, (p.y * ppp).round() as i32);
    let (press, release, right, escape, enter, hover) = ui.input(|i| (i.pointer.primary_pressed(), i.pointer.primary_released(), i.pointer.secondary_pressed(), i.key_pressed(egui::Key::Escape), i.key_pressed(egui::Key::Enter), i.pointer.latest_pos()));
    if let Some(p) = hover {
        overlay.cursor = physical(p);
    }
    let response = ui.interact(full, egui::Id::unique("capture-surface"), Sense::hover());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Application, true, format!("Openray capture: {}", hint(overlay.mode))));
    ctx.set_cursor_icon(egui::CursorIcon::Crosshair);
    if escape || right {
        return Outcome::Cancel;
    }
    if enter {
        if let Some(r) = overlay.ruler.filter(|_| overlay.mode == "ruler") {
            let text = format!("{}x{}", r.w, r.h);
            match crate::clipboard::write_text(&text) {
                Ok(()) => success(&ctx, format!("Copied {text}")),
                Err(e) => failure(&ctx, e),
            }
            return Outcome::Cancel;
        }
    }
    let bounds = Rect { x: 0, y: 0, w: overlay.screen.w, h: overlay.screen.h };
    let done = |rect: Option<Rect>| rect.and_then(|r| r.intersect(&bounds)).map_or(Outcome::Cancel, Outcome::Finish);
    if press {
        if overlay.mode == "color" {
            return done(Some(Rect { x: overlay.cursor.0, y: overlay.cursor.1, w: 1, h: 1 }));
        }
        overlay.ruler = None;
        overlay.anchor = Some(overlay.cursor);
    }
    if release {
        if let Some(anchor) = overlay.anchor.take() {
            let rect = rect_from(anchor, overlay.cursor);
            if overlay.mode == "ruler" {
                overlay.ruler = Some(rect);
            } else if rect.w < 4 && rect.h < 4 {
                return done(target_at(overlay.cursor, &overlay.windows, &overlay.monitors));
            } else {
                return done(Some(rect));
            }
        }
    }
    paint_overlay(ui, overlay, ppp);
    Outcome::Keep
}

fn paint_overlay(ui: &Ui, overlay: &Overlay, ppp: f32) {
    let full = ui.max_rect();
    let painter = ui.painter();
    let unit = egui::Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
    painter.image(overlay.texture.id(), full, unit, Color32::WHITE);
    let css = |v: i32| v as f32 / ppp;
    let to_rect = |r: Rect| egui::Rect::from_min_size(pos2(css(r.x), css(r.y)), vec2(css(r.w), css(r.h)));
    let mode = overlay.mode;
    let selection = overlay.anchor.map(|a| rect_from(a, overlay.cursor)).or(overlay.ruler);
    let hovered = (overlay.anchor.is_none() && overlay.ruler.is_none() && mode != "color" && mode != "ruler").then(|| target_at(overlay.cursor, &overlay.windows, &overlay.monitors)).flatten();
    let shown = selection.or(hovered);
    match shown.map(to_rect) {
        Some(area) => {
            for dim in [
                egui::Rect::from_min_max(full.min, pos2(full.right(), area.top())),
                egui::Rect::from_min_max(pos2(full.left(), area.bottom()), full.max),
                egui::Rect::from_min_max(pos2(full.left(), area.top()), pos2(area.left(), area.bottom())),
                egui::Rect::from_min_max(pos2(area.right(), area.top()), pos2(full.right(), area.bottom())),
            ] {
                painter.rect_filled(dim, 0.0, DIM);
            }
            painter.rect_stroke(area, 0.0, Stroke::new(1.0, theme::ACCENT()), egui::StrokeKind::Inside);
        }
        None => {
            painter.rect_filled(full, 0.0, DIM);
        }
    }
    let (cx, cy) = (css(overlay.cursor.0), css(overlay.cursor.1));
    if mode != "color" && overlay.anchor.is_none() {
        let line = Stroke::new(1.0, theme::ACCENT().gamma_multiply(0.5));
        painter.vline(cx, full.y_range(), line);
        painter.hline(full.x_range(), cy, line);
    }
    if let Some(r) = shown {
        let label = if mode == "ruler" { measure(r) } else { format!("{} × {}", r.w, r.h) };
        let area = to_rect(r);
        let font = theme::weighted(12.0, theme::MEDIUM);
        let width = ui.fonts_mut(|f| f.layout_no_wrap(crate::i18n::tr(&label).into_owned(), font.clone(), theme::FG())).size().x + 12.0;
        let top = if area.top() > 28.0 { area.top() - 26.0 } else { area.bottom() + 6.0 };
        let pill = egui::Rect::from_min_size(pos2(area.left(), top), vec2(width, 20.0));
        painter.rect_filled(pill, 5.0, theme::PANEL());
        paint_text(ui, pill.center(), Align2::CENTER_CENTER, &label, font, theme::FG(), width);
    }
    paint_loupe(ui, overlay, full, cx, cy);
    let monitor = target_at(overlay.cursor, &[], &overlay.monitors).map(to_rect).unwrap_or(full);
    let extra = if mode == "ruler" { "Enter to copy · Esc to close" } else { "Esc to cancel" };
    let text = hint(mode);
    let (first, second) = ui.fonts_mut(|f| (f.layout_no_wrap(crate::i18n::tr(&text).into_owned(), theme::font(13.0), theme::FG()).size().x, f.layout_no_wrap(crate::i18n::tr(&extra).into_owned(), theme::font(13.0), theme::FG()).size().x));
    let box_size = vec2(first + 8.0 + second + 24.0, 30.0);
    let bar = egui::Rect::from_min_size(pos2(monitor.center().x - box_size.x / 2.0, monitor.top() + 16.0), box_size);
    painter.rect(bar, 8.0, theme::PANEL(), Stroke::new(1.0, theme::PANEL_BORDER()), egui::StrokeKind::Inside);
    paint_text(ui, pos2(bar.left() + 12.0, bar.center().y), Align2::LEFT_CENTER, text, theme::font(13.0), theme::FG(), first + 1.0);
    paint_text(ui, pos2(bar.left() + 12.0 + first + 8.0, bar.center().y), Align2::LEFT_CENTER, extra, theme::font(13.0), theme::FG_60(), second + 1.0);
}

fn paint_loupe(ui: &Ui, overlay: &Overlay, full: egui::Rect, cx: f32, cy: f32) {
    let painter = ui.painter();
    let left = if cx + 20.0 + LOUPE > full.right() { cx - 20.0 - LOUPE } else { cx + 20.0 };
    let top = if cy + 20.0 + LOUPE + 44.0 > full.bottom() { cy - 20.0 - LOUPE - 44.0 } else { cy + 20.0 };
    let outer = egui::Rect::from_min_size(pos2(left, top), vec2(LOUPE, LOUPE + 40.0));
    painter.rect(outer, 8.0, theme::PANEL(), Stroke::new(1.0, theme::PANEL_BORDER()), egui::StrokeKind::Inside);
    let zoom = egui::Rect::from_min_size(outer.min, vec2(LOUPE, LOUPE)).shrink(1.0);
    let (w, h) = (overlay.screen.w as f32, overlay.screen.h as f32);
    let start = pos2((overlay.cursor.0 - ZOOM_PIXELS / 2) as f32 / w, (overlay.cursor.1 - ZOOM_PIXELS / 2) as f32 / h);
    let uv = egui::Rect::from_min_size(start, vec2(ZOOM_PIXELS as f32 / w, ZOOM_PIXELS as f32 / h));
    painter.image(overlay.texture.id(), zoom, uv, Color32::WHITE);
    let cell = zoom.width() / ZOOM_PIXELS as f32;
    let center = egui::Rect::from_min_size(zoom.min + vec2(cell, cell) * (ZOOM_PIXELS / 2) as f32, vec2(cell, cell));
    painter.rect_stroke(center, 0.0, Stroke::new(1.0, theme::FG()), egui::StrokeKind::Inside);
    let y = outer.bottom() - 12.0;
    paint_text(ui, pos2(outer.left() + 8.0, y - 16.0), Align2::LEFT_CENTER, &format!("{}, {}", overlay.cursor.0, overlay.cursor.1), theme::font(11.0), theme::FG_60(), LOUPE - 16.0);
    if let Some((hex, color)) = pixel(overlay.cursor) {
        paint_text(ui, pos2(outer.left() + 22.0, y), Align2::LEFT_CENTER, &hex, theme::weighted(11.0, theme::MEDIUM), theme::FG(), LOUPE - 30.0);
        let swatch = egui::Rect::from_center_size(pos2(outer.left() + 13.0, y), vec2(10.0, 10.0));
        painter.rect(swatch, 2.0, color, Stroke::new(1.0, theme::PANEL_BORDER()), egui::StrokeKind::Inside);
    }
}

fn pixel((x, y): (i32, i32)) -> Option<(String, Color32)> {
    let frozen = FROZEN.lock().unwrap_or_else(|e| e.into_inner());
    let frame = frozen.as_ref()?;
    if x < 0 || y < 0 || x >= frame.rect.w || y >= frame.rect.h {
        return None;
    }
    let p = &frame.bgra[((y * frame.rect.w + x) * 4) as usize..];
    Some((format!("#{:02X}{:02X}{:02X}", p[2], p[1], p[0]), Color32::from_rgb(p[2], p[1], p[0])))
}

fn show_pins(ctx: &egui::Context) {
    let mut pins = std::mem::take(&mut *PINS.lock().unwrap_or_else(|e| e.into_inner()));
    if pins.is_empty() {
        return;
    }
    pins.retain_mut(|pin| {
        let title = format!("Pinned Screenshot {}", pin.id);
        let scale = cursor_area().1;
        let builder = popup(&title, vec2(pin.rect.w.max(24) as f32, pin.rect.h.max(24) as f32) / scale, pos2(pin.rect.x as f32, pin.rect.y as f32) / scale, true).with_visible(false);
        ctx.show_viewport_immediate(egui::ViewportId::from_hash_of(("capture-pin", pin.id)), builder, |ui, _| pin_ui(ui, pin, &title))
    });
    let mut shared = PINS.lock().unwrap_or_else(|e| e.into_inner());
    pins.append(&mut shared);
    *shared = pins;
}

fn pin_ui(ui: &mut Ui, pin: &mut Pin, title: &str) -> bool {
    let ctx = ui.ctx().clone();
    let hwnd = own_window(title);
    reveal(&ctx, title, &mut pin.stage, Rect { w: pin.rect.w.max(24), h: pin.rect.h.max(24), ..pin.rect });
    let full = ui.max_rect();
    ui.painter().image(pin.texture.id(), full, egui::Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
    let response = ui.interact(full, egui::Id::unique(("pin-surface", pin.id)), Sense::click_and_drag());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Image, true, PIN_HELP));
    ctx.set_cursor_icon(egui::CursorIcon::Move);
    let (escape, copy, wheel, closing) = ui.input(|i| (i.key_pressed(egui::Key::Escape), i.events.iter().any(|e| matches!(e, egui::Event::Copy)), i.events.iter().find_map(|e| if let egui::Event::MouseWheel { delta, .. } = e { Some(delta.y) } else { None }).unwrap_or(0.0), i.viewport().close_requested()));
    if escape || response.double_clicked() || closing {
        return false;
    }
    if response.drag_started() {
        pin.drag_from = capture::cursor();
    } else if response.dragged() {
        let now = capture::cursor();
        pin.rect.x += now.0 - pin.drag_from.0;
        pin.rect.y += now.1 - pin.drag_from.1;
        pin.drag_from = now;
        if let Some(hwnd) = hwnd {
            unsafe {
                let _ = SetWindowPos(hwnd, None, pin.rect.x, pin.rect.y, 0, 0, SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE);
            }
        }
    }
    if copy {
        match crate::clipboard::write_image(&pin.png) {
            Ok(()) => success(&ctx, "Copied Image to Clipboard"),
            Err(e) => failure(&ctx, e),
        }
    }
    if wheel != 0.0 {
        pin.zoom = (pin.zoom * if wheel > 0.0 { 1.1 } else { 1.0 / 1.1 }).clamp(0.2, 4.0);
        if let Some(hwnd) = hwnd {
            let (w, h) = ((pin.rect.w as f32 * pin.zoom).round() as i32, (pin.rect.h as f32 * pin.zoom).round() as i32);
            unsafe {
                let _ = SetWindowPos(hwnd, None, 0, 0, w.max(24), h.max(24), SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE);
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_a_rectangle_from_two_corners() {
        assert_eq!(rect_from((10, 20), (5, 2)), Rect { x: 5, y: 2, w: 6, h: 19 });
        assert_eq!(rect_from((3, 3), (3, 3)), Rect { x: 3, y: 3, w: 1, h: 1 });
    }

    #[test]
    fn picks_the_topmost_window_then_the_monitor() {
        let windows = [Rect { x: 100, y: 100, w: 200, h: 200 }, Rect { x: 0, y: 0, w: 1000, h: 800 }];
        let monitors = [Rect { x: 0, y: 0, w: 1920, h: 1080 }];
        assert_eq!(target_at((150, 150), &windows, &monitors), Some(windows[0]));
        assert_eq!(target_at((50, 50), &windows, &monitors), Some(windows[1]));
        assert_eq!(target_at((1500, 900), &windows, &monitors), Some(monitors[0]));
        assert_eq!(target_at((5000, 900), &windows, &monitors), None);
    }

    #[test]
    fn formats_measurements_and_fits_images() {
        assert_eq!(measure(Rect { x: 0, y: 0, w: 300, h: 400 }), "300 × 400  ·  500 px");
        assert_eq!(fit((4000, 2000), (1000, 1000)), (1000, 500));
        assert_eq!(fit((300, 200), (1000, 1000)), (300, 200));
    }

    #[test]
    fn decodes_what_a_frame_encodes() {
        let frame = Frame { rect: Rect { x: 0, y: 0, w: 2, h: 1 }, bgra: vec![1, 2, 3, 255, 4, 5, 6, 255] };
        assert_eq!(decode_png(&frame.png().unwrap()).unwrap(), (2, 1, vec![3, 2, 1, 255, 6, 5, 4, 255]));
    }
}
