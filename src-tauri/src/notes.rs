use crate::launcher::hwnd;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewWindow};
use windows::Win32::Foundation::COLORREF;
use windows::Win32::UI::WindowsAndMessaging::{SetLayeredWindowAttributes, SetWindowPos, HWND_TOPMOST, LWA_ALPHA, SWP_NOACTIVATE};

pub const LABEL: &str = "notes";
pub const TOOLTIP_LABEL: &str = "tooltip";
const EDGE_MARGIN: f64 = 16.0;
const TOOLTIP_GAP: f64 = 6.0;

static PLACED: AtomicBool = AtomicBool::new(false);
static TOOLTIP_ANCHOR: Mutex<Option<[i32; 4]>> = Mutex::new(None);

pub fn right_edge_position(area: (i32, i32, i32, i32), size: (i32, i32), margin: i32) -> (i32, i32) {
    let (x, y, width, height) = area;
    (x + width - size.0 - margin, y + ((height - size.1) / 2).max(0))
}

fn place(app: &AppHandle, window: &WebviewWindow) {
    let monitor = app
        .cursor_position()
        .ok()
        .and_then(|p| app.monitor_from_point(p.x, p.y).ok().flatten())
        .or_else(|| window.primary_monitor().ok().flatten());
    let (Some(monitor), Ok(size)) = (monitor, window.outer_size()) else { return };
    let area = monitor.work_area();
    let height = size.height.min(area.size.height);
    if height != size.height {
        let _ = window.set_size(PhysicalSize::new(size.width, height));
    }
    let (x, y) = right_edge_position(
        (area.position.x, area.position.y, area.size.width as i32, area.size.height as i32),
        (size.width as i32, height as i32),
        (EDGE_MARGIN * monitor.scale_factor()).round() as i32,
    );
    let _ = window.set_position(PhysicalPosition::new(x, y));
}

pub fn show(app: &AppHandle, request: serde_json::Value) {
    let Some(window) = app.get_webview_window(LABEL) else { return };
    if !PLACED.swap(true, Ordering::SeqCst) {
        place(app, &window);
    }
    let _ = window.unminimize();
    let _ = window.show();
    let _ = window.set_focus();
    let _ = app.emit_to(LABEL, "notes-open", request);
}

pub fn tooltip_position(anchor: [i32; 4], size: (i32, i32), area: (i32, i32, i32, i32), gap: i32) -> (i32, i32) {
    let [left, top, right, bottom] = anchor;
    let (ax, ay, aw, ah) = area;
    let x = ((left + right) / 2 - size.0 / 2).clamp(ax, (ax + aw - size.0).max(ax));
    let above = top - gap - size.1;
    let y = if above >= ay { above } else { (bottom + gap).min(ay + ah - size.1) };
    (x, y)
}

pub fn set_tooltip_anchor(window: &WebviewWindow, rect: Option<[f64; 4]>) {
    let anchor = rect.and_then(|[left, top, width, height]| {
        let origin = window.inner_position().ok()?;
        let scale = window.scale_factor().ok()?;
        let px = |v: f64| (v * scale).round() as i32;
        Some([origin.x + px(left), origin.y + px(top), origin.x + px(left + width), origin.y + px(top + height)])
    });
    *TOOLTIP_ANCHOR.lock().unwrap_or_else(|e| e.into_inner()) = anchor;
}

pub fn present_tooltip(app: &AppHandle, width: f64, height: f64) {
    let Some(anchor) = *TOOLTIP_ANCHOR.lock().unwrap_or_else(|e| e.into_inner()) else { return };
    let Some(window) = app.get_webview_window(TOOLTIP_LABEL) else { return };
    let Some(hwnd) = hwnd(&window) else { return };
    let Some(monitor) = app.monitor_from_point(f64::from(anchor[0]), f64::from(anchor[1])).ok().flatten() else { return };
    let scale = monitor.scale_factor();
    let size = ((width * scale).round() as i32, (height * scale).round() as i32);
    let area = monitor.work_area();
    let (x, y) = tooltip_position(
        anchor,
        size,
        (area.position.x, area.position.y, area.size.width as i32, area.size.height as i32),
        (TOOLTIP_GAP * scale).round() as i32,
    );
    unsafe {
        let _ = SetWindowPos(hwnd, Some(HWND_TOPMOST), x, y, size.0, size.1, SWP_NOACTIVATE);
        let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), 255, LWA_ALPHA);
    }
}

pub fn hide_tooltip(app: &AppHandle) {
    *TOOLTIP_ANCHOR.lock().unwrap_or_else(|e| e.into_inner()) = None;
    let Some(hwnd) = app.get_webview_window(TOOLTIP_LABEL).as_ref().and_then(hwnd) else { return };
    unsafe {
        let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), 0, LWA_ALPHA);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notes_window_spawns_against_the_right_edge() {
        assert_eq!(right_edge_position((0, 0, 1920, 1032), (440, 762), 16), (1464, 135));
        assert_eq!(right_edge_position((1920, 0, 1280, 600), (440, 762), 16), (2744, 0));
    }

    #[test]
    fn tooltip_sits_above_its_button_and_flips_below_at_the_screen_top() {
        let area = (0, 0, 1920, 1032);
        assert_eq!(tooltip_position([100, 300, 124, 324], (120, 30), area, 6), (52, 264));
        assert_eq!(tooltip_position([0, 10, 24, 34], (120, 30), area, 6), (0, 40));
        assert_eq!(tooltip_position([1910, 300, 1920, 324], (120, 30), area, 6), (1800, 264));
    }
}
