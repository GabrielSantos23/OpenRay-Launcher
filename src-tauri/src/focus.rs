use crate::launcher::{self, hwnd};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager, WebviewWindow};
use windows::core::{w, BSTR, PCWSTR, PWSTR};
use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM, POINT, WPARAM};
use windows::Win32::Graphics::Dwm::{DWMWA_BORDER_COLOR, DWMWA_TRANSITIONS_FORCEDISABLED, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND};
use windows::Win32::Media::Audio::{PlaySoundW, SND_ALIAS, SND_ASYNC, SND_NODEFAULT};
use windows::Win32::Storage::Packaging::Appx::GetPackageFamilyName;
use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, IPersistFile, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, COINIT_MULTITHREADED, STGM_READ};
use windows::Win32::System::Threading::{OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION};
use windows::Win32::System::Variant::VARIANT;
use windows::Win32::UI::Accessibility::{
    CUIAutomation, IUIAutomation, IUIAutomationElement, TreeScope_Descendants, UIA_ControlTypePropertyId, UIA_EditControlTypeId, UIA_ValueValuePropertyId,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VIRTUAL_KEY, VK_CONTROL, VK_W};
use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, DestroyMenu, EnumChildWindows, EnumWindows, GetAncestor, GetCursorPos, GetForegroundWindow, GetWindowLongPtrW, GetWindowRect,
    GetWindowThreadProcessId, IsWindowVisible, PostMessageW, SetForegroundWindow, SetWindowLongPtrW, SetWindowPos, ShowWindow, TrackPopupMenu,
    GA_ROOT, GWL_EXSTYLE, HWND_TOPMOST, MF_SEPARATOR, MF_STRING, SWP_NOACTIVATE, SWP_NOSIZE, SW_HIDE, SW_SHOWNOACTIVATE, TPM_BOTTOMALIGN,
    TPM_LEFTALIGN, TPM_RETURNCMD, WM_CLOSE, WS_EX_APPWINDOW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
};

pub const PANEL: &str = "focus-panel";
pub const TOAST: &str = "focus-toast";
pub const BLOCK: &str = "focus-block";
const PANEL_BOTTOM_MARGIN: f64 = 24.0;
const TOAST_TOP_MARGIN: f64 = 24.0;
const TICK: Duration = Duration::from_millis(300);
const PANEL_SIZE: (f64, f64) = (265.0, 40.0);
const TOAST_SIZE: (f64, f64) = (392.0, 66.0);
const BROWSERS: [&str; 14] = [
    "chrome.exe", "msedge.exe", "brave.exe", "firefox.exe", "opera.exe", "vivaldi.exe", "arc.exe", "zen.exe", "librewolf.exe", "waterfox.exe",
    "thorium.exe", "floorp.exe", "chromium.exe", "helium.exe",
];
const ALWAYS_ALLOWED: [&str; 10] = [
    "explorer.exe", "openray.exe", "searchhost.exe", "startmenuexperiencehost.exe", "shellexperiencehost.exe", "lockapp.exe", "taskmgr.exe",
    "systemsettings.exe", "applicationframehost.exe", "textinputhost.exe",
];

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Block,
    Allow,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Running,
    Paused,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct StartInput {
    pub title: String,
    pub mode: Mode,
    pub duration_ms: Option<u64>,
    pub apps: Vec<String>,
    pub websites: Vec<String>,
    pub allow_all_websites: bool,
    pub snooze_ms: Option<u64>,
    pub play_sound: bool,
    pub restore_apps: bool,
    pub form: serde_json::Value,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub id: u64,
    pub title: String,
    pub mode: Mode,
    pub status: Status,
    pub started_at: u64,
    pub duration_ms: Option<u64>,
    pub end_at: Option<u64>,
    pub remaining_ms: u64,
    pub pause_ends_at: Option<u64>,
    pub form: serde_json::Value,
    #[serde(skip)]
    rules: Rules,
    #[serde(skip)]
    snooze_ms: Option<u64>,
    #[serde(skip)]
    play_sound: bool,
    #[serde(skip)]
    restore_apps: bool,
}

#[derive(Clone, Debug, Default)]
pub struct Rules {
    pub allow: bool,
    pub apps: Vec<String>,
    pub websites: Vec<String>,
    pub allow_all_websites: bool,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
struct BlockInfo {
    key: String,
    name: String,
    website: bool,
    snooze_ms: Option<u64>,
    #[serde(skip)]
    target: isize,
    #[serde(skip)]
    pid: u32,
    #[serde(skip)]
    exe: String,
}

#[derive(Default)]
struct State {
    session: Option<Session>,
    snoozed: HashMap<String, u64>,
    block: Option<BlockInfo>,
    terminated: Vec<String>,
    panel_position: Option<(i32, i32)>,
    next_id: u64,
}

static STATE: Mutex<Option<State>> = Mutex::new(None);
static WORKER: AtomicBool = AtomicBool::new(false);
static MENU_OPEN: AtomicBool = AtomicBool::new(false);
static HOVER_WATCH: AtomicBool = AtomicBool::new(false);

fn with_state<T>(f: impl FnOnce(&mut State) -> T) -> T {
    let mut guard = STATE.lock().unwrap_or_else(|e| e.into_inner());
    f(guard.get_or_insert_with(State::default))
}

pub fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

pub fn app_id(raw: &str) -> String {
    let raw = raw.trim();
    let id = raw.strip_prefix(r"shell:AppsFolder\").map(|aumid| aumid.split('!').next().unwrap_or(aumid)).unwrap_or(raw);
    id.rsplit(['\\', '/']).next().unwrap_or(id).to_lowercase()
}

pub fn host_of(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() || value.contains(char::is_whitespace) {
        return None;
    }
    let rest = value.split_once("://").map(|(_, r)| r).unwrap_or(value);
    let host = rest.split(['/', '?', '#']).next()?.rsplit('@').next()?.split(':').next()?.to_lowercase();
    let host = host.strip_prefix("www.").unwrap_or(&host).to_string();
    (host.contains('.') && !host.starts_with('.') && !host.ends_with('.')).then_some(host)
}

pub fn host_matches(pattern: &str, host: &str) -> bool {
    let pattern = pattern.trim().to_lowercase();
    let pattern = pattern.strip_prefix("www.").unwrap_or(&pattern);
    if let Some(base) = pattern.strip_suffix(".*") {
        let labels: Vec<&str> = host.split('.').collect();
        let base: Vec<&str> = base.split('.').collect();
        return (0..labels.len()).any(|i| labels[i..].starts_with(&base) && labels.len() > i + base.len());
    }
    host == pattern || host.ends_with(&format!(".{pattern}"))
}

pub fn app_blocked(rules: &Rules, candidates: &[String]) -> bool {
    let listed = candidates.iter().any(|c| rules.apps.iter().any(|a| a == c));
    if rules.allow {
        !listed && !candidates.iter().any(|c| ALWAYS_ALLOWED.contains(&c.as_str()))
    } else {
        listed
    }
}

pub fn website_blocked(rules: &Rules, host: &str) -> bool {
    let listed = rules.websites.iter().any(|p| host_matches(p, host));
    if rules.allow {
        !listed && !(rules.websites.is_empty() && rules.allow_all_websites)
    } else {
        listed
    }
}

fn emit(app: &AppHandle) {
    let session = with_state(|s| s.session.clone());
    let _ = app.emit("focus-session-changed", session);
}

fn rules_of(input: &StartInput) -> Rules {
    Rules {
        allow: input.mode == Mode::Allow,
        apps: input.apps.iter().map(|a| resolve_app(a)).collect(),
        websites: input.websites.iter().filter_map(|w| if w.contains('*') { Some(w.to_lowercase()) } else { host_of(w) }).collect(),
        allow_all_websites: input.allow_all_websites,
    }
}

fn apply_input(session: &mut Session, input: StartInput, now: u64) {
    session.rules = rules_of(&input);
    session.title = if input.title.trim().is_empty() { "Focus session".into() } else { input.title.trim().into() };
    session.mode = input.mode;
    session.duration_ms = input.duration_ms;
    session.snooze_ms = input.snooze_ms;
    session.play_sound = input.play_sound;
    session.restore_apps = input.restore_apps;
    session.form = input.form;
    session.end_at = input.duration_ms.map(|d| now + d);
    session.remaining_ms = input.duration_ms.unwrap_or(0);
}

pub fn get() -> Option<Session> {
    with_state(|s| s.session.clone())
}

pub fn start(app: &AppHandle, input: StartInput) -> Session {
    let now = now_ms();
    let session = with_state(|s| {
        s.next_id += 1;
        let mut session = Session {
            id: s.next_id,
            title: String::new(),
            mode: Mode::Block,
            status: Status::Running,
            started_at: now,
            duration_ms: None,
            end_at: None,
            remaining_ms: 0,
            pause_ends_at: None,
            form: serde_json::Value::Null,
            rules: Rules::default(),
            snooze_ms: None,
            play_sound: true,
            restore_apps: false,
        };
        apply_input(&mut session, input, now);
        s.session = Some(session.clone());
        s.snoozed.clear();
        s.terminated.clear();
        session
    });
    ensure_worker(app);
    show_panel(app);
    emit(app);
    session
}

pub fn update(app: &AppHandle, input: StartInput) -> Result<Session, String> {
    let now = now_ms();
    let session = with_state(|s| {
        let session = s.session.as_mut()?;
        apply_input(session, input, now);
        if session.status == Status::Paused {
            session.end_at = None;
        }
        Some(session.clone())
    })
    .ok_or("No focus session is running")?;
    emit(app);
    Ok(session)
}

pub fn pause(app: &AppHandle, duration_ms: Option<u64>) -> Result<(), String> {
    let now = now_ms();
    with_state(|s| {
        let session = s.session.as_mut().ok_or("No focus session is running")?;
        if session.status == Status::Running {
            session.remaining_ms = session.end_at.map(|end| end.saturating_sub(now)).unwrap_or(0);
            session.end_at = None;
        }
        session.status = Status::Paused;
        session.pause_ends_at = duration_ms.map(|d| now + d);
        Ok::<_, String>(())
    })?;
    hide_block(app);
    emit(app);
    Ok(())
}

pub fn resume(app: &AppHandle) -> Result<(), String> {
    let now = now_ms();
    with_state(|s| {
        let session = s.session.as_mut().ok_or("No focus session is running")?;
        if session.status == Status::Paused {
            session.status = Status::Running;
            session.pause_ends_at = None;
            session.end_at = session.duration_ms.map(|_| now + session.remaining_ms);
        }
        Ok::<_, String>(())
    })?;
    emit(app);
    Ok(())
}

pub fn restart(app: &AppHandle) -> Result<(), String> {
    let now = now_ms();
    with_state(|s| {
        let session = s.session.as_mut().ok_or("No focus session is running")?;
        session.status = Status::Running;
        session.started_at = now;
        session.pause_ends_at = None;
        session.end_at = session.duration_ms.map(|d| now + d);
        session.remaining_ms = session.duration_ms.unwrap_or(0);
        Ok::<_, String>(())
    })?;
    emit(app);
    Ok(())
}

fn finish(app: &AppHandle, completed: bool) -> Result<(), String> {
    let (session, terminated) = with_state(|s| (s.session.take(), std::mem::take(&mut s.terminated)));
    let session = session.ok_or("No focus session is running")?;
    with_state(|s| s.snoozed.clear());
    hide_block(app);
    if let Some(panel) = app.get_webview_window(PANEL) {
        let _ = panel.destroy();
    }
    if session.restore_apps {
        for exe in terminated {
            let _ = std::process::Command::new(exe).spawn();
        }
    }
    if completed {
        show_toast(app, serde_json::json!({ "title": "Focus session completed", "message": "You stayed focused." }), session.play_sound);
    }
    emit(app);
    Ok(())
}

pub fn complete(app: &AppHandle) -> Result<(), String> {
    finish(app, true)
}

pub fn cancel(app: &AppHandle) -> Result<(), String> {
    finish(app, false)
}

fn play_sound() {
    unsafe {
        let _ = PlaySoundW(w!("Notification.Default"), None, SND_ALIAS | SND_ASYNC | SND_NODEFAULT);
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
    let saved = if reset { None } else { with_state(|s| s.panel_position) };
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
    with_state(|s| s.panel_position = Some((x, y)));
    unsafe {
        let _ = SetWindowPos(hwnd, Some(HWND_TOPMOST), x, y, 0, 0, SWP_NOSIZE | SWP_NOACTIVATE);
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
    }
}

fn show_panel(app: &AppHandle) {
    crate::open_window(app, PANEL, |app, window| {
        popup_styles(&window, false, PANEL_SIZE);
        place_panel(app, &window, false);
        watch_hover(app);
    });
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
                let mut rect = windows::Win32::Foundation::RECT::default();
                GetCursorPos(&mut cursor).is_ok()
                    && GetWindowRect(panel, &mut rect).is_ok()
                    && cursor.x >= rect.left
                    && cursor.x < rect.right
                    && cursor.y >= rect.top
                    && cursor.y < rect.bottom
            });
            let hovered = inside || MENU_OPEN.load(Ordering::SeqCst);
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
        with_state(|s| s.panel_position = Some((position.x, position.y)));
    }
}

fn show_toast(app: &AppHandle, payload: serde_json::Value, sound: bool) {
    if sound {
        play_sound();
    }
    crate::open_window(app, TOAST, move |app, window| {
        popup_styles(&window, false, TOAST_SIZE);
        if let (Some(hwnd), Some((origin, size, scale)), Ok(outer)) = (hwnd(&window), work_area(app, &window), window.outer_size()) {
            let x = origin.x + (size.width as i32 - outer.width as i32) / 2;
            let y = origin.y + (TOAST_TOP_MARGIN * scale).round() as i32;
            unsafe {
                let _ = SetWindowPos(hwnd, Some(HWND_TOPMOST), x, y, 0, 0, SWP_NOSIZE | SWP_NOACTIVATE);
                let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            }
        }
        let _ = app.emit_to(TOAST, "focus-toast", payload.clone());
        let handle = app.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(400));
            let _ = handle.emit_to(TOAST, "focus-toast", payload);
        });
    });
}

pub fn hide_toast(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(TOAST) {
        let _ = window.destroy();
    }
}

fn hide_block(app: &AppHandle) {
    let had = with_state(|s| s.block.take()).is_some();
    if let Some(window) = app.get_webview_window(BLOCK) {
        if let Some(hwnd) = hwnd(&window) {
            unsafe {
                let _ = ShowWindow(hwnd, SW_HIDE);
            }
        }
    }
    if had {
        hide_toast(app);
    }
}

fn show_block(app: &AppHandle, info: BlockInfo) {
    let target = HWND(info.target as _);
    let Ok((_, visible)) = crate::window_layout::outer_and_visible(target) else { return };
    with_state(|s| s.block = Some(info.clone()));
    let snooze = info.snooze_ms.map(|ms| ms / 60_000);
    show_toast(
        app,
        serde_json::json!({
            "title": format!("{} is blocked", info.name),
            "message": format!("Openray Focus is blocking {}", info.name),
            "snooze": snooze.is_some(),
        }),
        with_state(|s| s.session.as_ref().is_some_and(|session| session.play_sound)),
    );
    crate::open_window(app, BLOCK, move |app, window| {
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

pub fn current_block() -> Option<serde_json::Value> {
    with_state(|s| s.block.clone()).and_then(|b| serde_json::to_value(b).ok())
}

pub fn snooze(app: &AppHandle) -> Result<(), String> {
    let (block, ms) = with_state(|s| (s.block.clone(), s.session.as_ref().and_then(|session| session.snooze_ms)));
    let block = block.ok_or("Nothing is blocked")?;
    let ms = ms.ok_or("Snooze is off")?;
    with_state(|s| s.snoozed.insert(block.key.clone(), now_ms() + ms));
    hide_block(app);
    launcher::force_foreground(HWND(block.target as _));
    Ok(())
}

fn send_close_tab() {
    let key = |vk: VIRTUAL_KEY, up: bool| INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: vk, dwFlags: if up { KEYEVENTF_KEYUP } else { Default::default() }, dwExtraInfo: crate::hotkey::MARKER, ..Default::default() } },
    };
    let inputs = [key(VK_CONTROL, false), key(VK_W, false), key(VK_W, true), key(VK_CONTROL, true)];
    unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
}

pub fn close_tab(app: &AppHandle) -> Result<(), String> {
    let block = with_state(|s| s.block.clone()).ok_or("Nothing is blocked")?;
    hide_block(app);
    let target = HWND(block.target as _);
    launcher::force_foreground(target);
    std::thread::spawn(|| {
        std::thread::sleep(Duration::from_millis(150));
        send_close_tab();
    });
    Ok(())
}

fn process_windows(pid: u32) -> Vec<HWND> {
    unsafe extern "system" fn collect(hwnd: HWND, data: LPARAM) -> windows::core::BOOL {
        let (pid, list) = unsafe { &mut *(data.0 as *mut (u32, Vec<HWND>)) };
        let mut owner = 0;
        unsafe { GetWindowThreadProcessId(hwnd, Some(&mut owner)) };
        if owner == *pid && unsafe { IsWindowVisible(hwnd).as_bool() } {
            list.push(hwnd);
        }
        true.into()
    }
    let mut data = (pid, Vec::new());
    unsafe {
        let _ = EnumWindows(Some(collect), LPARAM(&mut data as *mut _ as isize));
    }
    data.1
}

pub fn block_left(app: &AppHandle) {
    let id = with_state(|s| s.block.as_ref().map(|b| b.key.clone()));
    let Some(key) = id else { return };
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(400));
        let block = with_state(|s| s.block.clone().filter(|b| b.key == key));
        let Some(block) = block else { return };
        let target = HWND(block.target as _);
        let now_front = unsafe { GetAncestor(GetForegroundWindow(), GA_ROOT) };
        let mut front_pid = 0;
        unsafe { GetWindowThreadProcessId(now_front, Some(&mut front_pid)) };
        if front_pid == std::process::id() {
            return;
        }
        if now_front == target || front_pid == block.pid {
            let handle = app.clone();
            let _ = app.run_on_main_thread(move || {
                if let Some(overlay) = handle.get_webview_window(BLOCK).as_ref().and_then(hwnd) {
                    launcher::force_foreground(overlay);
                }
            });
            return;
        }
        if !block.website {
            for hwnd in process_windows(block.pid) {
                unsafe {
                    let _ = PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0));
                }
            }
            with_state(|s| {
                if !s.terminated.contains(&block.exe) {
                    s.terminated.push(block.exe.clone());
                }
            });
        }
        let handle = app.clone();
        let _ = app.run_on_main_thread(move || hide_block(&handle));
    });
}

fn resolve_app(raw: &str) -> String {
    if raw.to_lowercase().ends_with(".lnk") {
        if let Some(target) = shortcut_target(raw) {
            return app_id(&target);
        }
    }
    app_id(raw)
}

pub(crate) fn shortcut_target(path: &str) -> Option<String> {
    let path: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
    std::thread::scope(|scope| {
        scope
            .spawn(|| unsafe {
                let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
                let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).ok()?;
                let file: IPersistFile = windows::core::Interface::cast(&link).ok()?;
                file.Load(PCWSTR(path.as_ptr()), STGM_READ).ok()?;
                let mut buffer = [0u16; 520];
                link.GetPath(&mut buffer, std::ptr::null_mut(), 0).ok()?;
                let len = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
                (len > 0).then(|| String::from_utf16_lossy(&buffer[..len]))
            })
            .join()
            .ok()
            .flatten()
    })
}

struct Foreground {
    hwnd: HWND,
    pid: u32,
    exe: String,
    candidates: Vec<String>,
}

pub(crate) fn process_info(pid: u32) -> Option<(String, Option<String>)> {
    unsafe {
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buffer = [0u16; 1024];
        let mut len = buffer.len() as u32;
        let path = QueryFullProcessImageNameW(process, PROCESS_NAME_WIN32, PWSTR(buffer.as_mut_ptr()), &mut len)
            .ok()
            .map(|_| String::from_utf16_lossy(&buffer[..len as usize]));
        let mut family = [0u16; 256];
        let mut family_len = family.len() as u32;
        let package = (GetPackageFamilyName(process, &mut family_len, Some(PWSTR(family.as_mut_ptr()))).0 == 0)
            .then(|| String::from_utf16_lossy(&family[..family_len.saturating_sub(1) as usize]).to_lowercase());
        let _ = CloseHandle(process);
        Some((path?, package))
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

fn foreground() -> Option<Foreground> {
    let hwnd = unsafe { GetAncestor(GetForegroundWindow(), GA_ROOT) };
    if hwnd.is_invalid() {
        return None;
    }
    let mut pid = 0;
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    if pid == 0 || pid == std::process::id() {
        return None;
    }
    let (mut exe, mut package) = process_info(pid)?;
    if app_id(&exe) == "applicationframehost.exe" {
        let hosted = hosted_pid(hwnd, pid);
        if hosted != pid {
            pid = hosted;
            (exe, package) = process_info(pid)?;
        }
    }
    let mut candidates = vec![app_id(&exe)];
    candidates.extend(package);
    Some(Foreground { hwnd, pid, exe, candidates })
}

struct UrlReader {
    automation: Option<IUIAutomation>,
    cache: Option<(isize, IUIAutomationElement)>,
}

impl UrlReader {
    fn read(&mut self, hwnd: HWND) -> Option<String> {
        if self.automation.is_none() {
            self.automation = unsafe { CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER) }.ok();
        }
        let automation = self.automation.as_ref()?;
        if self.cache.as_ref().map(|(h, _)| *h) != Some(hwnd.0 as isize) {
            let element = unsafe {
                let root = automation.ElementFromHandle(hwnd).ok()?;
                let condition = automation.CreatePropertyCondition(UIA_ControlTypePropertyId, &VARIANT::from(UIA_EditControlTypeId.0)).ok()?;
                root.FindFirst(TreeScope_Descendants, &condition).ok()?
            };
            self.cache = Some((hwnd.0 as isize, element));
        }
        let (_, element) = self.cache.as_ref()?;
        let value = unsafe { element.GetCurrentPropertyValue(UIA_ValueValuePropertyId) };
        match value {
            Ok(value) => BSTR::try_from(&value).ok().map(|b| b.to_string()),
            Err(_) => {
                self.cache = None;
                None
            }
        }
    }
}

fn check(app: &AppHandle, reader: &mut UrlReader) {
    let now = now_ms();
    let (session, blocking) = with_state(|s| {
        s.snoozed.retain(|_, until| *until > now);
        (s.session.clone(), s.block.is_some())
    });
    let Some(session) = session else { return };
    if session.status == Status::Paused {
        if session.pause_ends_at.is_some_and(|end| end <= now) {
            let handle = app.clone();
            let _ = app.run_on_main_thread(move || {
                let _ = resume(&handle);
            });
        }
        return;
    }
    if session.end_at.is_some_and(|end| end <= now) {
        let handle = app.clone();
        let _ = app.run_on_main_thread(move || {
            let _ = complete(&handle);
        });
        return;
    }
    if blocking {
        return;
    }
    let Some(fg) = foreground() else { return };
    let exe_id = app_id(&fg.exe);
    let snoozed = |key: &str| with_state(|s| s.snoozed.contains_key(key));
    let name = || crate::clipboard::window_app(fg.hwnd).map(|(name, _)| name).unwrap_or_else(|| exe_id.trim_end_matches(".exe").to_string());
    let mut block = None;
    if app_blocked(&session.rules, &fg.candidates) {
        let key = format!("app:{}", fg.candidates.last().cloned().unwrap_or_default());
        if !snoozed(&key) {
            block = Some(BlockInfo { key, name: name(), website: false, snooze_ms: session.snooze_ms, target: fg.hwnd.0 as isize, pid: fg.pid, exe: fg.exe.clone() });
        }
    } else if BROWSERS.contains(&exe_id.as_str()) {
        if let Some(host) = reader.read(fg.hwnd).and_then(|url| host_of(&url)) {
            let key = format!("site:{host}");
            if website_blocked(&session.rules, &host) && !snoozed(&key) {
                block = Some(BlockInfo { key, name: host, website: true, snooze_ms: session.snooze_ms, target: fg.hwnd.0 as isize, pid: fg.pid, exe: fg.exe.clone() });
            }
        }
    }
    if let Some(block) = block {
        let handle = app.clone();
        let _ = app.run_on_main_thread(move || show_block(&handle, block));
    }
}

fn ensure_worker(app: &AppHandle) {
    if WORKER.swap(true, Ordering::SeqCst) {
        return;
    }
    let app = app.clone();
    std::thread::spawn(move || {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        }
        let mut reader = UrlReader { automation: None, cache: None };
        loop {
            std::thread::sleep(TICK);
            check(&app, &mut reader);
        }
    });
}

pub fn panel_menu(app: &AppHandle, menu: &str, x: f64, y: f64) -> Result<(), String> {
    let window = app.get_webview_window(PANEL).ok_or("No focus panel")?;
    let hwnd = hwnd(&window).ok_or("No focus panel")?;
    let origin = window.inner_position().map_err(|e| e.to_string())?;
    let scale = window.scale_factor().unwrap_or(1.0);
    let point = POINT { x: origin.x + (x * scale).round() as i32, y: origin.y + (y * scale).round() as i32 };
    let items: Vec<(u32, &str)> = match menu {
        "pause" => vec![(1, "1 minute"), (2, "2 minutes"), (5, "5 minutes"), (10, "10 minutes"), (0, ""), (99, "Indefinitely")],
        _ => vec![(1, "Edit"), (2, "Cancel"), (0, ""), (3, "Reset Panel Position")],
    };
    let previous = unsafe { GetForegroundWindow() };
    let choice = unsafe {
        let popup = CreatePopupMenu().map_err(|e| e.message())?;
        for (id, title) in &items {
            if *id == 0 {
                let _ = AppendMenuW(popup, MF_SEPARATOR, 0, PCWSTR::null());
            } else {
                let text: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
                let _ = AppendMenuW(popup, MF_STRING, *id as usize, PCWSTR(text.as_ptr()));
            }
        }
        let _ = SetForegroundWindow(hwnd);
        MENU_OPEN.store(true, Ordering::SeqCst);
        let choice = TrackPopupMenu(popup, TPM_RETURNCMD | TPM_LEFTALIGN | TPM_BOTTOMALIGN, point.x, point.y, None, hwnd, None).0 as u32;
        let _ = DestroyMenu(popup);
        MENU_OPEN.store(false, Ordering::SeqCst);
        choice
    };
    if !previous.is_invalid() && previous != hwnd {
        launcher::force_foreground(previous);
    }
    match (menu, choice) {
        (_, 0) => Ok(()),
        ("pause", 99) => pause(app, None),
        ("pause", minutes) => pause(app, Some(u64::from(minutes) * 60_000)),
        (_, 1) => {
            launcher::show(app);
            let _ = app.emit_to(launcher::LABEL, "run-command", "focus/edit");
            Ok(())
        }
        (_, 2) => cancel(app),
        (_, 3) => {
            reset_panel(app);
            Ok(())
        }
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules(allow: bool, apps: &[&str], websites: &[&str], allow_all: bool) -> Rules {
        Rules { allow, apps: apps.iter().map(|s| s.to_string()).collect(), websites: websites.iter().map(|s| s.to_string()).collect(), allow_all_websites: allow_all }
    }

    #[test]
    fn app_ids_come_from_paths_and_packaged_ids() {
        assert_eq!(app_id(r"C:\Program Files\Discord\Discord.exe"), "discord.exe");
        assert_eq!(app_id(r"shell:AppsFolder\5319275A.WhatsAppDesktop_cv1g1gvanyjgm!App"), "5319275a.whatsappdesktop_cv1g1gvanyjgm");
        assert_eq!(app_id(r"shell:AppsFolder\{1AC14E77-02E7-4E5D-B744-2EB1AE5198B7}\notepad.exe"), "notepad.exe");
        assert_eq!(app_id("Steam.exe"), "steam.exe");
    }

    #[test]
    fn hosts_are_parsed_from_address_bar_values() {
        assert_eq!(host_of("https://www.youtube.com/watch?v=1").as_deref(), Some("youtube.com"));
        assert_eq!(host_of("x.com/home").as_deref(), Some("x.com"));
        assert_eq!(host_of("http://user@news.ycombinator.com:443/item").as_deref(), Some("news.ycombinator.com"));
        assert_eq!(host_of("search for cats"), None);
        assert_eq!(host_of("localhost"), None);
    }

    #[test]
    fn host_patterns_match_subdomains_and_any_tld() {
        assert!(host_matches("youtube.com", "youtube.com"));
        assert!(host_matches("youtube.com", "m.youtube.com"));
        assert!(!host_matches("youtube.com", "notyoutube.com"));
        assert!(host_matches("amazon.*", "amazon.com"));
        assert!(host_matches("amazon.*", "amazon.co.uk"));
        assert!(host_matches("amazon.*", "smile.amazon.de"));
        assert!(!host_matches("amazon.*", "amazonia.com"));
    }

    #[test]
    #[ignore = "opens a private Edge window and reads its address bar"]
    fn reads_the_address_bar_of_a_real_browser() {
        std::process::Command::new(r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe")
            .args(["--inprivate", "--new-window", "https://example.com/"])
            .spawn()
            .unwrap();
        std::thread::sleep(Duration::from_secs(5));
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        }
        let fg = foreground().expect("a foreground window");
        assert!(fg.candidates.contains(&"msedge.exe".to_string()), "{:?}", fg.candidates);
        let mut reader = UrlReader { automation: None, cache: None };
        let started = std::time::Instant::now();
        let url = reader.read(fg.hwnd);
        let elapsed = started.elapsed();
        unsafe {
            let _ = PostMessageW(Some(fg.hwnd), WM_CLOSE, WPARAM(0), LPARAM(0));
        }
        assert_eq!(url.as_deref().and_then(host_of).as_deref(), Some("example.com"), "{url:?}");
        let again = std::time::Instant::now();
        let _ = reader.read(fg.hwnd);
        println!("first read {elapsed:?}, cached read {:?}", again.elapsed());
    }

    #[test]
    fn block_mode_blocks_only_listed_items() {
        let r = rules(false, &["discord.exe"], &["x.com"], false);
        assert!(app_blocked(&r, &["discord.exe".into()]));
        assert!(!app_blocked(&r, &["code.exe".into()]));
        assert!(website_blocked(&r, "mobile.x.com"));
        assert!(!website_blocked(&r, "github.com"));
    }

    #[test]
    fn allow_mode_blocks_everything_else_but_the_shell() {
        let r = rules(true, &["code.exe", "chrome.exe"], &["github.com"], false);
        assert!(app_blocked(&r, &["discord.exe".into()]));
        assert!(!app_blocked(&r, &["code.exe".into()]));
        assert!(!app_blocked(&r, &["explorer.exe".into()]));
        assert!(website_blocked(&r, "x.com"));
        assert!(!website_blocked(&r, "github.com"));
        assert!(website_blocked(&rules(true, &["chrome.exe"], &[], false), "x.com"));
        assert!(!website_blocked(&rules(true, &["chrome.exe"], &[], true), "x.com"));
    }
}
