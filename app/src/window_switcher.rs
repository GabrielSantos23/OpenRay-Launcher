use crate::{clipboard, launcher, system_actions};
use serde::Serialize;
use std::time::{Duration, Instant};
use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM, WPARAM};
use windows::Win32::System::Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumChildWindows, GetWindowTextW, GetWindowThreadProcessId, IsIconic, IsWindow, IsZoomed, PostMessageW, ShowWindow, SW_MAXIMIZE, SW_MINIMIZE, SW_RESTORE, WM_CLOSE,
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowEntry {
    pub id: isize,
    pub title: String,
    pub app_name: Option<String>,
    pub path: Option<String>,
    pub minimized: bool,
    pub maximized: bool,
}

pub(crate) fn app_pid(hwnd: HWND) -> u32 {
    let pid = system_actions::process_of(hwnd);
    let hosted = clipboard::process_app(pid).is_some_and(|(_, path)| path.to_lowercase().ends_with("\\applicationframehost.exe"));
    if hosted {
        hosted_pid(hwnd, pid)
    } else {
        pid
    }
}

fn title(hwnd: HWND) -> String {
    let mut buffer = [0u16; 512];
    let len = unsafe { GetWindowTextW(hwnd, &mut buffer) } as usize;
    String::from_utf16_lossy(&buffer[..len])
}

pub fn list() -> Vec<WindowEntry> {
    system_actions::app_windows()
        .into_iter()
        .map(|hwnd| {
            let app = clipboard::process_app(app_pid(hwnd));
            WindowEntry {
                id: hwnd.0 as isize,
                title: title(hwnd),
                app_name: app.as_ref().map(|(name, _)| name.clone()),
                path: app.map(|(_, path)| path),
                minimized: unsafe { IsIconic(hwnd) }.as_bool(),
                maximized: unsafe { IsZoomed(hwnd) }.as_bool(),
            }
        })
        .collect()
}

fn close(hwnd: HWND) {
    unsafe {
        let _ = PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0));
    }
}

pub(crate) fn hosted_pid(hwnd: HWND, host_pid: u32) -> u32 {
    unsafe extern "system" fn find(child: HWND, data: LPARAM) -> windows::core::BOOL {
        let (host, found) = unsafe { &mut *(data.0 as *mut (u32, u32)) };
        let mut pid = 0;
        unsafe { GetWindowThreadProcessId(child, Some(&mut pid)) };
        if pid != *host {
            *found = pid;
            return false.into();
        }
        true.into()
    }
    let mut data = (host_pid, host_pid);
    unsafe {
        let _ = EnumChildWindows(Some(hwnd), Some(find), LPARAM(&mut data as *mut _ as isize));
    }
    data.1
}

fn wait_closed(windows: &[HWND]) {
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(1) && windows.iter().any(|&hwnd| unsafe { IsWindow(Some(hwnd)) }.as_bool()) {
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn activate(hwnd: HWND, show: windows::Win32::UI::WindowsAndMessaging::SHOW_WINDOW_CMD) {
    unsafe {
        let _ = ShowWindow(hwnd, show);
    }
    launcher::force_foreground(hwnd);
}

pub fn act(hide: &dyn Fn(bool), id: isize, action: &str) -> Result<(), String> {
    let hwnd = HWND(id as _);
    if !unsafe { IsWindow(Some(hwnd)) }.as_bool() {
        return Err("The window is no longer open".into());
    }
    match action {
        "switch" => {
            hide(false);
            if unsafe { IsIconic(hwnd) }.as_bool() {
                activate(hwnd, SW_RESTORE);
            } else {
                launcher::force_foreground(hwnd);
            }
        }
        "minimize" => {
            hide(true);
            unsafe {
                let _ = ShowWindow(hwnd, SW_MINIMIZE);
            }
        }
        "close" => {
            hide(true);
            close(hwnd);
        }
        "toggle-maximize" => {
            hide(false);
            activate(hwnd, if unsafe { IsZoomed(hwnd) }.as_bool() { SW_RESTORE } else { SW_MAXIMIZE });
        }
        "quit" => {
            let pid = app_pid(hwnd);
            let windows: Vec<HWND> = system_actions::app_windows().into_iter().filter(|&w| app_pid(w) == pid).collect();
            windows.iter().copied().for_each(close);
            wait_closed(&windows);
        }
        "force-quit" => unsafe {
            let process = OpenProcess(PROCESS_TERMINATE, false, app_pid(hwnd)).map_err(|e| e.message())?;
            let result = TerminateProcess(process, 1);
            let _ = CloseHandle(process);
            result.map_err(|e| e.message())?;
            wait_closed(&[hwnd]);
        },
        _ => return Err(format!("Unknown window action: {action}")),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "reads the live desktop"]
    fn lists_titled_windows_from_other_processes() {
        let windows = list();
        assert!(windows.iter().all(|w| !w.title.is_empty()));
        assert!(windows.iter().all(|w| w.path.as_deref().is_none_or(|p| !p.to_lowercase().ends_with("\\applicationframehost.exe"))));
        for w in &windows {
            println!("{} | {:?} | min={} max={}", w.title, w.app_name, w.minimized, w.maximized);
        }
    }
}
