use super::command;
use crate::extension::{Action, Command, Empty, Extension, Icon, Item, Load, Section, Shortcut, Tone, View};
use crate::window_switcher::{self, WindowEntry};
use egui::Key;
use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;

const LAYOUTS: &[(&str, &str, &str, &[&str])] = &[
    ("maximize", "Maximize", "maximize-2", &[]),
    ("center", "Center", "app-window", &[]),
    ("left-half", "Left Half", "panel-left", &[]),
    ("right-half", "Right Half", "panel-right", &[]),
    ("top-half", "Top Half", "panel-top", &[]),
    ("bottom-half", "Bottom Half", "panel-bottom", &[]),
    ("first-third", "First Third", "columns-3", &["left"]),
    ("center-third", "Center Third", "columns-3", &[]),
    ("last-third", "Last Third", "columns-3", &["right"]),
    ("first-two-thirds", "First Two Thirds", "columns-2", &["left"]),
    ("center-two-thirds", "Center Two Thirds", "columns-2", &[]),
    ("last-two-thirds", "Last Two Thirds", "columns-2", &["right"]),
    ("top-third", "Top Third", "rows-3", &[]),
    ("middle-third", "Middle Third", "rows-3", &[]),
    ("bottom-third", "Bottom Third", "rows-3", &[]),
    ("top-two-thirds", "Top Two Thirds", "rows-3", &[]),
    ("bottom-two-thirds", "Bottom Two Thirds", "rows-3", &[]),
    ("restore", "Restore", "undo-2", &["undo", "previous"]),
    ("minimize", "Minimize", "minimize", &["hide", "taskbar"]),
    ("minimize-on-display", "Minimize on This Display", "minimize", &["show desktop", "minimize all", "hide", "monitor", "screen"]),
    ("maximize-height", "Maximize Height", "move-vertical", &["vertical"]),
    ("maximize-width", "Maximize Width", "move-horizontal", &["horizontal"]),
    ("almost-maximize", "Almost Maximize", "maximize-2", &[]),
    ("reasonable-size", "Reasonable Size", "square-dashed", &[]),
    ("center-half", "Center Half", "columns-2", &[]),
    ("first-three-fourths", "First Three Fourths", "columns-4", &["left"]),
    ("center-three-fourths", "Center Three Fourths", "columns-4", &[]),
    ("last-three-fourths", "Last Three Fourths", "columns-4", &["right"]),
    ("top-three-fourths", "Top Three Fourths", "rows-4", &[]),
    ("bottom-three-fourths", "Bottom Three Fourths", "rows-4", &[]),
    ("first-fourth", "First Fourth", "columns-4", &["left"]),
    ("second-fourth", "Second Fourth", "columns-4", &[]),
    ("third-fourth", "Third Fourth", "columns-4", &[]),
    ("last-fourth", "Last Fourth", "columns-4", &["right"]),
    ("top-first-fourth", "Top First Fourth", "rows-4", &[]),
    ("top-second-fourth", "Top Second Fourth", "rows-4", &[]),
    ("top-third-fourth", "Top Third Fourth", "rows-4", &[]),
    ("top-last-fourth", "Top Last Fourth", "rows-4", &[]),
    ("top-center-two-thirds", "Top Center Two Thirds", "columns-2", &[]),
    ("bottom-center-two-thirds", "Bottom Center Two Thirds", "columns-2", &[]),
    ("top-left-quarter", "Top Left Quarter", "grid-2x2", &["corner"]),
    ("top-right-quarter", "Top Right Quarter", "grid-2x2", &["corner"]),
    ("bottom-left-quarter", "Bottom Left Quarter", "grid-2x2", &["corner"]),
    ("bottom-right-quarter", "Bottom Right Quarter", "grid-2x2", &["corner"]),
    ("top-left-sixth", "Top Left Sixth", "columns-3", &[]),
    ("top-center-sixth", "Top Center Sixth", "columns-3", &[]),
    ("top-right-sixth", "Top Right Sixth", "columns-3", &[]),
    ("bottom-left-sixth", "Bottom Left Sixth", "columns-3", &[]),
    ("bottom-center-sixth", "Bottom Center Sixth", "columns-3", &[]),
    ("bottom-right-sixth", "Bottom Right Sixth", "columns-3", &[]),
    ("move-left", "Move Left", "arrow-left-to-line", &[]),
    ("move-right", "Move Right", "arrow-right-to-line", &[]),
    ("move-top", "Move Top", "arrow-up-to-line", &["up"]),
    ("move-bottom", "Move Bottom", "arrow-down-to-line", &["down"]),
    ("make-larger", "Make Larger", "expand", &["bigger", "grow"]),
    ("make-smaller", "Make Smaller", "shrink", &["shrink"]),
    ("toggle-always-on-top", "Toggle Always on Top", "pin", &["pin", "float", "topmost"]),
    ("focus-next-app-window", "Focus Next App Window", "layers-2", &["cycle", "switch"]),
    ("focus-previous-app-window", "Focus Previous App Window", "layers-2", &["cycle", "switch"]),
    ("previous-display", "Move to Previous Display", "arrow-left-from-line", &["desktop", "monitor", "screen"]),
    ("next-display", "Move to Next Display", "arrow-right-from-line", &["desktop", "monitor", "screen"]),
];

pub struct WindowManagement;

impl Extension for WindowManagement {
    fn id(&self) -> &'static str {
        "window-management"
    }

    fn commands(&self) -> Vec<Command> {
        LAYOUTS
            .iter()
            .map(|&(id, title, glyph, keywords)| {
                let mut cmd = command(self.id(), id, title, Icon::Tile(glyph, Tone::Blue), "Command", "Run Command", move |_| crate::window_layout::apply(id, Some(crate::launcher::monitor())));
                cmd.subtitle = Some("Window Management".into());
                cmd.keywords = ["window", "resize", "move", "janela", "janelas", "redimensionar", "mover", "organizar"].iter().chain(keywords).map(|k| k.to_string()).collect();
                cmd
            })
            .collect()
    }
}

pub struct Navigation;

impl Extension for Navigation {
    fn id(&self) -> &'static str {
        "navigation"
    }

    fn commands(&self) -> Vec<Command> {
        let mut cmd = command(self.id(), "switch-windows", "Switch Windows", Icon::Tile("app-window", Tone::Blue), "Command", "Open Command", |_| Ok(()));
        cmd.subtitle = Some("Navigation".into());
        cmd.keywords = ["windows switch", "window switcher", "janelas", "alternador de janelas"].map(String::from).to_vec();
        cmd.view = Some(Rc::new(|ctx| Box::new(SwitchWindows::new(ctx))));
        vec![cmd]
    }
}

pub struct SwitchWindows {
    load: Load<Arc<Vec<WindowEntry>>>,
    dirty: Rc<Cell<bool>>,
    ctx: egui::Context,
}

fn list_windows() -> Result<Arc<Vec<WindowEntry>>, String> {
    Ok(Arc::new(window_switcher::list()))
}

impl SwitchWindows {
    fn new(ctx: &egui::Context) -> Self {
        Self { load: Load::start(ctx, list_windows), dirty: Rc::new(Cell::new(false)), ctx: ctx.clone() }
    }
}

fn act(id: isize, action: &'static str) -> impl Fn(&mut crate::extension::Host) -> Result<(), String> {
    move |host| {
        let hidden = Cell::new(None);
        let result = window_switcher::act(&|restore| hidden.set(Some(restore)), id, action);
        if let Some(restore) = hidden.get() {
            host.hide(restore);
        }
        result
    }
}

fn window_actions(target: &WindowEntry, dirty: Rc<Cell<bool>>) -> Vec<Vec<Action>> {
    let id = target.id;
    let mut groups = vec![vec![
        Action::new("switch", "Switch to Window", act(id, "switch")).icon("app-window"),
        Action::new("minimize", if target.minimized { "Unminimize Window" } else { "Minimize Window" }, act(id, if target.minimized { "switch" } else { "minimize" })).icon("circle-minus").shortcut(Shortcut::key(Key::M).ctrl()),
        Action::new("close", "Close Window", act(id, "close")).icon("circle-x").destructive().shortcut(Shortcut::key(Key::W).ctrl().shift()),
        Action::new("toggle-maximize", if target.maximized { "Restore" } else { "Maximize" }, act(id, "toggle-maximize")).icon(if target.maximized { "minimize-2" } else { "maximize-2" }).shortcut(Shortcut::key(Key::F).ctrl()),
    ]];
    if target.app_name.is_some() {
        let quit = |action: &'static str| {
            let dirty = dirty.clone();
            move |host: &mut crate::extension::Host| {
                dirty.set(true);
                act(id, action)(host)
            }
        };
        groups.push(vec![
            Action::new("quit", "Quit Application", quit("quit")).icon("power").destructive().keep_open().shortcut(Shortcut::key(Key::Q).ctrl()),
            Action::new("force-quit", "Force Quit Application", quit("force-quit")).icon("power").destructive().keep_open().shortcut(Shortcut::key(Key::Q).ctrl().shift()),
        ]);
    }
    groups
}

impl View for SwitchWindows {
    fn placeholder(&self) -> String {
        "Search windows…".into()
    }

    fn footer(&self) -> (String, Icon) {
        ("Switch Windows".into(), Icon::Tile("app-window", Tone::Blue))
    }

    fn loading(&self) -> bool {
        self.load.get().is_none()
    }

    fn empty(&self) -> Empty {
        Empty { title: "No Results".into(), description: None }
    }

    fn tab_cycles(&self) -> bool {
        true
    }

    fn initial_selection(&self) -> Option<String> {
        self.load.get().and_then(Result::ok).and_then(|w| w.get(1).map(|w| w.id.to_string()))
    }

    fn sections(&mut self) -> Vec<Section> {
        if self.dirty.replace(false) {
            self.load = Load::start(&self.ctx, list_windows);
        }
        let Some(Ok(windows)) = self.load.get() else { return Vec::new() };
        vec![Section {
            title: String::new(),
            items: windows
                .iter()
                .map(|w| Item {
                    id: w.id.to_string(),
                    title: w.title.clone(),
                    subtitle: w.app_name.clone(),
                    icon: w.path.clone().map_or(Icon::Glyph("app-window"), Icon::File),
                    accessories: Vec::new(),
                    keywords: w.app_name.iter().cloned().collect(),
                    actions: window_actions(w, self.dirty.clone()),
                })
                .collect(),
        }]
    }
}
