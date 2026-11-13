pub mod calculator;
mod calendar;
pub mod clipboard_history;
mod contacts;
mod dictionary;
mod emoji;
mod file_search;
pub mod flowkey;
mod focus;
mod placeholders;
mod quicklinks;
mod run;
mod screenshots;
mod script_commands;
pub mod snippets;
mod games;
mod menu_items;
mod sound;
mod translator;
mod system;
mod system_pages;
mod web;
mod windows;

pub use dictionary::{dictionary_language, set_dictionary_language};
pub use focus::{focus_preferences, save_focus_preferences, session_changed, session_tick, Preferences as FocusPreferences};
use crate::apps::InstalledApp;
use crate::extension::{Action, Command, Extension, Icon, Request, Shortcut, Tone};
use egui::Key;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub static APPS: Mutex<Vec<InstalledApp>> = Mutex::new(Vec::new());
static SCANNED: Mutex<Option<Instant>> = Mutex::new(None);
pub static GENERATION: AtomicU64 = AtomicU64::new(0);
const APP_TTL: Duration = Duration::from_secs(20);

pub fn rescan_apps(ctx: &egui::Context) {
    {
        let mut scanned = SCANNED.lock().unwrap_or_else(|e| e.into_inner());
        if scanned.is_some_and(|at| at.elapsed() < APP_TTL) {
            return;
        }
        *scanned = Some(Instant::now());
    }
    let ctx = ctx.clone();
    std::thread::spawn(move || {
        let games = crate::games::steam_games();
        let apps: Vec<InstalledApp> = crate::apps::scan(&crate::apps::start_menu_roots(), crate::apps::packaged_apps()).into_iter().filter(|a| !(a.path.to_lowercase().ends_with(".url") && crate::games::is_steam_shortcut(&a.path, &games))).collect();
        let pages = crate::settings_pages::system_pages();
        *games::LAUNCHERS.lock().unwrap_or_else(|e| e.into_inner()) = crate::games::launchers();
        let mut changed = false;
        {
            let mut current = games::GAMES.lock().unwrap_or_else(|e| e.into_inner());
            if *current != games {
                *current = games;
                changed = true;
            }
        }
        {
            let mut current = APPS.lock().unwrap_or_else(|e| e.into_inner());
            if *current != apps {
                *current = apps;
                changed = true;
            }
        }
        {
            let mut current = system_pages::PAGES.lock().unwrap_or_else(|e| e.into_inner());
            let same = current.len() == pages.len() && current.iter().zip(&pages).all(|(a, b)| a.path == b.path && a.name == b.name);
            if !same {
                *current = pages;
                changed = true;
            }
        }
        if changed {
            GENERATION.fetch_add(1, Ordering::SeqCst);
            ctx.request_repaint();
        }
    });
}

pub fn command(extension_id: &'static str, id: &str, title: &str, icon: Icon, kind: &'static str, primary: &'static str, run: impl Fn(&mut crate::extension::Host) -> Result<(), String> + 'static) -> Command {
    Command {
        key: format!("{extension_id}/{id}"),
        title: title.into(),
        subtitle: None,
        icon,
        keywords: Vec::new(),
        kind,
        primary_action_title: primary,
        confirm: None,
        actions: Vec::new(),
        run: Rc::new(run),
        view: None,
        fallback: None,
        query: None,
        argument: None,
    }
}

pub struct Applications;

fn app_actions(app: &InstalledApp) -> Vec<Vec<Action>> {
    let (name, path) = (app.name.clone(), app.path.clone());
    let copy_name = Action::new("copy-name", "Copy Name", move |host| {
        host.copy(&name, "name");
        Ok(())
    })
    .icon("clipboard")
    .keep_open();
    if app.path.starts_with(r"shell:AppsFolder\") {
        return vec![vec![], vec![copy_name]];
    }
    let (admin, explorer, copy) = (path.clone(), path.clone(), path);
    vec![
        vec![
            Action::new("run-as-admin", "Run as Administrator", move |_| crate::apps::launch(&admin, true)).icon("shield").shortcut(Shortcut::key(Key::Enter).ctrl().shift()),
            Action::new("show-in-explorer", "Show in File Explorer", move |_| crate::apps::show_in_explorer(&explorer)).icon("folder"),
        ],
        vec![
            Action::new("copy-path", "Copy Path", move |host| {
                host.copy(&copy, "path");
                Ok(())
            })
            .icon("clipboard")
            .shortcut(Shortcut::key(Key::C).ctrl().alt())
            .keep_open(),
            copy_name,
        ],
    ]
}

impl Extension for Applications {
    fn id(&self) -> &'static str {
        "applications"
    }

    fn commands(&self) -> Vec<Command> {
        let apps = APPS.lock().unwrap_or_else(|e| e.into_inner()).clone();
        apps.iter()
            .map(|app| {
                let path = app.path.clone();
                let mut cmd = command(self.id(), &app.path, &app.name, Icon::File(app.path.clone()), "Application", "Open Application", move |_| crate::apps::launch(&path, false));
                cmd.actions = app_actions(app);
                cmd
            })
            .collect()
    }
}

pub struct Core;

impl Extension for Core {
    fn id(&self) -> &'static str {
        "openray-core"
    }

    fn commands(&self) -> Vec<Command> {
        let mut settings = command(self.id(), "settings", "Settings", Icon::Tile("settings", Tone::Slate), "Command", "Open Command", |host| {
            host.requests.push(Request::OpenSettings);
            Ok(())
        });
        settings.subtitle = Some("Openray".into());
        settings.keywords = ["preferences", "hotkey", "options", "preferências", "atalhos", "opções", "configurações"].map(String::from).to_vec();
        let mut quit = command(self.id(), "quit", "Quit Openray", Icon::Tile("power", Tone::Red), "Command", "Run Command", |host| {
            host.requests.push(Request::Quit);
            Ok(())
        });
        quit.subtitle = Some("Openray".into());
        quit.keywords = ["exit", "close", "sair", "fechar", "encerrar"].map(String::from).to_vec();
        let mut check = command(self.id(), "check-for-updates", "Check for Updates", Icon::Tile("download", Tone::Blue), "Command", "Check for Updates", |host| {
            crate::updater::check_now(&host.ctx);
            host.hud(Ok("Checking for updates…".into()));
            Ok(())
        });
        check.subtitle = Some("Openray".into());
        check.keywords = ["update", "upgrade", "version", "atualizar", "atualização", "versão"].map(String::from).to_vec();
        let mut list = vec![settings, check, quit];
        if let crate::updater::State::Ready(version) = crate::updater::state() {
            let mut update = command(self.id(), "restart-to-update", &format!("Restart to Update to {version}"), Icon::Tile("download", Tone::Green), "Command", "Restart to Update", |host| {
                crate::updater::apply()?;
                host.requests.push(Request::Quit);
                Ok(())
            });
            update.subtitle = Some("Openray".into());
            update.keywords = ["update", "upgrade", "restart", "atualizar", "reiniciar"].map(String::from).to_vec();
            list.insert(0, update);
        }
        list
    }
}

pub struct Notes;

impl Extension for Notes {
    fn id(&self) -> &'static str {
        "notes"
    }

    fn commands(&self) -> Vec<Command> {
        let icon = Icon::Tile("notebook-pen", Tone::Pink);
        let mut open = command(self.id(), "open", "Openray Notes", icon.clone(), "Command", "Open Command", |host| {
            host.requests.push(Request::OpenNotes { create: false });
            Ok(())
        });
        open.keywords = ["notes", "scratchpad", "markdown", "notas"].map(String::from).to_vec();
        let mut create = command(self.id(), "create", "Create Note", icon, "Command", "Open Command", |host| {
            host.requests.push(Request::OpenNotes { create: true });
            Ok(())
        });
        create.keywords = ["notes", "new", "notas", "criar nota"].map(String::from).to_vec();
        for cmd in [&mut open, &mut create] {
            cmd.subtitle = Some("Notes".into());
        }
        vec![open, create]
    }
}

pub fn built_in() -> Vec<Box<dyn Extension>> {
    vec![Box::new(Applications), Box::new(calculator::Calculator), Box::new(calendar::CalendarExtension), Box::new(clipboard_history::ClipboardHistory), Box::new(contacts::Contacts), Box::new(dictionary::Dictionary), Box::new(emoji::EmojiPicker), Box::new(file_search::FileSearch), Box::new(focus::Focus), Box::new(games::Games), Box::new(quicklinks::Quicklinks), Box::new(run::Run), Box::new(screenshots::Screenshots), Box::new(script_commands::ScriptCommands), Box::new(snippets::Snippets), Box::new(menu_items::MenuItems), Box::new(windows::Navigation), Box::new(Notes), Box::new(system::System), Box::new(system_pages::SystemPages), Box::new(translator::Translator), Box::new(web::Urls), Box::new(web::WebSearches), Box::new(windows::WindowManagement), Box::new(Core)]
}

pub fn load(extensions: &[Box<dyn Extension>], settings: &crate::store::Settings) -> Vec<Command> {
    let disabled = |key: &str| settings.commands.get(key).is_some_and(|p| p.disabled);
    extensions.iter().filter(|e| !disabled(e.id())).flat_map(|e| e.commands()).filter(|c| !disabled(&c.key)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_extensions_and_commands_are_skipped() {
        let mut settings = crate::store::Settings::default();
        settings.commands.insert("notes/create".into(), crate::store::CommandPrefs { disabled: true, ..Default::default() });
        let keys: Vec<String> = load(&built_in(), &settings).into_iter().map(|c| c.key).collect();
        assert!(keys.contains(&"notes/open".to_string()) && !keys.contains(&"notes/create".to_string()));
        settings.commands.insert("openray-core".into(), crate::store::CommandPrefs { disabled: true, ..Default::default() });
        assert!(!load(&built_in(), &settings).iter().any(|c| c.key.starts_with("openray-core/")));
    }
}
