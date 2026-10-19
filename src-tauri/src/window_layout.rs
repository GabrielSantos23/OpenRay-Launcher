use crate::launcher;
use windows::core::BOOL;
use windows::Win32::Foundation::{HWND, LPARAM, RECT};
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_EXTENDED_FRAME_BOUNDS};
use windows::Win32::Graphics::Gdi::{EnumDisplayMonitors, GetMonitorInfoW, MonitorFromWindow, HDC, HMONITOR, MONITORINFO, MONITOR_DEFAULTTONEAREST};
use windows::Win32::UI::WindowsAndMessaging::{
    GetAncestor, GetForegroundWindow, GetShellWindow, GetWindowRect, IsIconic, IsWindow, IsWindowVisible, IsZoomed, SetWindowPos, ShowWindow,
    GA_ROOT, SWP_NOACTIVATE, SWP_NOZORDER, SW_MAXIMIZE, SW_RESTORE,
};

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
        _ => return None,
    })
}

pub fn frame(layout: &str, window: Rect, work: Rect) -> Option<Rect> {
    if layout == "center" {
        let width = window.width().min(work.width());
        let height = window.height().min(work.height());
        let left = work.left + (work.width() - width) / 2;
        let top = work.top + (work.height() - height) / 2;
        return Some(Rect { left, top, right: left + width, bottom: top + height });
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
    let hwnd = if launcher::is_shown() { launcher::previous_window() } else { unsafe { GetAncestor(GetForegroundWindow(), GA_ROOT) } };
    let usable = !hwnd.is_invalid() && unsafe { IsWindow(Some(hwnd)).as_bool() && IsWindowVisible(hwnd).as_bool() && hwnd != GetShellWindow() };
    if usable {
        Ok(hwnd)
    } else {
        Err("No active window".into())
    }
}

fn outer_and_visible(hwnd: HWND) -> Result<(Rect, Rect), String> {
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

pub fn apply(layout: &str) -> Result<(), String> {
    apply_to(target()?, layout)
}

fn apply_to(hwnd: HWND, layout: &str) -> Result<(), String> {
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
    fn display_order_runs_left_to_right_and_wraps() {
        let displays = [r(1920, 0, 3840, 1080), r(-1280, 0, 0, 1024), r(0, 0, 1920, 1080)];
        assert_eq!(neighbour(&displays, displays[2], 1), Some(0));
        assert_eq!(neighbour(&displays, displays[0], 1), Some(1));
        assert_eq!(neighbour(&displays, displays[1], -1), Some(0));
    }
}
