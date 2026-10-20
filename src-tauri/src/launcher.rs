use crate::store::{ShowOn, Store};
use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, WebviewWindow};
use windows::Win32::Foundation::{COLORREF, HWND, RECT};
use windows::Win32::Graphics::Dwm::{
    DwmSetWindowAttribute, DWMWA_BORDER_COLOR, DWMWA_TRANSITIONS_FORCEDISABLED, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND,
};
use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
use windows::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, GetAncestor, GetForegroundWindow, GetShellWindow, GetWindowLongPtrW, GetWindowRect, GetWindowThreadProcessId, IsWindow,
    SetForegroundWindow, SetLayeredWindowAttributes, SetWindowLongPtrW, SetWindowPos, ShowWindow, GA_ROOT, GWL_EXSTYLE,
    GWL_STYLE, HWND_NOTOPMOST, HWND_TOPMOST, LWA_ALPHA, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
    SWP_NOZORDER, SW_SHOWNOACTIVATE, WS_CAPTION, WS_EX_APPWINDOW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT,
    WS_MAXIMIZEBOX, WS_MINIMIZEBOX, WS_POPUP, WS_SYSMENU, WS_THICKFRAME,
};

pub const LABEL: &str = "launcher";
const TOP_FRACTION: f64 = 0.353;
const WIDTH: f64 = 750.0;
const HEIGHT: f64 = 475.0;

static SHOWN: AtomicBool = AtomicBool::new(false);
static PREVIOUS_FOREGROUND: AtomicIsize = AtomicIsize::new(0);

fn window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window(LABEL)
}

pub(crate) fn hwnd(window: &WebviewWindow) -> Option<HWND> {
    window.hwnd().ok().map(|h| HWND(h.0 as _))
}

pub(crate) fn set_dwm<T>(hwnd: HWND, attribute: windows::Win32::Graphics::Dwm::DWMWINDOWATTRIBUTE, value: T) {
    unsafe {
        let _ = DwmSetWindowAttribute(hwnd, attribute, &value as *const T as _, std::mem::size_of::<T>() as u32);
    }
}

fn set_hidden_styles(hwnd: HWND, hidden: bool) {
    unsafe {
        let ex = (GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32 & !WS_EX_APPWINDOW.0) | WS_EX_LAYERED.0 | WS_EX_TOOLWINDOW.0;
        let toggled = WS_EX_TRANSPARENT.0 | WS_EX_NOACTIVATE.0;
        let ex = if hidden { ex | toggled } else { ex & !toggled };
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex as isize);
        let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), if hidden { 0 } else { 255 }, LWA_ALPHA);
        let z = if hidden { HWND_NOTOPMOST } else { HWND_TOPMOST };
        let _ = SetWindowPos(hwnd, Some(z), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
    }
}

pub(crate) fn make_popup(hwnd: HWND, width: i32, height: i32) {
    unsafe {
        let frame = (WS_CAPTION | WS_THICKFRAME | WS_SYSMENU | WS_MINIMIZEBOX | WS_MAXIMIZEBOX).0;
        let style = (GetWindowLongPtrW(hwnd, GWL_STYLE) as u32 & !frame) | WS_POPUP.0;
        SetWindowLongPtrW(hwnd, GWL_STYLE, style as isize);
        let _ = SetWindowPos(hwnd, None, 0, 0, width, height, SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED);
    }
}

pub fn init(window: &WebviewWindow) {
    let Some(hwnd) = hwnd(window) else { return };
    let scale = window.scale_factor().unwrap_or(1.0);
    make_popup(hwnd, (WIDTH * scale).round() as i32, (HEIGHT * scale).round() as i32);
    set_dwm(hwnd, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND);
    set_dwm(hwnd, DWMWA_BORDER_COLOR, 0x003B_3B3Bu32);
    set_dwm(hwnd, DWMWA_TRANSITIONS_FORCEDISABLED, 1i32);
    set_hidden_styles(hwnd, true);
    set_memory_target(window, true);
    unsafe {
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
    }
}

fn active_window_center(previous: HWND) -> Option<(f64, f64)> {
    let mut rect = RECT::default();
    unsafe { GetWindowRect(previous, &mut rect).ok()? };
    Some((f64::from(rect.left + rect.right) / 2.0, f64::from(rect.top + rect.bottom) / 2.0))
}

fn place(app: &AppHandle, window: &WebviewWindow, previous: HWND) {
    let point = match app.state::<Store>().settings().show_on {
        ShowOn::MouseScreen => app.cursor_position().ok().map(|p| (p.x, p.y)),
        ShowOn::ActiveScreen => active_window_center(previous).or_else(|| app.cursor_position().ok().map(|p| (p.x, p.y))),
        ShowOn::PrimaryScreen => None,
    };
    let monitor = point
        .and_then(|(x, y)| app.monitor_from_point(x, y).ok().flatten())
        .or_else(|| window.primary_monitor().ok().flatten());
    let (Some(monitor), Ok(size)) = (monitor, window.outer_size()) else { return };
    let area = monitor.work_area();
    let x = area.position.x + (area.size.width as i32 - size.width as i32) / 2;
    let y = area.position.y + (area.size.height as f64 * TOP_FRACTION).round() as i32;
    let max_y = area.position.y + area.size.height as i32 - size.height as i32;
    let _ = window.set_position(PhysicalPosition::new(x, y.min(max_y).max(area.position.y)));
}

pub(crate) fn set_memory_target(window: &WebviewWindow, low: bool) {
    use webview2_com::Microsoft::Web::WebView2::Win32::{ICoreWebView2_19, COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW, COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL};
    use webview_core::Interface;
    let _ = window.with_webview(move |webview| unsafe {
        let Ok(core) = webview.controller().CoreWebView2() else { return };
        if let Ok(core) = core.cast::<ICoreWebView2_19>() {
            let _ = core.SetMemoryUsageTargetLevel(if low { COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW } else { COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL });
        }
    });
}

pub(crate) fn force_foreground(hwnd: HWND) {
    unsafe {
        let current = GetCurrentThreadId();
        let foreground = GetWindowThreadProcessId(GetForegroundWindow(), None);
        let attach = foreground != 0 && foreground != current;
        if attach {
            let _ = AttachThreadInput(foreground, current, true);
        }
        let _ = BringWindowToTop(hwnd);
        let _ = SetForegroundWindow(hwnd);
        if attach {
            let _ = AttachThreadInput(foreground, current, false);
        }
    }
}

pub fn owns_foreground(window: &WebviewWindow) -> bool {
    let Some(hwnd) = hwnd(window) else { return false };
    unsafe { GetAncestor(GetForegroundWindow(), GA_ROOT) == hwnd }
}

pub fn previous_window() -> HWND {
    HWND(PREVIOUS_FOREGROUND.load(Ordering::SeqCst) as _)
}

pub(crate) fn set_previous_window(hwnd: HWND) {
    PREVIOUS_FOREGROUND.store(hwnd.0 as isize, Ordering::SeqCst);
}

pub(crate) fn monitor(app: &AppHandle) -> Option<windows::Win32::Graphics::Gdi::HMONITOR> {
    let hwnd = hwnd(&window(app)?)?;
    Some(unsafe { windows::Win32::Graphics::Gdi::MonitorFromWindow(hwnd, windows::Win32::Graphics::Gdi::MONITOR_DEFAULTTONEAREST) })
}

pub fn active_window() -> HWND {
    if is_shown() {
        previous_window()
    } else {
        unsafe { GetAncestor(GetForegroundWindow(), GA_ROOT) }
    }
}

pub fn is_shown() -> bool {
    SHOWN.load(Ordering::SeqCst)
}

pub fn show(app: &AppHandle) {
    let Some(window) = window(app) else { return };
    let Some(hwnd) = hwnd(&window) else { return };
    if !is_shown() {
        let previous = unsafe { GetAncestor(GetForegroundWindow(), GA_ROOT) };
        if previous != hwnd {
            PREVIOUS_FOREGROUND.store(previous.0 as isize, Ordering::SeqCst);
        }
        place(app, &window, previous);
    }
    set_memory_target(&window, false);
    SHOWN.store(true, Ordering::SeqCst);
    set_hidden_styles(hwnd, false);
    force_foreground(hwnd);
    let _ = window.set_focus();
    let _ = app.emit_to(LABEL, "launcher-shown", ());
}

pub fn hide(app: &AppHandle, restore_focus: bool) {
    let Some(window) = window(app) else { return };
    let Some(hwnd) = hwnd(&window) else { return };
    if !SHOWN.swap(false, Ordering::SeqCst) {
        return;
    }
    set_hidden_styles(hwnd, true);
    set_memory_target(&window, true);
    let _ = app.emit_to(LABEL, "launcher-hidden", ());
    if restore_focus && owns_foreground(&window) {
        let previous = HWND(PREVIOUS_FOREGROUND.load(Ordering::SeqCst) as _);
        unsafe {
            let target = if !previous.is_invalid() && IsWindow(Some(previous)).as_bool() { previous } else { GetShellWindow() };
            let _ = SetForegroundWindow(target);
        }
    }
}

pub fn toggle(app: &AppHandle) {
    if is_shown() {
        hide(app, true);
    } else {
        show(app);
    }
}
