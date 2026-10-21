use crate::launcher::{hwnd, make_popup, set_dwm};
use crate::store::Settings;
use base64::Engine;
use serde::Serialize;
use serde_json::Value;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, WebviewWindow};
use windows::Media::Control::GlobalSystemMediaTransportControlsSessionManager as MediaManager;
use windows::Storage::Streams::DataReader;
use windows::Win32::Foundation::{POINT, RECT};
use windows::Win32::Graphics::Dwm::{DWMWA_BORDER_COLOR, DWMWA_TRANSITIONS_FORCEDISABLED, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DONOTROUND};
use windows::Win32::Graphics::Gdi::{CreateRoundRectRgn, GetMonitorInfoW, MonitorFromWindow, SetWindowRgn, MONITORINFO, MONITOR_DEFAULTTONEAREST};
use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
use windows::Win32::UI::WindowsAndMessaging::{
    GetClassNameW, GetCursorPos, GetForegroundWindow, GetShellWindow, GetWindowLongPtrW, GetWindowRect, SetWindowLongPtrW, SetWindowPos, ShowWindow, GWL_EXSTYLE,
    HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SW_HIDE, SW_SHOWNOACTIVATE, WS_EX_APPWINDOW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
};

pub const LABEL: &str = "notch";
const DWMWA_COLOR_NONE: u32 = 0xFFFF_FFFE;
const TICK: Duration = Duration::from_millis(100);
const FULLSCREEN_EVERY: Duration = Duration::from_millis(500);
const AUDIO_REFRESH_EVERY: Duration = Duration::from_secs(5);

static WATCHING: AtomicBool = AtomicBool::new(false);
static SHOW_IN_FULLSCREEN: AtomicBool = AtomicBool::new(false);
static ART: Mutex<Option<(String, Option<String>)>> = Mutex::new(None);

pub fn apply(app: &AppHandle, settings: &Settings) {
    SHOW_IN_FULLSCREEN.store(settings.notch_show_in_fullscreen, Ordering::SeqCst);
    match (settings.notch_enabled, app.get_webview_window(LABEL)) {
        (true, None) => crate::open_window(app, LABEL, |app, window| {
            init(&window);
            watch(app);
        }),
        (false, Some(window)) => {
            let _ = window.destroy();
        }
        _ => {}
    }
}

fn init(window: &WebviewWindow) {
    let Some(hwnd) = hwnd(window) else { return };
    make_popup(hwnd, 1, 1);
    set_dwm(hwnd, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DONOTROUND);
    set_dwm(hwnd, DWMWA_BORDER_COLOR, DWMWA_COLOR_NONE);
    set_dwm(hwnd, DWMWA_TRANSITIONS_FORCEDISABLED, 1i32);
    unsafe {
        let ex = (GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32 & !WS_EX_APPWINDOW.0) | (WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TOPMOST).0;
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex as isize);
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
    }
}

pub fn resize(app: &AppHandle, width: f64, height: f64, radius: f64) {
    let Some(window) = app.get_webview_window(LABEL) else { return };
    let (Some(hwnd), Ok(Some(monitor))) = (hwnd(&window), window.primary_monitor()) else { return };
    let scale = monitor.scale_factor();
    let (w, h, r) = ((width * scale).round() as i32, (height * scale).round() as i32, (radius * scale).round() as i32);
    let x = monitor.position().x + (monitor.size().width as i32 - w) / 2;
    unsafe {
        let _ = SetWindowPos(hwnd, Some(HWND_TOPMOST), x, monitor.position().y, w, h, SWP_NOACTIVATE);
        let region = CreateRoundRectRgn(0, -r, w + 1, h + 1, r * 2, r * 2);
        SetWindowRgn(hwnd, Some(region), true);
    }
}

pub fn covers(window: (i32, i32, i32, i32), monitor: (i32, i32, i32, i32)) -> bool {
    window.0 <= monitor.0 && window.1 <= monitor.1 && window.2 >= monitor.2 && window.3 >= monitor.3
}

fn fullscreen_on_primary() -> bool {
    unsafe {
        let foreground = GetForegroundWindow();
        if foreground.is_invalid() || foreground == GetShellWindow() {
            return false;
        }
        let mut class = [0u16; 32];
        let len = GetClassNameW(foreground, &mut class) as usize;
        if matches!(String::from_utf16_lossy(&class[..len]).as_str(), "WorkerW" | "Progman") {
            return false;
        }
        let mut info = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
        let mut rect = RECT::default();
        if !GetMonitorInfoW(MonitorFromWindow(foreground, MONITOR_DEFAULTTONEAREST), &mut info).as_bool() || info.dwFlags & 1 == 0 || GetWindowRect(foreground, &mut rect).is_err() {
            return false;
        }
        let m = info.rcMonitor;
        covers((rect.left, rect.top, rect.right, rect.bottom), (m.left, m.top, m.right, m.bottom))
    }
}

fn cursor_inside(window: &WebviewWindow) -> bool {
    let Some(hwnd) = hwnd(window) else { return false };
    let mut cursor = POINT::default();
    let mut rect = RECT::default();
    unsafe { GetCursorPos(&mut cursor).is_ok() && GetWindowRect(hwnd, &mut rect).is_ok() && cursor.x >= rect.left && cursor.x < rect.right && cursor.y >= rect.top && cursor.y < rect.bottom }
}

fn watch(app: &AppHandle) {
    if WATCHING.swap(true, Ordering::SeqCst) {
        return;
    }
    let app = app.clone();
    std::thread::spawn(move || {
        let (mut hovered, mut hidden) = (false, false);
        let mut last_fullscreen_check = Instant::now();
        let mut audio = None;
        let mut audio_at = Instant::now() - AUDIO_REFRESH_EVERY;
        let mut volume: Option<(u32, bool)> = None;
        while let Some(window) = app.get_webview_window(LABEL) {
            let inside = !hidden && cursor_inside(&window);
            if inside != hovered {
                hovered = inside;
                let _ = app.emit_to(LABEL, "notch-hover", hovered);
            }
            if last_fullscreen_check.elapsed() >= FULLSCREEN_EVERY {
                last_fullscreen_check = Instant::now();
                let hide = !SHOW_IN_FULLSCREEN.load(Ordering::SeqCst) && fullscreen_on_primary();
                if hide != hidden {
                    hidden = hide;
                    if let Some(hwnd) = hwnd(&window) {
                        unsafe {
                            let _ = ShowWindow(hwnd, if hide { SW_HIDE } else { SW_SHOWNOACTIVATE });
                            if !hide {
                                let _ = SetWindowPos(hwnd, Some(HWND_TOPMOST), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
                            }
                        }
                    }
                }
            }
            if audio_at.elapsed() >= AUDIO_REFRESH_EVERY {
                audio_at = Instant::now();
                audio = crate::system_actions::speaker_volume().ok();
            }
            let level = audio.as_ref().and_then(|a| unsafe { Some(((a.GetMasterVolumeLevelScalar().ok()? * 100.0).round() as u32, a.GetMute().ok()?.as_bool())) });
            if level.is_some() && volume.is_some() && level != volume && !hidden {
                let (percent, muted) = level.unwrap_or_default();
                let _ = app.emit_to(LABEL, "notch-volume", serde_json::json!({ "percent": percent, "muted": muted }));
            }
            if level.is_some() {
                volume = level;
            }
            std::thread::sleep(TICK);
        }
        WATCHING.store(false, Ordering::SeqCst);
    });
}

fn source_name(app_id: &str) -> String {
    let id = app_id.rsplit(['\\', '/']).next().unwrap_or(app_id);
    let id = id.split('!').next_back().unwrap_or(id);
    let id = id.strip_suffix(".exe").or_else(|| id.strip_suffix(".EXE")).unwrap_or(id);
    let id = id.split('_').next().unwrap_or(id);
    id.rsplit('.').next().unwrap_or(id).to_string()
}

fn artwork(session: &windows::Media::Control::GlobalSystemMediaTransportControlsSession) -> windows::core::Result<Option<String>> {
    let Ok(thumbnail) = session.TryGetMediaPropertiesAsync()?.get()?.Thumbnail() else { return Ok(None) };
    let stream = thumbnail.OpenReadAsync()?.get()?;
    let size = stream.Size()? as u32;
    if size == 0 {
        return Ok(None);
    }
    let reader = DataReader::CreateDataReader(&stream)?;
    reader.LoadAsync(size)?.get()?;
    let mut bytes = vec![0u8; size as usize];
    reader.ReadBytes(&mut bytes)?;
    let mime = stream.ContentType().map(|t| t.to_string()).ok().filter(|t| t.starts_with("image/")).unwrap_or_else(|| "image/png".into());
    Ok(Some(format!("data:{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes))))
}

fn media() -> Value {
    let Ok(mut value) = crate::flowkey::media::current() else { return serde_json::json!({ "playing": false }) };
    let session = MediaManager::RequestAsync().and_then(|op| op.get()).and_then(|m| m.GetCurrentSession());
    if let (Ok(session), Some(object)) = (session, value.as_object_mut()) {
        let app_id = session.SourceAppUserModelId().map(|s| s.to_string()).unwrap_or_default();
        object.insert("source".into(), source_name(&app_id).into());
        let key = format!("{app_id}|{}|{}", object.get("title").and_then(Value::as_str).unwrap_or(""), object.get("artist").and_then(Value::as_str).unwrap_or(""));
        let mut cache = ART.lock().unwrap_or_else(|e| e.into_inner());
        if cache.as_ref().is_none_or(|(k, art)| *k != key || art.is_none()) {
            *cache = Some((key, artwork(&session).ok().flatten()));
        }
        object.insert("artwork".into(), cache.as_ref().and_then(|(_, art)| art.clone()).into());
    }
    value
}

#[derive(Serialize)]
pub struct Battery {
    percent: u8,
    charging: bool,
}

fn battery() -> Option<Battery> {
    let mut status = SYSTEM_POWER_STATUS::default();
    unsafe { GetSystemPowerStatus(&mut status) }.ok()?;
    (status.BatteryFlag & 128 == 0 && status.BatteryLifePercent <= 100).then(|| Battery { percent: status.BatteryLifePercent, charging: status.ACLineStatus == 1 })
}

#[tauri::command]
pub fn notch_resize(app: AppHandle, width: f64, height: f64, radius: f64) {
    resize(&app, width, height, radius)
}

#[tauri::command]
pub async fn notch_media() -> Value {
    tauri::async_runtime::spawn_blocking(media).await.unwrap_or_default()
}

#[tauri::command]
pub fn notch_battery() -> Option<Battery> {
    battery()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_windows_covering_the_whole_monitor_count_as_fullscreen() {
        let monitor = (0, 0, 1920, 1080);
        assert!(covers((0, 0, 1920, 1080), monitor));
        assert!(covers((-8, -8, 1928, 1088), monitor));
        assert!(!covers((0, 0, 1920, 1040), monitor));
        assert!(!covers((1920, 0, 3840, 1080), monitor));
    }

    #[test]
    fn source_names_come_from_app_ids() {
        assert_eq!(source_name("Spotify.exe"), "Spotify");
        assert_eq!(source_name("SpotifyAB.SpotifyMusic_zpdnekdrzrea0!Spotify"), "Spotify");
        assert_eq!(source_name("Microsoft.ZuneMusic_8wekyb3d8bbwe!Microsoft.ZuneMusic"), "ZuneMusic");
        assert_eq!(source_name("C:\\Program Files\\VideoLAN\\VLC\\vlc.exe"), "vlc");
        assert_eq!(source_name("MSEdge"), "MSEdge");
    }
}
