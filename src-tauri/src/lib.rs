mod apps;
mod hotkey;
mod hud;
mod launcher;
mod store;

use std::collections::HashMap;
use store::Store;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, State, WindowEvent};

#[tauri::command]
async fn list_apps() -> Vec<apps::InstalledApp> {
    apps::scan(&apps::start_menu_roots(), apps::packaged_apps())
}

fn serve_app_icon(request: tauri::http::Request<Vec<u8>>, responder: tauri::UriSchemeResponder) {
    let uri = request.uri();
    let path = percent_encoding::percent_decode_str(uri.path().trim_start_matches('/')).decode_utf8_lossy().into_owned();
    let size = uri.query().and_then(|q| q.strip_prefix("size=")).and_then(|s| s.parse().ok()).filter(|s| (16..=256).contains(s)).unwrap_or(32);
    std::thread::spawn(move || {
        let response = match apps::icon_png(path, size) {
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

fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Open Openray", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Openray", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &quit])?;
    let mut tray = TrayIconBuilder::with_id("main")
        .tooltip("Openray")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => launcher::show(app),
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
        .register_asynchronous_uri_scheme_protocol("appicon", |_ctx, request, responder| serve_app_icon(request, responder))
        .setup(|app| {
            let store = Store::load(app.path().app_data_dir()?);
            let hotkey = store.settings.hotkey.clone();
            app.manage(store);

            if let Some(window) = app.get_webview_window(launcher::LABEL) {
                launcher::init(&window);
            }
            if let Some(window) = app.get_webview_window(hud::LABEL) {
                hud::init(&window);
            }
            setup_tray(app)?;

            let handle = app.handle().clone();
            let registered = hotkey::parse(&hotkey).and_then(|parsed| {
                hotkey::install(
                    parsed,
                    Box::new(move || {
                        let app = handle.clone();
                        let _ = handle.run_on_main_thread(move || launcher::toggle(&app));
                    }),
                )
            });
            if let Err(e) = registered {
                eprintln!("openray: could not register hotkey {hotkey}: {e}");
                if let Some(tray) = app.tray_by_id("main") {
                    let _ = tray.set_tooltip(Some(format!("Openray — hotkey {hotkey} unavailable")));
                }
            }

            if std::env::args().any(|a| a == "--show") {
                launcher::show(app.handle());
            }
            Ok(())
        })
        .on_window_event(|window, event| {
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
            frecency_visit,
            frecency_all,
            frecency_reset,
            favorites_get,
            favorites_set,
            hide_launcher,
            show_hud,
            hud_present,
            hud_hide,
            quit_app
        ])
        .run(tauri::generate_context!())
        .expect("error while running Openray");
}
