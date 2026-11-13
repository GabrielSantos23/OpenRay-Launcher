use super::command;
use crate::extension::{Accessory, Action, Command, Empty, Extension, Icon, Item, Load, Section, Shortcut, Tone, View};
use crate::menu_items::{self, MenuItem};
use egui::Key;
use std::rc::Rc;
use std::sync::Arc;
use windows::Win32::Foundation::HWND;

pub struct MenuItems;

impl Extension for MenuItems {
    fn id(&self) -> &'static str {
        "menu-items"
    }

    fn commands(&self) -> Vec<Command> {
        let mut cmd = command(self.id(), "search", "Search Menu Items", Icon::Tile("menu", Tone::Slate), "Command", "Open Command", |_| Ok(()));
        cmd.subtitle = Some("Menu Items".into());
        cmd.keywords = ["menu bar", "menu", "commands", "active app", "itens de menu", "barra de menus"].map(String::from).to_vec();
        cmd.view = Some(Rc::new(|ctx| Box::new(SearchMenuItems::new(ctx))));
        vec![cmd]
    }
}

struct SearchMenuItems {
    hwnd: isize,
    load: Load<Arc<(String, Vec<MenuItem>)>>,
}

impl SearchMenuItems {
    fn new(ctx: &egui::Context) -> Self {
        let hwnd = crate::launcher::previous_window().0 as isize;
        let load = Load::start(ctx, move || {
            let hwnd = HWND(hwnd as _);
            let app = crate::clipboard::window_app(hwnd).map(|(name, _)| name).unwrap_or_default();
            Ok(Arc::new((app, menu_items::list(hwnd))))
        });
        Self { hwnd, load }
    }

    fn app(&self) -> String {
        self.load.get().and_then(Result::ok).map(|d| d.0.clone()).unwrap_or_default()
    }
}

impl View for SearchMenuItems {
    fn placeholder(&self) -> String {
        match self.app() {
            app if app.is_empty() => "Search menu items…".into(),
            app => format!("Search menu items of {app}…"),
        }
    }

    fn footer(&self) -> (String, Icon) {
        ("Search Menu Items".into(), Icon::Tile("menu", Tone::Slate))
    }

    fn loading(&self) -> bool {
        self.load.get().is_none()
    }

    fn empty(&self) -> Empty {
        let app = self.app();
        Empty { title: "No Menu Items".into(), description: Some(format!("{} has no classic menu bar that Openray can read.", if app.is_empty() { "This app" } else { &app })) }
    }

    fn sections(&mut self) -> Vec<Section> {
        let Some(Ok(data)) = self.load.get() else { return Vec::new() };
        let mut sections: Vec<Section> = Vec::new();
        for item in &data.1 {
            let title = item.path.first().cloned().unwrap_or_else(|| "Menu".into());
            let location = item.path.iter().skip(1).cloned().collect::<Vec<_>>().join(" › ");
            let (hwnd, id) = (self.hwnd, item.id);
            let full = item.path.iter().chain([&item.title]).cloned().collect::<Vec<_>>().join(" › ");
            let entry = Item {
                id: format!("{}:{}:{}", item.id, item.path.join("/"), item.title),
                title: if item.checked { format!("{} ✓", item.title) } else { item.title.clone() },
                subtitle: (!location.is_empty()).then_some(location),
                icon: Icon::Glyph("menu"),
                accessories: if item.shortcut.is_empty() { Vec::new() } else { vec![Accessory::Text(item.shortcut.clone())] },
                keywords: item.path.clone(),
                actions: vec![vec![
                    Action::new("run", "Run Menu Item", move |host| {
                        host.hide(false);
                        menu_items::invoke(HWND(hwnd as _), id)
                    })
                    .icon("play"),
                    Action::new("copy", "Copy Menu Path", move |host| {
                        host.copy_and_close(&full, "Copied Menu Path");
                        Ok(())
                    })
                    .icon("clipboard")
                    .shortcut(Shortcut::key(Key::C).ctrl().shift()),
                ]],
            };
            match sections.iter_mut().find(|s| s.title == title) {
                Some(section) => section.items.push(entry),
                None => sections.push(Section { title, items: vec![entry] }),
            }
        }
        sections
    }
}
