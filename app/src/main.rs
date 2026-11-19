#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[allow(dead_code, unused_imports)]
mod apps;
#[allow(dead_code, unused_imports)]
mod clipboard;
#[allow(dead_code, unused_imports)]
mod system_actions;
#[allow(dead_code, unused_imports)]
mod window_layout;
#[allow(dead_code, unused_imports)]
mod toggles;
#[allow(dead_code, unused_imports)]
mod app_volume;
#[allow(dead_code, unused_imports)]
mod sound_devices;
#[allow(dead_code, unused_imports)]
mod window_switcher;
#[allow(dead_code, unused_imports)]
mod settings_pages;
#[allow(dead_code, unused_imports)]
mod games;
#[allow(dead_code, unused_imports)]
#[allow(dead_code, unused_imports)]
mod file_search;
#[allow(dead_code, unused_imports)]
mod explorer;
mod menu_items;
#[allow(dead_code, unused_imports)]
mod capture;
mod capture_ui;
mod emoji_render;
#[allow(dead_code, unused_imports)]
mod flowkey_core;
mod flowkey_js;
mod updater;
mod flowkey_gate;
mod flowkey_host;
#[allow(dead_code, unused_imports)]
mod registry;
#[allow(dead_code, unused_imports)]
mod focus;
mod focus_ui;
mod date_picker;
mod extension;
mod http;
mod extensions;
#[allow(dead_code, unused_imports)]
mod hotkey;
mod icons;
mod i18n;
mod icu;
mod launcher;
mod root;
mod notes;
mod search;
#[allow(dead_code, unused_imports)]
mod store;
mod theme;

#[allow(dead_code, unused_imports)]
mod run;
#[allow(dead_code, unused_imports)]
mod uninstall;

#[allow(dead_code, unused_imports)]
mod contacts;
#[allow(dead_code, unused_imports)]
mod calendar;

#[allow(dead_code, unused_imports)]
mod scripts;

#[allow(dead_code, unused_imports)]
mod snippets;

use root::{Launcher, Signal};
use std::sync::atomic::{AtomicBool, Ordering};

pub static TRAY_VISIBLE: AtomicBool = AtomicBool::new(true);
use std::sync::mpsc::{channel, Sender};
use windows::core::w;
use windows::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS};
use windows::Win32::System::Threading::{CreateEventW, CreateMutexW, SetEvent, WaitForSingleObject, INFINITE};

fn data_dir() -> std::path::PathBuf {
    std::env::var_os("APPDATA").map(std::path::PathBuf::from).unwrap_or_default().join("dev.openray.app")
}

fn tray_icon() -> Option<tray_icon::Icon> {
    let decoder = png::Decoder::new(std::io::Cursor::new(include_bytes!("../assets/app-icons/32x32.png")));
    let mut reader = decoder.read_info().ok()?;
    let mut buffer = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buffer).ok()?;
    (info.color_type == png::ColorType::Rgba).then(|| tray_icon::Icon::from_rgba(buffer[..info.buffer_size()].to_vec(), info.width, info.height).ok())?
}

fn setup_tray(ctx: &egui::Context, signals: Sender<Signal>) -> Option<tray_icon::TrayIcon> {
    use tray_icon::menu::{Menu, MenuEvent, MenuItem};
    use tray_icon::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
    let open = MenuItem::with_id("open", "Open Openray", true, None);
    let settings = MenuItem::with_id("settings", "Settings", true, None);
    let quit = MenuItem::with_id("quit", "Quit Openray", true, None);
    let menu = Menu::with_items(&[&open, &settings, &quit]).ok()?;
    let (menu_ctx, menu_signals) = (ctx.clone(), signals.clone());
    MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
        let signal = match event.id.as_ref() {
            "open" => Signal::Show,
            "settings" => Signal::Settings,
            "quit" => Signal::Quit,
            _ => return,
        };
        let _ = menu_signals.send(signal);
        menu_ctx.request_repaint();
    }));
    let click_ctx = ctx.clone();
    TrayIconEvent::set_event_handler(Some(move |event: TrayIconEvent| {
        if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
            let _ = signals.send(Signal::Show);
            click_ctx.request_repaint();
        }
    }));
    let mut builder = TrayIconBuilder::new().with_tooltip("Openray").with_menu(Box::new(menu)).with_menu_on_left_click(false);
    if let Some(icon) = tray_icon() {
        builder = builder.with_icon(icon);
    }
    builder.build().map_err(|e| eprintln!("openray: tray unavailable: {e}")).ok()
}

fn listen_for_second_instance(ctx: egui::Context, signals: Sender<Signal>) {
    let Ok(event) = (unsafe { CreateEventW(None, false, false, w!("Local\\dev.openray.app.show")) }) else { return };
    let event = event.0 as isize;
    std::thread::spawn(move || loop {
        unsafe { WaitForSingleObject(windows::Win32::Foundation::HANDLE(event as _), INFINITE) };
        let _ = signals.send(Signal::Show);
        ctx.request_repaint();
    });
}

struct App {
    launcher: Launcher,
    _tray: Option<tray_icon::TrayIcon>,
    tray_visible: bool,
}

impl eframe::App for App {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.launcher.logic(ctx);
        let visible = TRAY_VISIBLE.load(Ordering::SeqCst);
        if visible != self.tray_visible {
            self.tray_visible = visible;
            if let Some(tray) = &self._tray {
                let _ = tray.set_visible(visible);
            }
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.launcher.ui(ui);
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        theme::BG().to_normalized_gamma_f32()
    }
}

fn main() -> eframe::Result {
    if std::env::args().any(|a| a == hotkey::HELPER_ARG) {
        hotkey::run_helper();
        return Ok(());
    }
    let updated = std::env::args().any(|a| a == "--updated");
    let mut _instance = unsafe { CreateMutexW(None, true, w!("Local\\dev.openray.app.instance")) };
    // After an update the previous process is still exiting, so wait for it to release the instance.
    for _ in 0..50 {
        if !updated || unsafe { GetLastError() } != ERROR_ALREADY_EXISTS {
            break;
        }
        if let Ok(handle) = _instance {
            let _ = unsafe { windows::Win32::Foundation::CloseHandle(handle) };
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
        _instance = unsafe { CreateMutexW(None, true, w!("Local\\dev.openray.app.instance")) };
    }
    if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        if let Ok(event) = unsafe { CreateEventW(None, false, false, w!("Local\\dev.openray.app.show")) } {
            let _ = unsafe { SetEvent(event) };
        }
        return Ok(());
    }
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Openray")
            .with_inner_size([launcher::WIDTH, launcher::HEIGHT])
            .with_position([-32000.0, -32000.0])
            .with_decorations(false)
            .with_resizable(false)
            .with_taskbar(false),
        ..Default::default()
    };
    eframe::run_native(
        "Openray",
        options,
        Box::new(|cc| {
            theme::install(&cc.egui_ctx);
            cc.egui_ctx.add_image_loader(std::sync::Arc::new(icons::AppIconLoader::default()));
            if let Some(hwnd) = launcher::hwnd_of(cc) {
                launcher::init(hwnd);
            }
            let (signals, receiver) = channel();
            *focus_ui::SIGNALS.lock().unwrap_or_else(|e| e.into_inner()) = Some(signals.clone());
            let ctx = cc.egui_ctx.clone();
            let hotkey_signals = signals.clone();
            let hotkey_error = hotkey::install(Box::new(move |id| {
                let _ = hotkey_signals.send(Signal::Hotkey(id));
                ctx.request_repaint();
            }))
            .err();
            listen_for_second_instance(cc.egui_ctx.clone(), signals.clone());
            let tray = setup_tray(&cc.egui_ctx, signals.clone());
            if std::env::args().any(|a| a == "--show") {
                let _ = signals.send(Signal::Show);
            }
            updater::start(&cc.egui_ctx);
            let launcher = Launcher::new(&cc.egui_ctx, data_dir(), receiver, hotkey_error);
            Ok(Box::new(App { launcher, _tray: tray, tray_visible: true }))
        }),
    )
}
