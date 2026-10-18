use crate::launcher::{hwnd, make_popup, set_dwm};
use tauri::{AppHandle, Manager, WebviewWindow};
use windows::Win32::Foundation::COLORREF;
use windows::Win32::Graphics::Dwm::{DWMWA_BORDER_COLOR, DWMWA_TRANSITIONS_FORCEDISABLED, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUNDSMALL};
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowLongPtrW, SetLayeredWindowAttributes, SetWindowLongPtrW, SetWindowPos, ShowWindow, GWL_EXSTYLE, HWND_TOPMOST, LWA_ALPHA,
    SWP_NOACTIVATE, SW_SHOWNOACTIVATE, WS_EX_APPWINDOW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT,
};

pub const LABEL: &str = "hud";
const BOTTOM_FRACTION: f64 = 0.12;

pub fn init(window: &WebviewWindow) {
    let Some(hwnd) = hwnd(window) else { return };
    make_popup(hwnd, 1, 1);
    set_dwm(hwnd, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUNDSMALL);
    set_dwm(hwnd, DWMWA_BORDER_COLOR, 0x0041_4141u32);
    set_dwm(hwnd, DWMWA_TRANSITIONS_FORCEDISABLED, 1i32);
    unsafe {
        let ex = (GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32 & !WS_EX_APPWINDOW.0)
            | (WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TRANSPARENT).0;
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex as isize);
        let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), 0, LWA_ALPHA);
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
    }
}

pub fn present(app: &AppHandle, width: f64, height: f64) {
    let Some(window) = app.get_webview_window(LABEL) else { return };
    let Some(hwnd) = hwnd(&window) else { return };
    let monitor = app
        .cursor_position()
        .ok()
        .and_then(|p| app.monitor_from_point(p.x, p.y).ok().flatten())
        .or_else(|| window.primary_monitor().ok().flatten());
    let Some(monitor) = monitor else { return };
    let scale = monitor.scale_factor();
    let (w, h) = ((width * scale).round() as i32, (height * scale).round() as i32);
    let area = monitor.work_area();
    let x = area.position.x + (area.size.width as i32 - w) / 2;
    let y = area.position.y + area.size.height as i32 - h - (area.size.height as f64 * BOTTOM_FRACTION).round() as i32;
    unsafe {
        let _ = SetWindowPos(hwnd, Some(HWND_TOPMOST), x, y, w, h, SWP_NOACTIVATE);
        let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), 255, LWA_ALPHA);
    }
}

pub fn hide(app: &AppHandle) {
    let Some(hwnd) = app.get_webview_window(LABEL).as_ref().and_then(hwnd) else { return };
    unsafe {
        let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), 0, LWA_ALPHA);
    }
}
