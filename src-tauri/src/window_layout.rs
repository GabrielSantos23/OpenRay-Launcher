use crate::{launcher, system_actions};
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use windows::core::BOOL;
use windows::Win32::Foundation::{HWND, LPARAM, RECT};
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_EXTENDED_FRAME_BOUNDS};
use windows::Win32::Graphics::Gdi::{EnumDisplayMonitors, GetMonitorInfoW, MonitorFromWindow, HDC, HMONITOR, MONITORINFO, MONITOR_DEFAULTTONEAREST};
use windows::Win32::UI::WindowsAndMessaging::{
    GetShellWindow, GetWindowLongPtrW, GetWindowRect, IsIconic, IsWindow, IsWindowVisible, IsZoomed, SetWindowPos, ShowWindow, GWL_EXSTYLE,
    HWND_NOTOPMOST, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SW_MAXIMIZE, SW_MINIMIZE, SW_RESTORE, WS_EX_TOPMOST,
};

const ALMOST_MAXIMIZE: f64 = 0.9;
const REASONABLE: (f64, i32, i32) = (0.6, 1024, 900);
const STEP: i32 = 32;

static PREVIOUS_FRAMES: LazyLock<Mutex<HashMap<isize, Rect>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Rect {
    fn width(&self) -> i32 {
        self.right - self.left
    }
    fn height(&self) -> i32 {
        self.bottom - self.top
    }
}

impl From<RECT> for Rect {
    fn from(r: RECT) -> Self {
        Rect { left: r.left, top: r.top, right: r.right, bottom: r.bottom }
    }
}

fn fractions(layout: &str) -> Option<[f64; 4]> {
    const T: f64 = 1.0 / 3.0;
    Some(match layout {
        "maximize" => [0.0, 1.0, 0.0, 1.0],
        "left-half" => [0.0, 0.5, 0.0, 1.0],
        "right-half" => [0.5, 1.0, 0.0, 1.0],
        "top-half" => [0.0, 1.0, 0.0, 0.5],
        "bottom-half" => [0.0, 1.0, 0.5, 1.0],
        "first-third" => [0.0, T, 0.0, 1.0],
        "center-third" => [T, 2.0 * T, 0.0, 1.0],
        "last-third" => [2.0 * T, 1.0, 0.0, 1.0],
        "first-two-thirds" => [0.0, 2.0 * T, 0.0, 1.0],
        "center-two-thirds" => [T / 2.0, 1.0 - T / 2.0, 0.0, 1.0],
        "last-two-thirds" => [T, 1.0, 0.0, 1.0],
        "top-third" => [0.0, 1.0, 0.0, T],
        "middle-third" => [0.0, 1.0, T, 2.0 * T],
        "bottom-third" => [0.0, 1.0, 2.0 * T, 1.0],
        "top-two-thirds" => [0.0, 1.0, 0.0, 2.0 * T],
        "bottom-two-thirds" => [0.0, 1.0, T, 1.0],
        "center-half" => [0.25, 0.75, 0.0, 1.0],
        "first-three-fourths" => [0.0, 0.75, 0.0, 1.0],
        "center-three-fourths" => [0.125, 0.875, 0.0, 1.0],
        "last-three-fourths" => [0.25, 1.0, 0.0, 1.0],
        "top-three-fourths" => [0.0, 1.0, 0.0, 0.75],
        "bottom-three-fourths" => [0.0, 1.0, 0.25, 1.0],
        "first-fourth" => [0.0, 0.25, 0.0, 1.0],
        "second-fourth" => [0.25, 0.5, 0.0, 1.0],
        "third-fourth" => [0.5, 0.75, 0.0, 1.0],
        "last-fourth" => [0.75, 1.0, 0.0, 1.0],
        "top-first-fourth" => [0.0, 1.0, 0.0, 0.25],
        "top-second-fourth" => [0.0, 1.0, 0.25, 0.5],
        "top-third-fourth" => [0.0, 1.0, 0.5, 0.75],
        "top-last-fourth" => [0.0, 1.0, 0.75, 1.0],
        "top-center-two-thirds" => [T / 2.0, 1.0 - T / 2.0, 0.0, 0.5],
        "bottom-center-two-thirds" => [T / 2.0, 1.0 - T / 2.0, 0.5, 1.0],
        "top-left-quarter" => [0.0, 0.5, 0.0, 0.5],
        "top-right-quarter" => [0.5, 1.0, 0.0, 0.5],
        "bottom-left-quarter" => [0.0, 0.5, 0.5, 1.0],
        "bottom-right-quarter" => [0.5, 1.0, 0.5, 1.0],
        "top-left-sixth" => [0.0, T, 0.0, 0.5],
        "top-center-sixth" => [T, 2.0 * T, 0.0, 0.5],
        "top-right-sixth" => [2.0 * T, 1.0, 0.0, 0.5],
        "bottom-left-sixth" => [0.0, T, 0.5, 1.0],
        "bottom-center-sixth" => [T, 2.0 * T, 0.5, 1.0],
        "bottom-right-sixth" => [2.0 * T, 1.0, 0.5, 1.0],
        _ => return None,
    })
}

fn sized(width: i32, height: i32, left: i32, top: i32, work: Rect) -> Rect {
    let width = width.min(work.width());
    let height = height.min(work.height());
    let left = left.clamp(work.left, work.right - width);
    let top = top.clamp(work.top, work.bottom - height);
    Rect { left, top, right: left + width, bottom: top + height }
}

fn centered(width: i32, height: i32, work: Rect) -> Rect {
    sized(width, height, work.left + (work.width() - width.min(work.width())) / 2, work.top + (work.height() - height.min(work.height())) / 2, work)
}

fn resized(window: Rect, delta: i32, work: Rect) -> Rect {
    let width = (window.width() + delta).clamp((work.width() / 4).min(window.width()), work.width());
    let height = (window.height() + delta).clamp((work.height() / 4).min(window.height()), work.height());
    let left = window.left - (width - window.width()) / 2;
    let top = window.top - (height - window.height()) / 2;
    sized(width, height, left, top, work)
}

pub fn frame(layout: &str, window: Rect, work: Rect) -> Option<Rect> {
    let (w, h) = (window.width(), window.height());
    let middle_x = work.left + (work.width() - w.min(work.width())) / 2;
    let middle_y = work.top + (work.height() - h.min(work.height())) / 2;
    let scaled = |f: f64, total: i32| (f64::from(total) * f).round() as i32;
    match layout {
        "center" => return Some(centered(w, h, work)),
        "almost-maximize" => return Some(centered(scaled(ALMOST_MAXIMIZE, work.width()), scaled(ALMOST_MAXIMIZE, work.height()), work)),
        "reasonable-size" => {
            let (f, max_w, max_h) = REASONABLE;
            return Some(centered(scaled(f, work.width()).min(max_w), scaled(f, work.height()).min(max_h), work));
        }
        "maximize-height" => return Some(sized(w, work.height(), window.left, work.top, work)),
        "maximize-width" => return Some(sized(work.width(), h, work.left, window.top, work)),
        "move-left" => return Some(sized(w, h, work.left, middle_y, work)),
        "move-right" => return Some(sized(w, h, work.right - w.min(work.width()), middle_y, work)),
        "move-top" => return Some(sized(w, h, middle_x, work.top, work)),
        "move-bottom" => return Some(sized(w, h, middle_x, work.bottom - h.min(work.height()), work)),
        "make-larger" => return Some(resized(window, STEP, work)),
        "make-smaller" => return Some(resized(window, -STEP, work)),
        _ => {}
    }
    let [x0, x1, y0, y1] = fractions(layout)?;
    let x = |f: f64| work.left + (f64::from(work.width()) * f).round() as i32;
    let y = |f: f64| work.top + (f64::from(work.height()) * f).round() as i32;
    Some(Rect { left: x(x0), top: y(y0), right: x(x1), bottom: y(y1) })
}

pub fn move_between(window: Rect, from: Rect, to: Rect) -> Rect {
    let sx = f64::from(to.width()) / f64::from(from.width());
    let sy = f64::from(to.height()) / f64::from(from.height());
    let scale = sx.min(sy);
    let width = ((f64::from(window.width()) * scale).round() as i32).min(to.width());
    let height = ((f64::from(window.height()) * scale).round() as i32).min(to.height());
    let left = to.left + (f64::from(window.left - from.left) * sx).round() as i32;
    let top = to.top + (f64::from(window.top - from.top) * sy).round() as i32;
    let left = left.clamp(to.left, to.right - width);
    let top = top.clamp(to.top, to.bottom - height);
    Rect { left, top, right: left + width, bottom: top + height }
}

pub fn neighbour(displays: &[Rect], current: Rect, step: isize) -> Option<usize> {
    let mut order: Vec<usize> = (0..displays.len()).collect();
    order.sort_by_key(|&i| (displays[i].left, displays[i].top));
    let position = order.iter().position(|&i| displays[i] == current)?;
    let next = (position as isize + step).rem_euclid(order.len() as isize) as usize;
    Some(order[next])
}

struct Display {
    bounds: Rect,
    work: Rect,
}

fn display_info(monitor: HMONITOR) -> Option<Display> {
    let mut info = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
    unsafe { GetMonitorInfoW(monitor, &mut info).ok().ok()? };
    Some(Display { bounds: info.rcMonitor.into(), work: info.rcWork.into() })
}

fn displays() -> Vec<Display> {
    unsafe extern "system" fn collect(monitor: HMONITOR, _: HDC, _: *mut RECT, data: LPARAM) -> BOOL {
        let list = unsafe { &mut *(data.0 as *mut Vec<HMONITOR>) };
        list.push(monitor);
        true.into()
    }
    let mut monitors: Vec<HMONITOR> = Vec::new();
    unsafe {
        let _ = EnumDisplayMonitors(None, None, Some(collect), LPARAM(&mut monitors as *mut _ as isize));
    }
    monitors.into_iter().filter_map(display_info).collect()
}

fn target() -> Result<HWND, String> {
    let hwnd = launcher::active_window();
    let usable = !hwnd.is_invalid() && unsafe { IsWindow(Some(hwnd)).as_bool() && IsWindowVisible(hwnd).as_bool() && hwnd != GetShellWindow() };
    if usable {
        Ok(hwnd)
    } else {
        Err("No active window".into())
    }
}

pub(crate) fn outer_and_visible(hwnd: HWND) -> Result<(Rect, Rect), String> {
    let mut outer = RECT::default();
    let mut visible = RECT::default();
    unsafe {
        GetWindowRect(hwnd, &mut outer).map_err(|e| e.to_string())?;
        if DwmGetWindowAttribute(hwnd, DWMWA_EXTENDED_FRAME_BOUNDS, &mut visible as *mut _ as *mut _, std::mem::size_of::<RECT>() as u32).is_err() {
            visible = outer;
        }
    }
    Ok((outer.into(), visible.into()))
}

fn place_visible(hwnd: HWND, target: Rect) -> Result<(), String> {
    let (outer, visible) = outer_and_visible(hwnd)?;
    let left = target.left - (visible.left - outer.left);
    let top = target.top - (visible.top - outer.top);
    let right = target.right + (outer.right - visible.right);
    let bottom = target.bottom + (outer.bottom - visible.bottom);
    unsafe { SetWindowPos(hwnd, None, left, top, right - left, bottom - top, SWP_NOZORDER | SWP_NOACTIVATE) }
        .map_err(|_| "This window can't be moved. It may be running as administrator.".to_string())
}

pub fn apply(layout: &str, launcher_monitor: Option<HMONITOR>) -> Result<(), String> {
    if layout == "minimize-on-display" {
        let monitor = launcher_monitor.ok_or("No display found")?;
        for hwnd in system_actions::app_windows() {
            if unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) } == monitor {
                unsafe {
                    let _ = ShowWindow(hwnd, SW_MINIMIZE);
                }
            }
        }
        launcher::set_previous_window(next_visible(HWND::default()));
        return Ok(());
    }
    apply_to(target()?, layout)
}

fn next_visible(skip: HWND) -> HWND {
    system_actions::app_windows().into_iter().find(|&w| w != skip && !unsafe { IsIconic(w) }.as_bool()).unwrap_or_default()
}

fn app_window_cycle(hwnd: HWND, forward: bool) -> Result<(), String> {
    let pid = system_actions::process_of(hwnd);
    let windows: Vec<HWND> = system_actions::app_windows().into_iter().filter(|&w| system_actions::process_of(w) == pid).collect();
    if windows.len() < 2 {
        return Err("The app has no other windows".into());
    }
    let next = if forward { windows[windows.len() - 1] } else { windows[1] };
    unsafe {
        if IsIconic(next).as_bool() {
            let _ = ShowWindow(next, SW_RESTORE);
        }
        if !forward {
            let _ = SetWindowPos(hwnd, Some(windows[windows.len() - 1]), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
        }
    }
    launcher::set_previous_window(next);
    Ok(())
}

fn apply_to(hwnd: HWND, layout: &str) -> Result<(), String> {
    match layout {
        "minimize" => {
            unsafe {
                let _ = ShowWindow(hwnd, SW_MINIMIZE);
            }
            launcher::set_previous_window(next_visible(hwnd));
            return Ok(());
        }
        "toggle-always-on-top" => {
            let topmost = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) } as u32 & WS_EX_TOPMOST.0 != 0;
            let after = if topmost { HWND_NOTOPMOST } else { HWND_TOPMOST };
            return unsafe { SetWindowPos(hwnd, Some(after), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE) }
                .map_err(|_| "This window can't be changed. It may be running as administrator.".to_string());
        }
        "focus-next-app-window" => return app_window_cycle(hwnd, true),
        "focus-previous-app-window" => return app_window_cycle(hwnd, false),
        "restore" => {
            let previous = PREVIOUS_FRAMES.lock().unwrap().remove(&(hwnd.0 as isize));
            if previous.is_none() || unsafe { IsZoomed(hwnd).as_bool() || IsIconic(hwnd).as_bool() } {
                unsafe {
                    let _ = ShowWindow(hwnd, SW_RESTORE);
                }
            }
            return previous.map_or(Ok(()), |frame| place_visible(hwnd, frame));
        }
        _ => {}
    }
    let zoomed = unsafe { IsZoomed(hwnd).as_bool() };
    if zoomed || unsafe { IsIconic(hwnd).as_bool() } {
        unsafe {
            let _ = ShowWindow(hwnd, SW_RESTORE);
        }
    }
    let current = display_info(unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) }).ok_or("No display found")?;
    let (_, visible) = outer_and_visible(hwnd)?;
    if let Some(step) = match layout {
        "next-display" => Some(1),
        "previous-display" => Some(-1),
        _ => None,
    } {
        let all = displays();
        let bounds: Vec<Rect> = all.iter().map(|d| d.bounds).collect();
        let index = neighbour(&bounds, current.bounds, step).ok_or("No display found")?;
        if bounds[index] == current.bounds {
            return Err("Only one display is connected".into());
        }
        place_visible(hwnd, move_between(visible, current.work, all[index].work))?;
        if zoomed {
            unsafe {
                let _ = ShowWindow(hwnd, SW_MAXIMIZE);
            }
        }
        return Ok(());
    }
    let frame = frame(layout, visible, current.work).ok_or_else(|| format!("Unknown layout {layout}"))?;
    PREVIOUS_FRAMES.lock().unwrap().insert(hwnd.0 as isize, visible);
    place_visible(hwnd, frame)
}

#[cfg(test)]
mod tests {
    use super::*;

    const WORK: Rect = Rect { left: 0, top: 0, right: 1920, bottom: 1032 };

    fn r(left: i32, top: i32, right: i32, bottom: i32) -> Rect {
        Rect { left, top, right, bottom }
    }

    #[test]
    fn halves_and_thirds_tile_the_work_area_without_gaps() {
        let window = r(100, 100, 900, 700);
        assert_eq!(frame("left-half", window, WORK), Some(r(0, 0, 960, 1032)));
        assert_eq!(frame("right-half", window, WORK), Some(r(960, 0, 1920, 1032)));
        assert_eq!(frame("bottom-half", window, WORK), Some(r(0, 516, 1920, 1032)));
        assert_eq!(frame("first-third", window, WORK), Some(r(0, 0, 640, 1032)));
        assert_eq!(frame("center-third", window, WORK), Some(r(640, 0, 1280, 1032)));
        assert_eq!(frame("last-third", window, WORK), Some(r(1280, 0, 1920, 1032)));
        assert_eq!(frame("center-two-thirds", window, WORK), Some(r(320, 0, 1600, 1032)));
        assert_eq!(frame("middle-third", window, WORK), Some(r(0, 344, 1920, 688)));
        assert_eq!(frame("maximize", window, WORK), Some(WORK));
        assert_eq!(frame("nope", window, WORK), None);
    }

    #[test]
    fn fourths_quarters_and_sixths_tile_the_work_area() {
        let window = r(100, 100, 900, 700);
        assert_eq!(frame("center-half", window, WORK), Some(r(480, 0, 1440, 1032)));
        assert_eq!(frame("center-three-fourths", window, WORK), Some(r(240, 0, 1680, 1032)));
        assert_eq!(frame("second-fourth", window, WORK), Some(r(480, 0, 960, 1032)));
        assert_eq!(frame("top-third-fourth", window, WORK), Some(r(0, 516, 1920, 774)));
        assert_eq!(frame("bottom-right-quarter", window, WORK), Some(r(960, 516, 1920, 1032)));
        assert_eq!(frame("bottom-center-sixth", window, WORK), Some(r(640, 516, 1280, 1032)));
        assert_eq!(frame("top-center-two-thirds", window, WORK), Some(r(320, 0, 1600, 516)));
    }

    #[test]
    fn relative_layouts_keep_the_window_on_screen() {
        let window = r(100, 100, 900, 700);
        assert_eq!(frame("almost-maximize", window, WORK), Some(r(96, 51, 1824, 980)));
        assert_eq!(frame("reasonable-size", window, WORK), Some(r(448, 206, 1472, 825)));
        assert_eq!(frame("maximize-height", window, WORK), Some(r(100, 0, 900, 1032)));
        assert_eq!(frame("maximize-width", window, WORK), Some(r(0, 100, 1920, 700)));
        assert_eq!(frame("move-right", window, WORK), Some(r(1120, 216, 1920, 816)));
        assert_eq!(frame("move-top", window, WORK), Some(r(560, 0, 1360, 600)));
        assert_eq!(frame("make-larger", window, WORK), Some(r(84, 84, 916, 716)));
        assert_eq!(frame("make-smaller", window, WORK), Some(r(116, 116, 884, 684)));
        assert_eq!(frame("make-smaller", r(0, 0, 480, 258), WORK), Some(r(0, 0, 480, 258)));
        assert_eq!(frame("make-larger", r(0, 0, 1920, 1032), WORK), Some(WORK));
    }

    #[test]
    fn center_keeps_the_size_and_clamps_to_the_screen() {
        assert_eq!(frame("center", r(0, 0, 800, 600), WORK), Some(r(560, 216, 1360, 816)));
        assert_eq!(frame("center", r(0, 0, 3000, 600), WORK), Some(r(0, 216, 1920, 816)));
    }

    #[test]
    fn moving_to_another_display_scales_and_keeps_the_aspect_ratio() {
        let small = r(1920, 0, 3200, 1000);
        assert_eq!(move_between(r(0, 0, 960, 1032), WORK, small), r(1920, 0, 2560, 688));
        assert_eq!(move_between(r(960, 0, 1920, 1032), WORK, small), r(2560, 0, 3200, 688));
    }

    #[test]
    #[ignore = "opens Notepad and moves it on the real desktop"]
    fn moves_a_real_window() {
        use windows::Win32::Foundation::{LPARAM, WPARAM};
        use windows::core::w;
        use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, PostMessageW, WM_CLOSE};
        if unsafe { FindWindowW(w!("Notepad"), None) }.is_err() {
            std::process::Command::new("notepad.exe").spawn().unwrap();
            std::thread::sleep(std::time::Duration::from_secs(3));
        }
        let hwnd = unsafe { FindWindowW(w!("Notepad"), None) }.unwrap();
        let work = display_info(unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) }).unwrap().work;
        for layout in ["left-half", "last-two-thirds", "bottom-third", "maximize", "center"] {
            apply_to(hwnd, layout).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(300));
            let (_, visible) = outer_and_visible(hwnd).unwrap();
            assert_eq!(Some(visible), frame(layout, visible, work), "{layout}");
        }
        unsafe {
            let _ = PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0));
        }
    }

    #[test]
    #[ignore = "opens Notepad and moves it on the real desktop"]
    fn restores_and_pins_a_real_window() {
        use windows::Win32::Foundation::{LPARAM, WPARAM};
        use windows::core::w;
        use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, PostMessageW, WM_CLOSE};
        if unsafe { FindWindowW(w!("Notepad"), None) }.is_err() {
            std::process::Command::new("notepad.exe").spawn().unwrap();
            std::thread::sleep(std::time::Duration::from_secs(3));
        }
        let hwnd = unsafe { FindWindowW(w!("Notepad"), None) }.unwrap();
        let work = display_info(unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) }).unwrap().work;
        apply_to(hwnd, "center").unwrap();
        let (_, before) = outer_and_visible(hwnd).unwrap();
        for layout in ["top-right-quarter", "bottom-center-sixth", "almost-maximize"] {
            apply_to(hwnd, layout).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(300));
            let (_, visible) = outer_and_visible(hwnd).unwrap();
            assert_eq!(Some(visible), frame(layout, before, work), "{layout}");
        }
        apply_to(hwnd, "restore").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(300));
        assert_eq!(outer_and_visible(hwnd).unwrap().1, frame("bottom-center-sixth", before, work).unwrap());
        let topmost = || unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) } as u32 & WS_EX_TOPMOST.0 != 0;
        apply_to(hwnd, "toggle-always-on-top").unwrap();
        assert!(topmost());
        apply_to(hwnd, "toggle-always-on-top").unwrap();
        assert!(!topmost());
        unsafe {
            let _ = PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0));
        }
    }

    #[test]
    fn display_order_runs_left_to_right_and_wraps() {
        let displays = [r(1920, 0, 3840, 1080), r(-1280, 0, 0, 1024), r(0, 0, 1920, 1080)];
        assert_eq!(neighbour(&displays, displays[2], 1), Some(0));
        assert_eq!(neighbour(&displays, displays[0], 1), Some(1));
        assert_eq!(neighbour(&displays, displays[1], -1), Some(0));
    }
}
