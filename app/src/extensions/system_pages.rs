use super::command;
use crate::extension::{Action, Command, Extension, Icon, Shortcut};
use crate::settings_pages::SystemPage;
use egui::Key;
use std::sync::Mutex;

pub static PAGES: Mutex<Vec<SystemPage>> = Mutex::new(Vec::new());

const SETTINGS_APP: &str = r"shell:AppsFolder\windows.immersivecontrolpanel_cw5n1h2txyewy!microsoft.windows.immersivecontrolpanel";

pub struct SystemPages;

impl Extension for SystemPages {
    fn id(&self) -> &'static str {
        "system-pages"
    }

    fn commands(&self) -> Vec<Command> {
        let pages = PAGES.lock().unwrap_or_else(|e| e.into_inner());
        pages
            .iter()
            .map(|page| {
                let settings = page.kind == "System Settings";
                let path = page.path.clone();
                let icon = Icon::File(if settings { SETTINGS_APP.into() } else { page.path.clone() });
                let mut cmd = command(self.id(), &format!("{}#{}", page.path, page.name), &page.name, icon, page.kind, if settings { "Open Settings Page" } else { "Open" }, move |_| crate::apps::launch(&path, false));
                let base: &[&str] = if settings { &["settings", "configurações"] } else { &["control panel", "painel de controle"] };
                cmd.keywords = base.iter().map(|k| k.to_string()).chain(page.keywords.split(';').filter(|k| !k.is_empty()).map(str::to_owned)).collect();
                let name = page.name.clone();
                cmd.actions = vec![vec![], vec![Action::new("copy-name", "Copy Name", move |host| {
                    host.copy_and_close(&name, "Copied Name to Clipboard");
                    Ok(())
                })
                .icon("clipboard")
                .shortcut(Shortcut::key(Key::C).alt().shift())]];
                cmd
            })
            .collect()
    }
}
