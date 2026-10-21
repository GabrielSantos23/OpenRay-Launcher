mod app_volume;
mod cache;
mod capture;
mod apps;
mod explorer;
mod menu_items;
mod scripts;
mod screenshot;
mod clipboard;
mod file_search;
mod flowkey;
mod games;
mod focus;
mod registry;
mod run;
pub mod hotkey;
mod hud;
mod launcher;
mod notes;
mod settings_pages;
mod snippets;
mod sound_devices;
mod store;
mod system_actions;
mod toggles;
mod uninstall;
mod translate;
mod window_layout;
mod window_switcher;

use std::collections::HashMap;
use store::{Settings, Store};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, State, WebviewWindow, WindowEvent};

static APPS: cache::Cached<Vec<apps::InstalledApp>> = cache::Cached::new(std::time::Duration::from_secs(20));
static SYSTEM_PAGES: cache::Cached<Vec<settings_pages::SystemPage>> = cache::Cached::new(std::time::Duration::from_secs(600));
static GAMES: cache::Cached<Vec<games::Game>> = cache::Cached::new(std::time::Duration::from_secs(60));

fn scan_apps() -> Vec<apps::InstalledApp> {
    let games = GAMES.get(games::steam_games);
    apps::scan(&apps::start_menu_roots(), apps::packaged_apps()).into_iter().filter(|a| !(a.path.to_lowercase().ends_with(".url") && games::is_steam_shortcut(&a.path, &games))).collect()
}

#[tauri::command]
async fn list_apps() -> Vec<apps::InstalledApp> {
    APPS.get(scan_apps)
}

#[derive(serde::Serialize)]
struct GameList {
    games: Vec<games::Game>,
    launchers: Vec<games::Launcher>,
}

#[tauri::command]
async fn list_system_pages() -> Vec<settings_pages::SystemPage> {
    SYSTEM_PAGES.get(settings_pages::system_pages)
}

#[tauri::command]
async fn list_games() -> GameList {
    GameList { games: GAMES.get(games::steam_games), launchers: games::launchers() }
}

fn serve_app_icon(request: tauri::http::Request<Vec<u8>>, responder: tauri::UriSchemeResponder) {
    let uri = request.uri();
    let path = percent_encoding::percent_decode_str(uri.path().trim_start_matches('/')).decode_utf8_lossy().into_owned();
    let query: HashMap<_, _> = url::form_urlencoded::parse(uri.query().unwrap_or("").as_bytes()).into_owned().collect();
    let size = query.get("size").and_then(|s| s.parse().ok()).filter(|s| (16..=512).contains(s)).unwrap_or(32);
    let thumbnail = query.contains_key("thumb");
    std::thread::spawn(move || {
        let response = match apps::icon_png(path, size, thumbnail) {
            Ok(png) => tauri::http::Response::builder()
                .header("Content-Type", "image/png")
                .header("Cache-Control", "max-age=86400")
                .body(png),
            Err(_) => tauri::http::Response::builder().status(404).body(Vec::new()),
        };
        responder.respond(response.expect("valid icon response"));
    });
}

#[tauri::command]
fn launch_app(path: String, as_admin: Option<bool>) -> Result<(), String> {
    apps::launch(&path, as_admin.unwrap_or(false))
}

#[tauri::command]
fn show_in_explorer(path: String) -> Result<(), String> {
    apps::show_in_explorer(&path)
}

#[tauri::command]
fn open_with(path: String) -> Result<(), String> {
    apps::open_with(&path)
}

#[tauri::command]
async fn file_search(app: AppHandle, query: String, kind: String) -> Result<file_search::Results, String> {
    tauri::async_runtime::spawn_blocking(move || file_search::search(&app, &query, &kind)).await.map_err(|e| e.to_string())
}

#[tauri::command]
fn window_list() -> Vec<window_switcher::WindowEntry> {
    window_switcher::list()
}

#[tauri::command]
async fn window_action(app: AppHandle, id: isize, action: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || window_switcher::act(&app, id, &action)).await.map_err(|e| e.to_string())?
}

#[tauri::command]
fn file_index_status() -> Option<usize> {
    file_search::status()
}

#[tauri::command]
async fn run_catalog() -> Result<Vec<run::Entry>, String> {
    tauri::async_runtime::spawn_blocking(run::catalog).await.map_err(|e| e.to_string())
}

#[tauri::command]
fn run_complete(text: String) -> Vec<run::Entry> {
    run::complete(&text)
}

#[tauri::command]
fn run_history() -> Vec<String> {
    run::history()
}

#[tauri::command]
fn run_forget(command: String) -> Result<(), String> {
    run::forget(&command)
}

#[tauri::command]
fn run_split(text: String) -> (String, String) {
    run::split(&text)
}

#[tauri::command]
fn run_execute(app: AppHandle, command: String, arguments: String, admin: bool, shell: bool) -> Result<(), String> {
    run::execute(&command, &arguments, admin, shell)?;
    launcher::hide(&app, false);
    Ok(())
}

#[tauri::command]
async fn scripts_list(directories: Vec<String>) -> Result<Vec<scripts::Script>, String> {
    tauri::async_runtime::spawn_blocking(move || scripts::list(&directories)).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn script_run(path: String, arguments: Vec<String>) -> Result<scripts::Output, String> {
    tauri::async_runtime::spawn_blocking(move || scripts::run(&path, &arguments)).await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn capture_start(app: AppHandle, mode: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || screenshot::start(&app, mode)).await.map_err(|e| e.to_string())?
}

#[tauri::command]
fn capture_pixel(x: i32, y: i32) -> Option<String> {
    screenshot::pixel(x, y)
}

#[tauri::command]
fn capture_pending() -> Option<serde_json::Value> {
    screenshot::pending()
}

#[tauri::command]
fn capture_ready(app: AppHandle) {
    screenshot::ready(&app)
}

#[tauri::command]
async fn capture_finish(app: AppHandle, rect: capture::Rect) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || screenshot::finish(&app, rect)).await.map_err(|e| e.to_string())?
}

#[tauri::command]
fn capture_cancel(app: AppHandle) {
    screenshot::cancel(&app)
}

#[tauri::command]
async fn capture_direct(app: AppHandle, kind: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || screenshot::direct(&app, &kind)).await.map_err(|e| e.to_string())?
}

#[tauri::command]
fn pin_file(app: AppHandle, path: String) -> Result<(), String> {
    screenshot::pin_file(&app, &path)
}

#[tauri::command]
fn pin_shown(app: AppHandle, label: String) {
    screenshot::pin_shown(&app, &label)
}

#[tauri::command]
fn pin_copy(label: String) -> Result<(), String> {
    clipboard::write_image(&screenshot::pin_image(&label).ok_or("This pin is gone")?)
}

#[tauri::command]
fn pin_close(app: AppHandle, label: String) {
    screenshot::pin_closed(&label);
    if let Some(window) = app.get_webview_window(&label) {
        let _ = window.destroy();
    }
}

#[tauri::command]
async fn screenshots_list() -> Result<Vec<serde_json::Value>, String> {
    tauri::async_runtime::spawn_blocking(|| {
        capture::screenshots(&[capture::folder(), capture::system_folder()])
            .into_iter()
            .map(|(path, modified, size)| {
                serde_json::json!({ "name": path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(), "path": path.to_string_lossy(), "modified": modified, "size": size, "video": false })
            })
            .collect()
    })
    .await
    .map_err(|e| e.to_string())
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ScreenshotFolder {
    path: String,
    default_path: String,
    custom: bool,
}

#[tauri::command]
fn screenshots_folder() -> ScreenshotFolder {
    let custom = capture::CUSTOM_FOLDER.lock().unwrap_or_else(|e| e.into_inner()).is_some();
    ScreenshotFolder { path: capture::folder().to_string_lossy().into_owned(), default_path: capture::default_folder().to_string_lossy().into_owned(), custom }
}

#[tauri::command]
fn screenshots_set_folder(path: Option<String>, store: State<Store>) -> Result<ScreenshotFolder, String> {
    let value = serde_json::json!({ "folder": path.unwrap_or_default() });
    let folder = capture::custom_folder(&value);
    if let Some(dir) = &folder {
        std::fs::create_dir_all(dir).map_err(|e| format!("Can't use {}: {e}", dir.display()))?;
    } else if value["folder"].as_str().is_some_and(|p| !p.trim().is_empty()) {
        return Err("Choose a full folder path".into());
    }
    store.set_data("screenshots", &value)?;
    *capture::CUSTOM_FOLDER.lock().unwrap_or_else(|e| e.into_inner()) = folder;
    Ok(screenshots_folder())
}

#[tauri::command]
async fn image_text(path: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let png = std::fs::read(&path).map_err(|e| e.to_string())?;
        let mut reader = png::Decoder::new(std::io::Cursor::new(png)).read_info().map_err(|_| "Only PNG screenshots can be read".to_string())?;
        let mut buffer = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buffer).map_err(|e| e.to_string())?;
        let channels = info.color_type.samples();
        if info.bit_depth != png::BitDepth::Eight || channels < 3 {
            return Err("This image format can't be read".into());
        }
        let bgra = buffer[..info.buffer_size()].chunks_exact(channels).flat_map(|p| [p[2], p[1], p[0], 255]).collect();
        capture::recognize_text(&capture::Frame { rect: capture::Rect { x: 0, y: 0, w: info.width as i32, h: info.height as i32 }, bgra })
    })
    .await
    .map_err(|e| e.to_string())?
}

fn copy_image_file(path: &str) -> Result<(), String> {
    if path.to_lowercase().ends_with(".png") {
        clipboard::write_image(&std::fs::read(path).map_err(|e| e.to_string())?)
    } else {
        clipboard::write_files(&[path.to_string()])
    }
}

#[tauri::command]
fn image_copy(path: String) -> Result<(), String> {
    copy_image_file(&path)
}

#[tauri::command]
fn image_paste(app: AppHandle, path: String) -> Result<(), String> {
    copy_image_file(&path)?;
    paste_into_previous(&app);
    Ok(())
}

#[tauri::command]
fn image_edit(path: String) -> Result<(), String> {
    apps::edit(&path)
}

#[tauri::command]
fn screenshot_paste_latest(app: AppHandle) -> Result<(), String> {
    let (latest, _, _) = capture::screenshots(&[capture::folder(), capture::system_folder()]).into_iter().next().ok_or("No screenshots found")?;
    copy_image_file(&latest.to_string_lossy())?;
    paste_into_previous(&app);
    Ok(())
}

#[tauri::command]
fn explorer_selection() -> Result<Vec<String>, String> {
    explorer::selected_files(launcher::previous_window())
}

#[tauri::command]
fn menu_items_list() -> (String, Vec<menu_items::MenuItem>) {
    let hwnd = launcher::previous_window();
    let app = clipboard::window_app(hwnd).map(|(name, _)| name).unwrap_or_default();
    (app, menu_items::list(hwnd))
}

#[tauri::command]
fn menu_items_invoke(app: AppHandle, id: u32) -> Result<(), String> {
    let hwnd = launcher::previous_window();
    launcher::hide(&app, false);
    menu_items::invoke(hwnd, id)
}

#[tauri::command]
fn file_reindex(app: AppHandle) {
    file_search::reindex(&app)
}

#[tauri::command]
async fn file_info(path: String) -> Result<file_search::FileInfo, String> {
    tauri::async_runtime::spawn_blocking(move || file_search::info(&path)).await.map_err(|e| e.to_string())?
}

#[tauri::command]
fn frecency_reset(key: String, store: State<Store>) -> Result<(), String> {
    store.reset(&key).map_err(|e| e.to_string())
}

#[tauri::command]
fn favorites_get(store: State<Store>) -> Vec<String> {
    store.favorites()
}

#[tauri::command]
fn favorites_set(keys: Vec<String>, store: State<Store>) -> Result<(), String> {
    store.set_favorites(keys).map_err(|e| e.to_string())
}

#[tauri::command]
fn frecency_visit(key: String, store: State<Store>) -> Result<f64, String> {
    store.visit(&key, store::now_ms()).map_err(|e| e.to_string())
}

#[tauri::command]
fn frecency_all(store: State<Store>) -> HashMap<String, f64> {
    store.scores(store::now_ms())
}

#[tauri::command]
fn hide_launcher(app: AppHandle, restore_focus: bool) {
    launcher::hide(&app, restore_focus)
}

#[tauri::command]
fn show_hud(app: AppHandle, toast: serde_json::Value) -> Result<(), String> {
    app.emit_to(hud::LABEL, "hud-show", toast).map_err(|e| e.to_string())
}

#[tauri::command]
fn hud_present(app: AppHandle, width: f64, height: f64) {
    hud::present(&app, width, height)
}

#[tauri::command]
fn hud_hide(app: AppHandle) {
    hud::hide(&app)
}

#[tauri::command]
fn quit_app(app: AppHandle) {
    app.exit(0)
}

const SETTINGS_LABEL: &str = "settings";
const LAUNCHER_BINDING: &str = "launcher";

fn hotkey_bindings(settings: &Settings) -> Result<Vec<(String, hotkey::Hotkey)>, String> {
    let disabled = |key: &str| {
        let extension = key.split('/').next().unwrap_or(key);
        [key, extension].iter().any(|k| settings.commands.get(*k).is_some_and(|p| p.disabled))
    };
    let mut pairs = vec![(LAUNCHER_BINDING.to_string(), settings.hotkey.clone())];
    let mut commands: Vec<_> = settings.commands.iter().filter(|(key, _)| key.contains('/') && !disabled(key)).collect();
    commands.sort_by_key(|(key, _)| key.as_str());
    pairs.extend(commands.into_iter().map(|(key, prefs)| (key.clone(), prefs.hotkey.clone())));
    hotkey::bindings(&pairs)
}

fn apply_settings(app: &AppHandle, settings: &Settings) -> Result<(), String> {
    hotkey::set_bindings(hotkey_bindings(settings)?);
    if let Some(tray) = app.tray_by_id("main") {
        let _ = tray.set_visible(settings.show_in_tray);
    }
    Ok(())
}

#[tauri::command]
fn settings_get(store: State<Store>) -> Settings {
    store.settings()
}

#[tauri::command]
fn settings_set(app: AppHandle, settings: Settings, store: State<Store>) -> Result<(), String> {
    hotkey_bindings(&settings)?;
    store.set_settings(settings.clone()).map_err(|e| e.to_string())?;
    apply_settings(&app, &settings)?;
    app.emit("settings-changed", settings).map_err(|e| e.to_string())
}

#[tauri::command]
fn hotkeys_suspend(suspended: bool) {
    hotkey::suspend(suspended)
}

#[tauri::command]
fn system_accent_color() -> Option<String> {
    registry::accent_color()
}

#[tauri::command]
fn login_item_get() -> bool {
    registry::login_item_enabled()
}

#[tauri::command]
fn login_item_set(enabled: bool) -> Result<(), String> {
    registry::set_login_item(enabled)
}

#[derive(serde::Serialize)]
struct AppInfo {
    name: String,
    path: String,
}

#[tauri::command]
fn frontmost_app() -> Option<AppInfo> {
    let previous = launcher::previous_window();
    if previous.is_invalid() {
        return None;
    }
    clipboard::window_app(previous).map(|(name, path)| AppInfo { name, path })
}

fn paste_into_previous(app: &AppHandle) {
    launcher::hide(app, true);
    std::thread::spawn(|| {
        std::thread::sleep(std::time::Duration::from_millis(80));
        clipboard::send_paste();
    });
}

#[tauri::command]
fn paste_text(app: AppHandle, text: String) -> Result<(), String> {
    clipboard::write_text(&text)?;
    paste_into_previous(&app);
    Ok(())
}

#[tauri::command]
fn show_launcher(app: AppHandle) {
    launcher::show(&app)
}

#[tauri::command]
fn read_clipboard_text() -> Option<String> {
    clipboard::read_clipboard_text()
}

#[tauri::command]
fn copy_text(text: String) -> Result<(), String> {
    clipboard::write_text(&text)
}

#[tauri::command]
fn clipboard_list(history: State<clipboard::History>) -> Vec<clipboard::Entry> {
    history.list()
}

fn clipboard_entry(history: &clipboard::History, id: &str) -> Result<clipboard::Entry, String> {
    history.get(id).ok_or_else(|| "entry no longer exists".to_string())
}

#[tauri::command]
fn clipboard_copy(id: String, history: State<clipboard::History>) -> Result<(), String> {
    clipboard::write_entry(&history, &clipboard_entry(&history, &id)?)
}

#[tauri::command]
fn clipboard_paste(app: AppHandle, id: String, history: State<clipboard::History>) -> Result<(), String> {
    clipboard::write_entry(&history, &clipboard_entry(&history, &id)?)?;
    paste_into_previous(&app);
    Ok(())
}

static PASTE_SEQUENCE: std::sync::Mutex<Option<clipboard::PasteSequence>> = std::sync::Mutex::new(None);
const PASTE_SEQUENCE_TIMEOUT_MS: u64 = 60_000;

#[tauri::command]
fn clipboard_paste_next(app: AppHandle, history: State<clipboard::History>) -> Result<bool, String> {
    let recent = history.recent();
    let now = store::now_ms();
    let next = PASTE_SEQUENCE.lock().unwrap().get_or_insert_with(Default::default).next(&recent, now, PASTE_SEQUENCE_TIMEOUT_MS);
    let Some(entry) = next else {
        launcher::hide(&app, true);
        return Ok(false);
    };
    clipboard::suppress_for(now, 2_000);
    match &entry.content {
        clipboard::Content::Text { text } => clipboard::write_text(&format!("{text}\n"))?,
        _ => clipboard::write_entry(&history, &entry)?,
    }
    launcher::hide(&app, true);
    let handle = app.clone();
    let top = recent.into_iter().next();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(80));
        clipboard::send_paste();
        std::thread::sleep(std::time::Duration::from_millis(500));
        if let Some(top) = top {
            clipboard::suppress_for(store::now_ms(), 1_000);
            let _ = clipboard::write_entry(&handle.state::<clipboard::History>(), &top);
        }
    });
    Ok(true)
}

#[tauri::command]
fn clipboard_reset_sequence() {
    if let Some(sequence) = PASTE_SEQUENCE.lock().unwrap().as_mut() {
        sequence.reset();
    }
}

fn clipboard_changed(app: &AppHandle, result: Result<(), String>) -> Result<(), String> {
    result?;
    app.emit_to(launcher::LABEL, "clipboard-changed", ()).map_err(|e| e.to_string())
}

#[tauri::command]
fn clipboard_delete(app: AppHandle, id: String, history: State<clipboard::History>) -> Result<(), String> {
    clipboard_changed(&app, history.delete(&id))
}

#[tauri::command]
fn clipboard_delete_all(app: AppHandle, history: State<clipboard::History>) -> Result<(), String> {
    clipboard_changed(&app, history.delete_all())
}

#[tauri::command]
fn clipboard_delete_since(app: AppHandle, since: u64, history: State<clipboard::History>) -> Result<(), String> {
    clipboard_changed(&app, history.delete_since(since))
}

#[tauri::command]
fn clipboard_pin(app: AppHandle, id: String, pinned: bool, history: State<clipboard::History>) -> Result<(), String> {
    clipboard_changed(&app, history.set_pinned(&id, pinned, store::now_ms()))
}

#[tauri::command]
fn data_get(name: String, store: State<Store>) -> Result<serde_json::Value, String> {
    store.data(&name)
}

#[tauri::command]
fn data_set(name: String, value: serde_json::Value, store: State<Store>) -> Result<(), String> {
    store.set_data(&name, &value)?;
    if name == "snippets" {
        snippets::load(&value);
    }
    Ok(())
}

#[tauri::command]
fn snippet_type(text: String, cursor_back: usize) {
    snippets::type_text(&text, cursor_back)
}

#[tauri::command]
fn open_url(url: String, app_path: Option<String>) -> Result<(), String> {
    apps::open_url(&url, app_path.as_deref())
}

#[tauri::command]
fn notes_show(app: AppHandle, request: serde_json::Value) {
    launcher::hide(&app, false);
    notes::show(&app, request)
}

#[tauri::command]
fn focus_get() -> Option<focus::Session> {
    focus::get()
}

#[tauri::command]
fn focus_start(app: AppHandle, input: focus::StartInput) -> focus::Session {
    focus::start(&app, input)
}

#[tauri::command]
fn focus_update(app: AppHandle, input: focus::StartInput) -> Result<focus::Session, String> {
    focus::update(&app, input)
}

#[tauri::command]
fn focus_pause(app: AppHandle, duration_ms: Option<u64>) -> Result<(), String> {
    focus::pause(&app, duration_ms)
}

#[tauri::command]
fn focus_resume(app: AppHandle) -> Result<(), String> {
    focus::resume(&app)
}

#[tauri::command]
fn focus_restart(app: AppHandle) -> Result<(), String> {
    focus::restart(&app)
}

#[tauri::command]
fn focus_complete(app: AppHandle) -> Result<(), String> {
    focus::complete(&app)
}

#[tauri::command]
fn focus_cancel(app: AppHandle) -> Result<(), String> {
    focus::cancel(&app)
}

#[tauri::command]
fn focus_snooze(app: AppHandle) -> Result<(), String> {
    focus::snooze(&app)
}

#[tauri::command]
fn focus_close_tab(app: AppHandle) -> Result<(), String> {
    focus::close_tab(&app)
}

#[tauri::command]
fn focus_block_info() -> Option<serde_json::Value> {
    focus::current_block()
}

#[tauri::command]
fn focus_panel_menu(app: AppHandle, menu: String, x: f64, y: f64) -> Result<(), String> {
    focus::panel_menu(&app, &menu, x, y)
}

#[tauri::command]
fn focus_panel_reset(app: AppHandle) {
    focus::reset_panel(&app)
}

#[tauri::command]
fn focus_toast_hide(app: AppHandle) {
    focus::hide_toast(&app)
}

#[tauri::command]
fn read_json_file(path: String) -> Result<String, String> {
    if !path.to_lowercase().ends_with(".json") {
        return Err("Choose a JSON file".into());
    }
    let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    Ok(text.strip_prefix('\u{feff}').map(str::to_string).unwrap_or(text))
}

#[tauri::command]
async fn app_volumes() -> Result<Vec<app_volume::AppAudio>, String> {
    tauri::async_runtime::spawn_blocking(app_volume::list).await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn app_volume_set(id: String, volume: Option<f32>, muted: Option<bool>) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || app_volume::set(&id, volume, muted)).await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn app_volume_active(action: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || app_volume::change_active(&action)).await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn sound_devices(direction: String) -> Result<Vec<sound_devices::SoundDevice>, String> {
    tauri::async_runtime::spawn_blocking(move || sound_devices::list(&direction)).await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn sound_device_set(id: String, default: Option<bool>, volume: Option<f32>, muted: Option<bool>) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        if default == Some(true) {
            sound_devices::set_default(&id)?;
        }
        if let Some(volume) = volume {
            sound_devices::set_volume(&id, volume)?;
        }
        if let Some(muted) = muted {
            sound_devices::set_muted(&id, muted)?;
        }
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn app_uninstaller(name: String, path: String) -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(move || uninstall::describe(&name, &path)).await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn app_uninstall(name: String, path: String) -> Result<String, String> {
    let result = tauri::async_runtime::spawn_blocking(move || uninstall::uninstall(&name, &path).map(str::to_string)).await.map_err(|e| e.to_string())?;
    APPS.clear();
    result
}

#[tauri::command]
async fn system_toggle(action: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || toggles::run(&action)).await.map_err(|e| e.to_string())?
}

#[tauri::command]
fn system_action(app: AppHandle, action: String) -> Result<(), String> {
    system_actions::run(&app, &action)
}

#[tauri::command]
fn window_layout(app: AppHandle, layout: String) -> Result<(), String> {
    window_layout::apply(&layout, launcher::monitor(&app))
}

#[tauri::command]
fn notes_pending() -> Option<serde_json::Value> {
    notes::take_pending()
}

#[tauri::command]
fn tooltip_show(app: AppHandle, window: WebviewWindow, tooltip: serde_json::Value, rect: [f64; 4]) -> Result<(), String> {
    notes::set_tooltip_anchor(&window, Some(rect));
    app.emit_to(notes::TOOLTIP_LABEL, "tooltip-show", tooltip).map_err(|e| e.to_string())
}

#[tauri::command]
fn tooltip_present(app: AppHandle, width: f64, height: f64) {
    notes::present_tooltip(&app, width, height)
}

#[tauri::command]
fn tooltip_hide(app: AppHandle) {
    notes::hide_tooltip(&app)
}

fn serve_clipboard_image(app: &AppHandle, request: tauri::http::Request<Vec<u8>>) -> tauri::http::Response<Vec<u8>> {
    let id = request.uri().path().trim_start_matches('/');
    let history = app.state::<clipboard::History>();
    match history.image_path(id).and_then(|path| std::fs::read(path).ok()) {
        Some(png) => tauri::http::Response::builder().header("Content-Type", "image/png").body(png),
        None => tauri::http::Response::builder().status(404).body(Vec::new()),
    }
    .expect("valid clipboard image response")
}

fn start_clipboard_listener(app: &AppHandle) -> Result<(), String> {
    let handle = app.clone();
    clipboard::listen(Box::new(move || {
        if clipboard::suppressed(store::now_ms()) {
            return;
        }
        let Some(captured) = clipboard::capture() else { return };
        let (name, path) = clipboard::owner_app().unwrap_or_default();
        let history = handle.state::<clipboard::History>();
        let now = store::now_ms();
        history.prune(now);
        if let Err(e) = clipboard_changed(&handle, history.record(captured, name, path, now)) {
            eprintln!("openray: could not save clipboard entry: {e}");
        }
    }))
}

pub(crate) fn open_window(app: &AppHandle, label: &'static str, then: impl FnOnce(&AppHandle, WebviewWindow) + Send + 'static) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let window = match app.get_webview_window(label) {
            Some(window) => window,
            None => {
                let Some(config) = app.config().app.windows.iter().find(|w| w.label == label).cloned() else { return };
                match tauri::WebviewWindowBuilder::from_config(&app, &config).and_then(|b| b.build()) {
                    Ok(window) => window,
                    Err(e) => {
                        eprintln!("openray: could not create the {label} window: {e}");
                        return;
                    }
                }
            }
        };
        let handle = app.clone();
        let _ = app.run_on_main_thread(move || then(&handle, window));
    });
}

fn show_settings(app: &AppHandle) {
    launcher::hide(app, false);
    open_window(app, SETTINGS_LABEL, |_, window| {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    });
}

#[tauri::command]
fn open_settings(app: AppHandle) {
    show_settings(&app)
}

fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Open Openray", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Openray", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &settings, &quit])?;
    let mut tray = TrayIconBuilder::with_id("main")
        .tooltip("Openray")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => launcher::show(app),
            "settings" => show_settings(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                launcher::show(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| launcher::show(app)))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .register_asynchronous_uri_scheme_protocol("appicon", |_ctx, request, responder| serve_app_icon(request, responder))
        .register_uri_scheme_protocol("capture", |_ctx, request| screenshot::serve(request))
        .register_uri_scheme_protocol("clipimg", |ctx, request| serve_clipboard_image(ctx.app_handle(), request))
        .register_uri_scheme_protocol("flowkey", |ctx, request| flowkey::serve(ctx.app_handle(), request))
        .register_uri_scheme_protocol("extimg", |ctx, request| flowkey::serve_image(ctx.app_handle(), request))
        .setup(|app| {
            screenshot::prepare(app.handle());
            std::thread::spawn(|| {
                APPS.get(scan_apps);
                SYSTEM_PAGES.get(settings_pages::system_pages);
            });
            let data_dir = app.path().app_data_dir()?;
            app.manage(clipboard::History::load(data_dir.join("clipboard"), store::now_ms()));
            let store = Store::load(data_dir);
            let settings = store.settings();
            snippets::init(app.handle(), &store.data("snippets").unwrap_or_default());
            *capture::CUSTOM_FOLDER.lock().unwrap_or_else(|e| e.into_inner()) = capture::custom_folder(&store.data("screenshots").unwrap_or_default());
            app.manage(store);

            if let Some(window) = app.get_webview_window(launcher::LABEL) {
                launcher::init(&window);
            }
            if let Some(window) = app.get_webview_window(hud::LABEL) {
                hud::init(&window);
            }
            setup_tray(app)?;
            if let Err(e) = start_clipboard_listener(app.handle()) {
                eprintln!("openray: clipboard history unavailable: {e}");
            }

            let handle = app.handle().clone();
            let registered = hotkey::install(Box::new(move |id| {
                let app = handle.clone();
                let _ = handle.run_on_main_thread(move || {
                    if id == LAUNCHER_BINDING {
                        launcher::toggle(&app);
                    } else {
                        let _ = app.emit_to(launcher::LABEL, "run-command", id);
                    }
                });
            }))
            .and_then(|()| apply_settings(app.handle(), &settings));
            if let Err(e) = registered {
                eprintln!("openray: could not register hotkeys: {e}");
                if let Some(tray) = app.tray_by_id("main") {
                    let _ = tray.set_tooltip(Some(format!("Openray — hotkey {} unavailable", settings.hotkey)));
                }
            }

            if std::env::args().any(|a| a == "--show") {
                launcher::show(app.handle());
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            match (window.label(), event) {
                (focus::BLOCK, WindowEvent::Focused(false)) => focus::block_left(window.app_handle()),
                (focus::PANEL, WindowEvent::Moved(_)) => focus::panel_moved(window),
                _ => {}
            }
            if window.label() == notes::LABEL {
                match event {
                    WindowEvent::Moved(_) | WindowEvent::Resized(_) => notes::remember_geometry(window),
                    WindowEvent::Destroyed => notes::closed(window.app_handle()),
                    _ => {}
                }
                if matches!(event, WindowEvent::Focused(false) | WindowEvent::Moved(_)) {
                    notes::hide_tooltip(window.app_handle());
                }
            }
            if window.label() != launcher::LABEL || !matches!(event, WindowEvent::Focused(false)) {
                return;
            }
            if let Some(webview) = window.app_handle().get_webview_window(launcher::LABEL) {
                if launcher::is_shown() && !launcher::owns_foreground(&webview) {
                    launcher::hide(window.app_handle(), false);
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            list_apps,
            launch_app,
            show_in_explorer,
            open_with,
            file_search,
            file_reindex,
            file_index_status,
            run_catalog,
            run_complete,
            run_history,
            run_forget,
            run_split,
            run_execute,
            scripts_list,
            script_run,
            capture_start,
            capture_ready,
            capture_pending,
            capture_pixel,
            capture_finish,
            capture_cancel,
            capture_direct,
            pin_file,
            pin_shown,
            pin_copy,
            pin_close,
            screenshots_list,
            screenshots_folder,
            screenshots_set_folder,
            image_text,
            image_copy,
            image_paste,
            image_edit,
            screenshot_paste_latest,
            explorer_selection,
            menu_items_list,
            menu_items_invoke,
            file_info,
            frecency_visit,
            frecency_all,
            frecency_reset,
            favorites_get,
            favorites_set,
            hide_launcher,
            show_hud,
            hud_present,
            hud_hide,
            quit_app,
            settings_get,
            settings_set,
            hotkeys_suspend,
            login_item_get,
            system_accent_color,
            login_item_set,
            open_settings,
            frontmost_app,
            paste_text,
            copy_text,
            show_launcher,
            read_clipboard_text,
            clipboard_list,
            clipboard_copy,
            clipboard_paste,
            clipboard_delete,
            clipboard_delete_all,
            clipboard_delete_since,
            clipboard_pin,
            data_get,
            data_set,
            snippet_type,
            open_url,
            notes_show,
            notes_pending,
            window_layout,
            window_list,
            sound_devices,
            app_volumes,
            system_toggle,
            app_uninstall,
            app_uninstaller,
            list_games,
            list_system_pages,
            app_volume_set,
            app_volume_active,
            sound_device_set,
            clipboard_paste_next,
            clipboard_reset_sequence,
            window_action,
            system_action,
            focus_get,
            focus_start,
            focus_update,
            focus_pause,
            focus_resume,
            focus_restart,
            focus_complete,
            focus_cancel,
            focus_snooze,
            focus_close_tab,
            focus_block_info,
            focus_panel_menu,
            focus_panel_reset,
            focus_toast_hide,
            read_json_file,
            tooltip_show,
            tooltip_present,
            tooltip_hide,
            translate::translate,
            flowkey::flowkey_list,
            flowkey::flowkey_extensions_dir,
            flowkey::flowkey_package_manifest,
            flowkey::flowkey_install,
            flowkey::flowkey_uninstall,
            flowkey::flowkey_data_get,
            flowkey::flowkey_data_set,
            flowkey::flowkey_secret,
            flowkey::flowkey_http,
            flowkey::flowkey_oauth,
            flowkey::flowkey_image,
            flowkey::flowkey_media_current,
            flowkey::flowkey_media_control,
            flowkey::flowkey_fs,
            flowkey::flowkey_default_app,
            flowkey::flowkey_pick_path,
            flowkey::flowkey_selected_text,
            flowkey::flowkey_edit_clipboard_entry,
            flowkey::copy_rich,
            flowkey::read_clipboard_rich
        ])
        .run(tauri::generate_context!())
        .expect("error while running Openray");
}

#[cfg(test)]
mod tests {
    use super::*;
    use store::CommandPrefs;

    #[test]
    fn disabled_commands_and_extensions_lose_their_hotkeys() {
        let mut settings = Settings::default();
        let prefs = |hotkey: &str, disabled: bool| CommandPrefs { alias: String::new(), hotkey: hotkey.into(), disabled };
        settings.commands.insert("applications/a".into(), prefs("Ctrl+Alt+A", false));
        settings.commands.insert("applications/b".into(), prefs("Ctrl+Alt+B", true));
        settings.commands.insert("openray-core/quit".into(), prefs("Ctrl+Alt+Q", false));
        settings.commands.insert("openray-core".into(), prefs("", true));
        let ids: Vec<String> = hotkey_bindings(&settings).unwrap().into_iter().map(|(id, _)| id).collect();
        assert_eq!(ids, ["launcher", "applications/a"]);
    }
}
