use crate::capture::{self, Frame, Rect, FROZEN, LAST_REGION};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder};
use windows::Win32::Foundation::{COLORREF, HWND};
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowLongPtrW, SetLayeredWindowAttributes, SetWindowLongPtrW, SetWindowPos, ShowWindow, GWL_EXSTYLE, HWND_TOPMOST, LWA_ALPHA, SW_HIDE, SW_SHOWNOACTIVATE, SWP_NOACTIVATE,
    WS_EX_APPWINDOW, WS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT,
};

pub const OVERLAY: &str = "capture";
const HIDE_DELAY: Duration = Duration::from_millis(180);

struct Session {
    mode: String,
    app: String,
    previous: isize,
}

static SESSION: Mutex<Option<Session>> = Mutex::new(None);
static PINS: Mutex<Option<HashMap<String, Vec<u8>>>> = Mutex::new(None);
static NEXT_PIN: AtomicU32 = AtomicU32::new(1);
static PENDING: Mutex<Option<Start>> = Mutex::new(None);

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Start {
    mode: String,
    width: i32,
    height: i32,
    windows: Vec<Rect>,
    monitors: Vec<Rect>,
    cursor: (i32, i32),
    nonce: u32,
}

fn hud(app: &AppHandle, variant: &str, title: &str, message: Option<&str>) {
    let _ = app.emit_to(crate::hud::LABEL, "hud-show", serde_json::json!({ "variant": variant, "title": title, "message": message }));
}

fn foreground_app() -> String {
    let hwnd = if crate::launcher::is_shown() { crate::launcher::previous_window() } else { unsafe { GetForegroundWindow() } };
    crate::clipboard::window_app(hwnd).map(|(_, path)| std::path::Path::new(&path).file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()).unwrap_or_default()
}

fn set_visible(hwnd: HWND, visible: bool) {
    unsafe {
        let ex = (GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32 & !WS_EX_APPWINDOW.0) | WS_EX_LAYERED.0 | WS_EX_TOOLWINDOW.0;
        let ex = if visible { ex & !WS_EX_TRANSPARENT.0 } else { ex | WS_EX_TRANSPARENT.0 };
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex as isize);
        let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), if visible { 255 } else { 0 }, LWA_ALPHA);
    }
}

fn foreground_window() -> HWND {
    if crate::launcher::is_shown() {
        crate::launcher::previous_window()
    } else {
        unsafe { GetForegroundWindow() }
    }
}

fn now() -> (i32, u32) {
    let time = unsafe { windows::Win32::System::SystemInformation::GetLocalTime() };
    (time.wYear as i32, time.wMonth as u32)
}

fn deliver(app: &AppHandle, frame: &Frame, app_name: &str) -> Result<(), String> {
    let (path, png) = capture::save(frame, app_name, now())?;
    crate::clipboard::write_image(&png)?;
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    hud(app, "success", "Screenshot copied to clipboard", Some(&name));
    let _ = app.emit("screenshot-saved", path.to_string_lossy().into_owned());
    Ok(())
}

pub fn start(app: &AppHandle, mode: String) -> Result<(), String> {
    let app_name = foreground_app();
    let previous = foreground_window().0 as isize;
    crate::launcher::hide(app, false);
    std::thread::sleep(HIDE_DELAY);
    let screen = capture::virtual_screen();
    let frame = capture::grab(screen)?;
    let relative = |r: Rect| Rect { x: r.x - screen.x, y: r.y - screen.y, ..r };
    let (cx, cy) = capture::cursor();
    let start = Start {
        mode: mode.clone(),
        width: screen.w,
        height: screen.h,
        windows: capture::windows().into_iter().filter_map(|r| r.intersect(&screen)).map(relative).collect(),
        monitors: capture::monitors().into_iter().map(relative).collect(),
        cursor: (cx - screen.x, cy - screen.y),
        nonce: NEXT_PIN.fetch_add(1, Ordering::Relaxed),
    };
    *FROZEN.lock().unwrap_or_else(|e| e.into_inner()) = Some(frame);
    *SESSION.lock().unwrap_or_else(|e| e.into_inner()) = Some(Session { mode, app: app_name, previous });
    crate::open_window(app, OVERLAY, move |app, window| {
        if let Some(hwnd) = crate::launcher::hwnd(&window) {
            crate::launcher::make_popup(hwnd, screen.w, screen.h);
            set_visible(hwnd, false);
            unsafe {
                let _ = SetWindowPos(hwnd, Some(HWND_TOPMOST), screen.x, screen.y, screen.w, screen.h, SWP_NOACTIVATE);
                let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            }
        }
        *PENDING.lock().unwrap_or_else(|e| e.into_inner()) = Some(start.clone());
        let _ = app.emit_to(OVERLAY, "capture-start", start);
    });
    Ok(())
}

pub fn pending() -> Option<serde_json::Value> {
    PENDING.lock().unwrap_or_else(|e| e.into_inner()).as_ref().and_then(|s| serde_json::to_value(s).ok())
}

pub fn pixel(x: i32, y: i32) -> Option<String> {
    let frozen = FROZEN.lock().unwrap_or_else(|e| e.into_inner());
    let frame = frozen.as_ref()?;
    if x < 0 || y < 0 || x >= frame.rect.w || y >= frame.rect.h {
        return None;
    }
    let i = ((y * frame.rect.w + x) * 4) as usize;
    Some(format!("#{:02X}{:02X}{:02X}", frame.bgra[i + 2], frame.bgra[i + 1], frame.bgra[i]))
}

pub fn ready(app: &AppHandle) {
    *PENDING.lock().unwrap_or_else(|e| e.into_inner()) = None;
    let Some(window) = app.get_webview_window(OVERLAY) else { return };
    let Some(hwnd) = crate::launcher::hwnd(&window) else { return };
    let screen = capture::virtual_screen();
    unsafe {
        let _ = SetWindowPos(hwnd, Some(HWND_TOPMOST), screen.x, screen.y, screen.w, screen.h, Default::default());
    }
    set_visible(hwnd, true);
    crate::launcher::force_foreground(hwnd);
    let _ = window.set_focus();
}

fn hide_overlay(app: &AppHandle) {
    if let Some(hwnd) = app.get_webview_window(OVERLAY).as_ref().and_then(crate::launcher::hwnd) {
        set_visible(hwnd, false);
        unsafe {
            let _ = ShowWindow(hwnd, SW_HIDE);
        }
    }
}

fn restore_focus(previous: isize) {
    if previous != 0 {
        unsafe {
            let _ = windows::Win32::UI::WindowsAndMessaging::SetForegroundWindow(HWND(previous as _));
        }
    }
}

pub fn cancel(app: &AppHandle) {
    hide_overlay(app);
    if let Some(session) = SESSION.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
        restore_focus(session.previous);
    }
    *PENDING.lock().unwrap_or_else(|e| e.into_inner()) = None;
    *SESSION.lock().unwrap_or_else(|e| e.into_inner()) = None;
    *FROZEN.lock().unwrap_or_else(|e| e.into_inner()) = None;
}

pub fn finish(app: &AppHandle, rect: Rect) -> Result<(), String> {
    hide_overlay(app);
    let session = SESSION.lock().unwrap_or_else(|e| e.into_inner()).take().ok_or("No capture in progress")?;
    let frozen = FROZEN.lock().unwrap_or_else(|e| e.into_inner()).take().ok_or("No capture in progress")?;
    restore_focus(session.previous);
    let absolute = Rect { x: rect.x + frozen.rect.x, y: rect.y + frozen.rect.y, ..rect };
    let frame = frozen.crop(absolute).ok_or("The selection is empty")?;
    match session.mode.as_str() {
        "color" => {
            let p = &frame.bgra;
            let hex = format!("#{:02X}{:02X}{:02X}", p[2], p[1], p[0]);
            crate::clipboard::write_text(&hex)?;
            hud(app, "success", &format!("Copied {hex}"), None);
        }
        "ocr" => {
            let text = capture::recognize_text(&frame)?;
            if text.trim().is_empty() {
                hud(app, "failure", "No text found", None);
            } else {
                crate::clipboard::write_text(&text)?;
                hud(app, "success", "Copied text to clipboard", Some(&format!("{} characters", text.chars().count())));
            }
        }
        "qr" => {
            let codes = capture::decode_qr(&frame);
            if codes.is_empty() {
                hud(app, "failure", "No QR code found", None);
            } else {
                crate::clipboard::write_text(&codes.join("\n"))?;
                hud(app, "success", "Copied QR code content", Some(&codes[0]));
            }
        }
        "pin" => pin(app, frame.png()?, frame.rect)?,
        _ => {
            *LAST_REGION.lock().unwrap_or_else(|e| e.into_inner()) = Some(absolute);
            deliver(app, &frame, &session.app)?;
        }
    }
    Ok(())
}

pub fn direct(app: &AppHandle, kind: &str) -> Result<(), String> {
    let app_name = foreground_app();
    let target = foreground_window();
    crate::launcher::hide(app, false);
    std::thread::sleep(HIDE_DELAY);
    let screen = capture::virtual_screen();
    let rect = match kind {
        "fullscreen" => screen,
        "window" => capture::window_frame(target).ok_or("There is no active window to capture")?,
        "monitor" => {
            let (x, y) = capture::cursor();
            capture::monitors().into_iter().find(|m| m.contains(x, y)).unwrap_or(screen)
        }
        "last-region" => LAST_REGION.lock().unwrap_or_else(|e| e.into_inner()).ok_or("No region captured yet. Use Capture Region first.")?,
        _ => return Err(format!("Unknown capture {kind}")),
    };
    let rect = rect.intersect(&screen).ok_or("The capture area is off screen")?;
    if kind == "last-region" {
        *LAST_REGION.lock().unwrap_or_else(|e| e.into_inner()) = Some(rect);
    }
    let frame = capture::grab(rect)?;
    deliver(app, &frame, &app_name)
}

pub fn pin(app: &AppHandle, png: Vec<u8>, rect: Rect) -> Result<(), String> {
    let label = format!("pin-{}", NEXT_PIN.fetch_add(1, Ordering::Relaxed));
    PINS.lock().unwrap_or_else(|e| e.into_inner()).get_or_insert_with(HashMap::new).insert(label.clone(), png);
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let built = WebviewWindowBuilder::new(&app, &label, WebviewUrl::App("index.html".into()))
            .title("Pinned Screenshot")
            .decorations(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .resizable(false)
            .maximizable(false)
            .minimizable(false)
            .shadow(true)
            .visible(false)
            .background_color(tauri::window::Color(0, 0, 0, 255))
            .build();
        match built {
            Ok(window) => {
                let _ = window.set_size(PhysicalSize::new(rect.w.max(24) as u32, rect.h.max(24) as u32));
                let _ = window.set_position(PhysicalPosition::new(rect.x, rect.y));
                let _ = window.show();
                let _ = window.set_focus();
            }
            Err(e) => eprintln!("openray: could not open a pinned screenshot: {e}"),
        }
    });
    Ok(())
}

pub fn pin_file(app: &AppHandle, path: &str) -> Result<(), String> {
    if !path.to_lowercase().ends_with(".png") {
        return Err("Only PNG screenshots can be pinned".into());
    }
    let png = std::fs::read(path).map_err(|e| e.to_string())?;
    let decoder = png::Decoder::new(std::io::Cursor::new(&png));
    let info = decoder.read_info().map_err(|e| e.to_string())?;
    let (w, h) = (info.info().width as i32, info.info().height as i32);
    let (cx, cy) = capture::cursor();
    let monitor = capture::monitors().into_iter().find(|m| m.contains(cx, cy)).unwrap_or_else(capture::virtual_screen);
    let (w2, h2) = fit((w, h), (monitor.w * 2 / 3, monitor.h * 2 / 3));
    crate::launcher::hide(app, false);
    pin(app, png, Rect { x: monitor.x + (monitor.w - w2) / 2, y: monitor.y + (monitor.h - h2) / 2, w: w2, h: h2 })
}

pub fn fit(size: (i32, i32), bounds: (i32, i32)) -> (i32, i32) {
    let scale = (bounds.0 as f64 / size.0 as f64).min(bounds.1 as f64 / size.1 as f64).min(1.0);
    (((size.0 as f64) * scale).round() as i32, ((size.1 as f64) * scale).round() as i32)
}

pub fn pin_image(label: &str) -> Option<Vec<u8>> {
    PINS.lock().unwrap_or_else(|e| e.into_inner()).as_ref()?.get(label).cloned()
}

pub fn pin_shown(app: &AppHandle, label: &str) {
    if let Some(window) = app.get_webview_window(label) {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

pub fn pin_closed(label: &str) {
    if let Some(pins) = PINS.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
        pins.remove(label);
    }
}

pub fn serve(request: tauri::http::Request<Vec<u8>>) -> tauri::http::Response<Vec<u8>> {
    let path = percent_encoding::percent_decode_str(request.uri().path().trim_start_matches('/')).decode_utf8_lossy().into_owned();
    let body = if path == "frame" {
        FROZEN.lock().unwrap_or_else(|e| e.into_inner()).as_ref().map(|f| (f.bmp(), "image/bmp"))
    } else {
        path.strip_prefix("pin/").and_then(pin_image).map(|png| (png, "image/png"))
    };
    match body {
        Some((bytes, kind)) => tauri::http::Response::builder().header("Content-Type", kind).header("Access-Control-Allow-Origin", "*").header("Cache-Control", "no-store").body(bytes),
        None => tauri::http::Response::builder().status(404).body(Vec::new()),
    }
    .expect("valid capture response")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fits_images_inside_bounds() {
        assert_eq!(fit((4000, 2000), (1000, 1000)), (1000, 500));
        assert_eq!(fit((300, 200), (1000, 1000)), (300, 200));
    }

    #[test]
    fn computes_the_current_month() {
        let (year, month) = now();
        assert!(year >= 2024 && (1..=12).contains(&month));
    }
}
