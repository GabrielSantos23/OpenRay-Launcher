use crate::store::ShowOn;
use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
use windows::Win32::Foundation::{CloseHandle, COLORREF, HWND, POINT, RECT};
use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_BORDER_COLOR, DWMWA_TRANSITIONS_FORCEDISABLED, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND, DWMWINDOWATTRIBUTE};
use windows::Win32::Graphics::Gdi::{GetMonitorInfoW, MonitorFromPoint, MonitorFromWindow, HMONITOR, MONITORINFO, MONITOR_DEFAULTTONEAREST, MONITOR_DEFAULTTOPRIMARY};
use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId, OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION};
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
use windows::Win32::UI::Input::KeyboardAndMouse::{SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP, VK_ESCAPE};
use windows::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, GetAncestor, GetCursorPos, GetForegroundWindow, GetShellWindow, GetWindowLongPtrW, GetWindowRect, GetWindowThreadProcessId, IsWindow,
    SetForegroundWindow, SetLayeredWindowAttributes, SetWindowLongPtrW, SetWindowPos, GA_ROOT, GWL_EXSTYLE, HWND_NOTOPMOST, HWND_TOPMOST, LWA_ALPHA,
    SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, WS_EX_APPWINDOW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT,
};

pub const WIDTH: f32 = 750.0;
pub const HEIGHT: f32 = 475.0;
const TOP_FRACTION: f64 = 0.353;

static SHOWN: AtomicBool = AtomicBool::new(false);
static PREVIOUS: AtomicIsize = AtomicIsize::new(0);
static LAUNCHER: AtomicIsize = AtomicIsize::new(0);

pub fn hwnd_of(handle: &impl raw_window_handle::HasWindowHandle) -> Option<HWND> {
    match handle.window_handle().ok()?.as_raw() {
        raw_window_handle::RawWindowHandle::Win32(h) => Some(HWND(h.hwnd.get() as _)),
        _ => None,
    }
}

fn launcher() -> HWND {
    HWND(LAUNCHER.load(Ordering::SeqCst) as _)
}

pub fn window() -> Option<HWND> {
    let hwnd = launcher();
    (!hwnd.is_invalid()).then_some(hwnd)
}

pub fn set_dwm<T>(hwnd: HWND, attribute: DWMWINDOWATTRIBUTE, value: T) {
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

pub fn init(hwnd: HWND) {
    LAUNCHER.store(hwnd.0 as isize, Ordering::SeqCst);
    set_dwm(hwnd, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND);
    set_dwm(hwnd, DWMWA_BORDER_COLOR, crate::theme::WINDOW_BORDER());
    set_dwm(hwnd, DWMWA_TRANSITIONS_FORCEDISABLED, 1i32);
    set_hidden_styles(hwnd, true);
}

fn cursor() -> POINT {
    let mut point = POINT::default();
    unsafe {
        let _ = GetCursorPos(&mut point);
    }
    point
}

fn place(hwnd: HWND, previous: HWND, show_on: ShowOn) {
    let point = match show_on {
        ShowOn::MouseScreen => Some(cursor()),
        ShowOn::ActiveScreen => {
            let mut rect = RECT::default();
            unsafe { GetWindowRect(previous, &mut rect).is_ok() }.then(|| POINT { x: (rect.left + rect.right) / 2, y: (rect.top + rect.bottom) / 2 }).or_else(|| Some(cursor()))
        }
        ShowOn::PrimaryScreen => None,
    };
    unsafe {
        let monitor = MonitorFromPoint(point.unwrap_or_default(), if point.is_some() { MONITOR_DEFAULTTONEAREST } else { MONITOR_DEFAULTTOPRIMARY });
        let mut info = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
        if !GetMonitorInfoW(monitor, &mut info).as_bool() {
            return;
        }
        let (mut dpi_x, mut dpi_y) = (96, 96);
        let _ = GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y);
        let scale = dpi_x as f64 / 96.0;
        let (width, height) = ((WIDTH as f64 * scale).round() as i32, (HEIGHT as f64 * scale).round() as i32);
        let area = info.rcWork;
        let x = area.left + (area.right - area.left - width) / 2;
        let y = (area.top + ((area.bottom - area.top) as f64 * TOP_FRACTION).round() as i32).min(area.bottom - height).max(area.top);
        let _ = SetWindowPos(hwnd, None, x, y, 0, 0, SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE);
    }
}

pub fn force_foreground(hwnd: HWND) {
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

fn process_path(hwnd: HWND) -> Option<String> {
    unsafe {
        let mut pid = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buffer = [0u16; 1024];
        let mut len = buffer.len() as u32;
        let ok = QueryFullProcessImageNameW(process, PROCESS_NAME_WIN32, windows::core::PWSTR(buffer.as_mut_ptr()), &mut len).is_ok();
        let _ = CloseHandle(process);
        ok.then(|| String::from_utf16_lossy(&buffer[..len as usize]))
    }
}

fn is_shell_flyout(hwnd: HWND) -> bool {
    process_path(hwnd).is_some_and(|path| {
        let path = path.to_ascii_lowercase();
        path.ends_with(r"\startmenuexperiencehost.exe") || path.ends_with(r"\searchhost.exe")
    })
}

fn close_shell_flyout() {
    let esc = |flags| INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: VK_ESCAPE, dwFlags: flags, dwExtraInfo: crate::hotkey::MARKER, ..Default::default() } },
    };
    unsafe { SendInput(&[esc(KEYBD_EVENT_FLAGS(0)), esc(KEYEVENTF_KEYUP)], std::mem::size_of::<INPUT>() as i32) };
}

pub fn hud_position(size: egui::Vec2) -> egui::Pos2 {
    unsafe {
        let monitor = MonitorFromPoint(cursor(), MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
        let _ = GetMonitorInfoW(monitor, &mut info);
        let (mut dpi_x, mut dpi_y) = (96, 96);
        let _ = GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y);
        let scale = dpi_x as f32 / 96.0;
        let area = info.rcWork;
        let (left, top, width, height) = (area.left as f32 / scale, area.top as f32 / scale, (area.right - area.left) as f32 / scale, (area.bottom - area.top) as f32 / scale);
        egui::pos2(left + (width - size.x) / 2.0, top + height - height * 0.12 - size.y)
    }
}

pub fn previous_window() -> HWND {
    HWND(PREVIOUS.load(Ordering::SeqCst) as _)
}

pub fn monitor() -> HMONITOR {
    unsafe { MonitorFromWindow(launcher(), MONITOR_DEFAULTTONEAREST) }
}

pub fn set_previous_window(hwnd: HWND) {
    PREVIOUS.store(hwnd.0 as isize, Ordering::SeqCst);
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

pub fn owns_foreground() -> bool {
    unsafe { GetAncestor(GetForegroundWindow(), GA_ROOT) == launcher() }
}

pub fn show(show_on: ShowOn) {
    let hwnd = launcher();
    if hwnd.is_invalid() {
        return;
    }
    if !is_shown() {
        let previous = unsafe { GetAncestor(GetForegroundWindow(), GA_ROOT) };
        if is_shell_flyout(previous) {
            close_shell_flyout();
        } else if previous != hwnd {
            PREVIOUS.store(previous.0 as isize, Ordering::SeqCst);
        }
        place(hwnd, previous, show_on);
    }
    SHOWN.store(true, Ordering::SeqCst);
    set_hidden_styles(hwnd, false);
    force_foreground(hwnd);
}

pub fn hide(restore_focus: bool) -> bool {
    let hwnd = launcher();
    if hwnd.is_invalid() || !SHOWN.swap(false, Ordering::SeqCst) {
        return false;
    }
    let owned = owns_foreground();
    set_hidden_styles(hwnd, true);
    if restore_focus && owned {
        let previous = previous_window();
        unsafe {
            let target = if !previous.is_invalid() && IsWindow(Some(previous)).as_bool() { previous } else { GetShellWindow() };
            let _ = SetForegroundWindow(target);
        }
    }
    true
}
