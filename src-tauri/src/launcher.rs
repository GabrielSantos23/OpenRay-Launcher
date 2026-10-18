use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, WebviewWindow};

pub const LABEL: &str = "launcher";
const TOP_FRACTION: f64 = 0.353;

fn window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window(LABEL)
}

pub fn style(window: &WebviewWindow) {
    #[cfg(windows)]
    {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::Graphics::Dwm::{
            DwmSetWindowAttribute, DWMWA_BORDER_COLOR, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND,
        };
        let Ok(handle) = window.hwnd() else { return };
        let hwnd = HWND(handle.0 as _);
        let corner = DWMWCP_ROUND;
        let border: u32 = 0x003B_3B3B;
        unsafe {
            let _ = DwmSetWindowAttribute(hwnd, DWMWA_WINDOW_CORNER_PREFERENCE, &corner as *const _ as _, std::mem::size_of_val(&corner) as u32);
            let _ = DwmSetWindowAttribute(hwnd, DWMWA_BORDER_COLOR, &border as *const _ as _, std::mem::size_of_val(&border) as u32);
        }
    }
}

fn place(app: &AppHandle, window: &WebviewWindow) {
    let monitor = app
        .cursor_position()
        .ok()
        .and_then(|p| app.monitor_from_point(p.x, p.y).ok().flatten())
        .or_else(|| window.primary_monitor().ok().flatten());
    let (Some(monitor), Ok(size)) = (monitor, window.outer_size()) else { return };
    let area = monitor.work_area();
    let x = area.position.x + (area.size.width as i32 - size.width as i32) / 2;
    let y = area.position.y + (area.size.height as f64 * TOP_FRACTION).round() as i32;
    let max_y = area.position.y + area.size.height as i32 - size.height as i32;
    let _ = window.set_position(PhysicalPosition::new(x, y.min(max_y).max(area.position.y)));
}

#[cfg(windows)]
fn force_foreground(window: &WebviewWindow) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
    use windows::Win32::UI::WindowsAndMessaging::{BringWindowToTop, GetForegroundWindow, GetWindowThreadProcessId, SetForegroundWindow};
    let Ok(handle) = window.hwnd() else { return };
    let hwnd = HWND(handle.0 as _);
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

#[cfg(windows)]
pub fn owns_foreground(window: &WebviewWindow) -> bool {
    use windows::Win32::UI::WindowsAndMessaging::{GetAncestor, GetForegroundWindow, GA_ROOT};
    let Ok(handle) = window.hwnd() else { return false };
    unsafe { GetAncestor(GetForegroundWindow(), GA_ROOT).0 == handle.0 as _ }
}

#[cfg(not(windows))]
pub fn owns_foreground(window: &WebviewWindow) -> bool {
    window.is_focused().unwrap_or(false)
}

pub fn show(app: &AppHandle) {
    let Some(window) = window(app) else { return };
    place(app, &window);
    let _ = window.show();
    #[cfg(windows)]
    {
        let target = window.clone();
        let _ = app.run_on_main_thread(move || {
            force_foreground(&target);
            let _ = target.set_focus();
        });
    }
    #[cfg(not(windows))]
    let _ = window.set_focus();
    let _ = app.emit_to(LABEL, "launcher-shown", ());
}

pub fn hide(app: &AppHandle) {
    if let Some(window) = window(app) {
        let _ = window.hide();
    }
}

pub fn toggle(app: &AppHandle) {
    match window(app).map(|w| w.is_visible().unwrap_or(false)) {
        Some(true) => hide(app),
        Some(false) => show(app),
        None => {}
    }
}
