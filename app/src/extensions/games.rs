use super::command;
use crate::extension::{Accessory, Action, Command, Empty, Extension, Icon, Item, Section, Shortcut, Tone, View};
use crate::games::{Game, Launcher};
use egui::Key;
use std::rc::Rc;
use std::sync::Mutex;

pub static GAMES: Mutex<Vec<Game>> = Mutex::new(Vec::new());
pub static LAUNCHERS: Mutex<Vec<Launcher>> = Mutex::new(Vec::new());

const ICON: Icon = Icon::Tile("gamepad-2", Tone::Green);

fn icon(game: &Game) -> Icon {
    match (&game.icon, &game.exe) {
        (Some(art), _) => Icon::Thumbnail(art.clone()),
        (None, Some(exe)) => Icon::File(exe.clone()),
        (None, None) => ICON,
    }
}

fn open(game: &Game) -> impl Fn(&mut crate::extension::Host) -> Result<(), String> + 'static {
    let url = game.launch.clone();
    move |_| crate::apps::open_url(&url, None)
}

fn game_actions(game: &Game) -> Vec<Vec<Action>> {
    let (dir, copy, name, uninstall) = (game.install_dir.clone(), game.install_dir.clone(), game.name.trim().to_owned(), game.uninstall.clone());
    let title = format!("Uninstall {name}?");
    let message = format!("{} will open to uninstall it.", game.launcher);
    vec![
        vec![Action::new("show-in-explorer", "Show in File Explorer", move |_| crate::apps::show_in_explorer(&dir)).icon("folder")],
        vec![
            Action::new("copy-path", "Copy Path", move |host| {
                host.copy_and_close(&copy, "Copied Path to Clipboard");
                Ok(())
            })
            .icon("clipboard")
            .shortcut(Shortcut::key(Key::C).ctrl().alt()),
            Action::new("copy-name", "Copy Name", move |host| {
                host.copy_and_close(&name, "Copied Name to Clipboard");
                Ok(())
            })
            .icon("clipboard")
            .shortcut(Shortcut::key(Key::C).alt().shift()),
        ],
        vec![Action::new("uninstall", "Uninstall Game", move |_| crate::apps::open_url(&uninstall, None)).icon("trash-2").destructive().confirm(&title, &message, "Uninstall")],
    ]
}

pub struct Games;

impl Extension for Games {
    fn id(&self) -> &'static str {
        "games"
    }

    fn commands(&self) -> Vec<Command> {
        let mut launcher = command(self.id(), "search-games", "Game Launcher", ICON, "Command", "Open Command", |_| Ok(()));
        launcher.subtitle = Some("Games".into());
        launcher.keywords = ["games", "steam", "play", "jogos", "jogar"].map(String::from).to_vec();
        launcher.view = Some(Rc::new(|_| Box::new(GameLauncher { query: String::new() })));
        let games = GAMES.lock().unwrap_or_else(|e| e.into_inner()).clone();
        std::iter::once(launcher)
            .chain(games.iter().map(|game| {
                let mut cmd = command(self.id(), &game.id, game.name.trim(), icon(game), "Game", "Open Game", open(game));
                cmd.keywords = ["game", "games", "jogo", "jogos", game.launcher].map(String::from).to_vec();
                cmd.actions = game_actions(game);
                cmd
            }))
            .collect()
    }
}

struct GameLauncher {
    query: String,
}

impl View for GameLauncher {
    fn placeholder(&self) -> String {
        "Search games…".into()
    }

    fn footer(&self) -> (String, Icon) {
        ("Game Launcher".into(), ICON)
    }

    fn set_query(&mut self, query: &str) {
        self.query = query.trim().to_owned();
    }

    fn empty(&self) -> Empty {
        if self.query.is_empty() {
            Empty { title: "No Games Found".into(), description: None }
        } else {
            Empty { title: "No Results".into(), description: Some("Try a different search term.".into()) }
        }
    }

    fn sections(&mut self) -> Vec<Section> {
        let games: Vec<Item> = GAMES
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .map(|game| {
                let mut actions = game_actions(game);
                actions[0].insert(0, Action::new("open", "Open Game", open(game)).icon("gamepad-2"));
                Item { id: game.id.clone(), title: game.name.trim().into(), subtitle: None, icon: icon(game), accessories: vec![Accessory::Text("Game".into())], keywords: vec![game.launcher.into()], actions }
            })
            .collect();
        let launchers: Vec<Item> = LAUNCHERS
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .map(|launcher| {
                let path = launcher.path.clone();
                Item {
                    id: launcher.path.clone(),
                    title: launcher.name.into(),
                    subtitle: None,
                    icon: Icon::File(launcher.path.clone()),
                    accessories: vec![Accessory::Text("Launcher".into())],
                    keywords: Vec::new(),
                    actions: vec![vec![Action::new("open", "Open Application", move |_| crate::apps::launch(&path, false)).icon("gamepad-2")]],
                }
            })
            .collect();
        if !self.query.is_empty() {
            return vec![Section { title: "Results".into(), items: games.into_iter().chain(launchers).collect() }];
        }
        let mut games = games;
        let rest = games.split_off(games.len().min(5));
        [("Suggestions", games), ("Games", rest), ("Game Launchers", launchers)].into_iter().filter(|(_, items)| !items.is_empty()).map(|(title, items)| Section { title: title.into(), items }).collect()
    }
}
