use crate::focus::{self, BlockInfo, Notice, Session};
use crate::launcher::{self, hwnd};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, WebviewWindow};
use windows::Win32::Foundation::{POINT, RECT};
use windows::Win32::Graphics::Dwm::{DWMWA_BORDER_COLOR, DWMWA_TRANSITIONS_FORCEDISABLED, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND};
use windows::Win32::UI::WindowsAndMessaging::{
    GetCursorPos, GetWindowLongPtrW, GetWindowRect, SetWindowLongPtrW, SetWindowPos, ShowWindow, GWL_EXSTYLE, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOSIZE, SW_HIDE,
    SW_SHOWNOACTIVATE, WS_EX_APPWINDOW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
};

pub const PANEL: &str = "focus-panel";
pub const TOAST: &str = "focus-toast";
pub const BLOCK: &str = "focus-block";
const PANEL_BOTTOM_MARGIN: f64 = 24.0;
const TOAST_TOP_MARGIN: f64 = 24.0;
const PANEL_SIZE: (f64, f64) = (265.0, 40.0);
const TOAST_SIZE: (f64, f64) = (392.0, 66.0);

static PANEL_POSITION: Mutex<Option<(i32, i32)>> = Mutex::new(None);
static HOVER_WATCH: AtomicBool = AtomicBool::new(false);

impl focus::Host for AppHandle {
    fn changed(&self, session: Option<&Session>) {
        let _ = self.emit("focus-session-changed", session);
    }

    fn main(&self, work: impl FnOnce(&Self) + Send + 'static) {
        let handle = self.clone();
        let _ = self.run_on_main_thread(move || work(&handle));
    }

    fn show_panel(&self) {
        crate::open_window(self, PANEL, |app, window| {
            popup_styles(&window, false, PANEL_SIZE);
            place_panel(app, &window, false);
            watch_hover(app);
        });
    }

    fn hide_panel(&self) {
        if let Some(panel) = self.get_webview_window(PANEL) {
            let _ = panel.destroy();
        }
    }

    fn show_toast(&self, notice: Notice) {
        crate::open_window(self, TOAST, move |app, window| {
            popup_styles(&window, false, TOAST_SIZE);
            if let (Some(hwnd), Some((origin, size, scale)), Ok(outer)) = (hwnd(&window), work_area(app, &window), window.outer_size()) {
                let x = origin.x + (size.width as i32 - outer.width as i32) / 2;
                let y = origin.y + (TOAST_TOP_MARGIN * scale).round() as i32;
                unsafe {
                    let _ = SetWindowPos(hwnd, Some(HWND_TOPMOST), x, y, 0, 0, SWP_NOSIZE | SWP_NOACTIVATE);
                    let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
                }
            }
            let _ = app.emit_to(TOAST, "focus-toast", notice.clone());
            let handle = app.clone();
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(400));
                let _ = handle.emit_to(TOAST, "focus-toast", notice);
            });
        });
    }

    fn hide_toast(&self) {
        hide_toast(self);
    }

    fn show_block(&self, block: &BlockInfo, visible: crate::window_layout::Rect) {
        let info = block.clone();
        crate::open_window(self, BLOCK, move |app, window| {
            popup_styles(&window, true, (800.0, 600.0));
            let Some(hwnd) = hwnd(&window) else { return };
            unsafe {
                let _ = SetWindowPos(hwnd, Some(HWND_TOPMOST), visible.left, visible.top, visible.right - visible.left, visible.bottom - visible.top, Default::default());
                let _ = ShowWindow(hwnd, windows::Win32::UI::WindowsAndMessaging::SW_SHOW);
            }
            launcher::force_foreground(hwnd);
            let _ = window.set_focus();
            let _ = app.emit_to(BLOCK, "focus-block", info.clone());
            let handle = app.clone();
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(400));
                let _ = handle.emit_to(BLOCK, "focus-block", info);
            });
        });
    }

    fn hide_block(&self) {
        if let Some(hwnd) = self.get_webview_window(BLOCK).as_ref().and_then(hwnd) {
            unsafe {
                let _ = ShowWindow(hwnd, SW_HIDE);
            }
        }
    }

    fn raise_block(&self) {
        if let Some(overlay) = self.get_webview_window(BLOCK).as_ref().and_then(hwnd) {
            launcher::force_foreground(overlay);
        }
    }
}

fn popup_styles(window: &WebviewWindow, activatable: bool, size: (f64, f64)) {
    let Some(hwnd) = hwnd(window) else { return };
    let scale = window.scale_factor().unwrap_or(1.0);
    launcher::make_popup(hwnd, (size.0 * scale).round() as i32, (size.1 * scale).round() as i32);
    launcher::set_dwm(hwnd, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND);
    launcher::set_dwm(hwnd, DWMWA_BORDER_COLOR, 0x003B_3B3Bu32);
    launcher::set_dwm(hwnd, DWMWA_TRANSITIONS_FORCEDISABLED, 1i32);
    unsafe {
        let mut ex = (GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32 & !WS_EX_APPWINDOW.0) | WS_EX_TOOLWINDOW.0 | WS_EX_TOPMOST.0;
        if !activatable {
            ex |= WS_EX_NOACTIVATE.0;
        }
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex as isize);
    }
}

fn work_area(app: &AppHandle, window: &WebviewWindow) -> Option<(tauri::PhysicalPosition<i32>, tauri::PhysicalSize<u32>, f64)> {
    let monitor = app
        .cursor_position()
        .ok()
        .and_then(|p| app.monitor_from_point(p.x, p.y).ok().flatten())
        .or_else(|| window.primary_monitor().ok().flatten())?;
    let area = monitor.work_area();
    Some((area.position, area.size, monitor.scale_factor()))
}

fn place_panel(app: &AppHandle, window: &WebviewWindow, reset: bool) {
    let Some(hwnd) = hwnd(window) else { return };
    let saved = if reset { None } else { *PANEL_POSITION.lock().unwrap_or_else(|e| e.into_inner()) };
    let (x, y) = match saved {
        Some(position) => position,
        None => {
            let Some((origin, size, scale)) = work_area(app, window) else { return };
            let Ok(outer) = window.outer_size() else { return };
            (
                origin.x + (size.width as i32 - outer.width as i32) / 2,
                origin.y + size.height as i32 - outer.height as i32 - (PANEL_BOTTOM_MARGIN * scale).round() as i32,
            )
        }
    };
    *PANEL_POSITION.lock().unwrap_or_else(|e| e.into_inner()) = Some((x, y));
    unsafe {
        let _ = SetWindowPos(hwnd, Some(HWND_TOPMOST), x, y, 0, 0, SWP_NOSIZE | SWP_NOACTIVATE);
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
    }
}

fn watch_hover(app: &AppHandle) {
    if HOVER_WATCH.swap(true, Ordering::SeqCst) {
        return;
    }
    let app = app.clone();
    std::thread::spawn(move || {
        let mut last = false;
        while let Some(window) = app.get_webview_window(PANEL) {
            let inside = hwnd(&window).is_some_and(|panel| unsafe {
                let mut cursor = POINT::default();
                let mut rect = RECT::default();
                GetCursorPos(&mut cursor).is_ok()
                    && GetWindowRect(panel, &mut rect).is_ok()
                    && cursor.x >= rect.left
                    && cursor.x < rect.right
                    && cursor.y >= rect.top
                    && cursor.y < rect.bottom
            });
            let hovered = inside || focus::MENU_OPEN.load(Ordering::SeqCst);
            if hovered != last {
                last = hovered;
                let _ = app.emit_to(PANEL, "focus-panel-hover", hovered);
            }
            std::thread::sleep(Duration::from_millis(80));
        }
        HOVER_WATCH.store(false, Ordering::SeqCst);
    });
}

pub fn reset_panel(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(PANEL) {
        place_panel(app, &window, true);
    }
}

pub fn panel_moved(window: &tauri::Window) {
    if let Ok(position) = window.outer_position() {
        *PANEL_POSITION.lock().unwrap_or_else(|e| e.into_inner()) = Some((position.x, position.y));
    }
}

pub fn hide_toast(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(TOAST) {
        let _ = window.destroy();
    }
}

pub fn panel_menu(app: &AppHandle, menu: &str, x: f64, y: f64) -> Result<(), String> {
    let window = app.get_webview_window(PANEL).ok_or("No focus panel")?;
    let hwnd = hwnd(&window).ok_or("No focus panel")?;
    let origin = window.inner_position().map_err(|e| e.to_string())?;
    let scale = window.scale_factor().unwrap_or(1.0);
    let point = POINT { x: origin.x + (x * scale).round() as i32, y: origin.y + (y * scale).round() as i32 };
    let choice = focus::track_menu(hwnd, point, &focus::menu_items(menu))?;
    match (menu, choice) {
        (_, 0) => Ok(()),
        ("pause", 99) => focus::pause(app, None),
        ("pause", minutes) => focus::pause(app, Some(u64::from(minutes) * 60_000)),
        (_, 1) => {
            launcher::show(app);
            let _ = app.emit_to(launcher::LABEL, "run-command", "focus/edit");
            Ok(())
        }
        (_, 2) => focus::cancel(app),
        (_, 3) => {
            reset_panel(app);
            Ok(())
        }
        _ => Ok(()),
    }
}
