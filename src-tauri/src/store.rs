use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

const HALF_LIFE_MS: f64 = 864e6;
const DATA_NAMES: [&str; 10] = ["quicklinks", "emoji", "notes", "focus", "snippets", "calculator-history", "run-commands", "script-commands", "screenshots", "dictionary"];

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum ShowOn {
    #[default]
    MouseScreen,
    ActiveScreen,
    PrimaryScreen,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum EscapeKeyBehavior {
    #[default]
    PopBackOrClose,
    CloseAndPopToRoot,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum Appearance {
    #[default]
    System,
    Dark,
    Light,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct CommandPrefs {
    pub alias: String,
    pub hotkey: String,
    pub disabled: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub hotkey: String,
    pub show_in_tray: bool,
    pub show_on: ShowOn,
    pub escape_key_behavior: EscapeKeyBehavior,
    pub escape_key_closes_window: bool,
    pub appearance: Appearance,
    pub theme_dark_id: String,
    pub theme_light_id: String,
    pub notch_enabled: bool,
    pub notch_show_in_fullscreen: bool,
    pub commands: HashMap<String, CommandPrefs>,
}

pub const DEFAULT_HOTKEYS: [(&str, &str); 2] = [("screenshots/region", "PrintScreen"), ("screenshots/window", "Alt+PrintScreen")];

pub fn add_default_hotkeys(settings: &mut Settings) {
    for (key, hotkey) in DEFAULT_HOTKEYS {
        let taken = settings.hotkey.eq_ignore_ascii_case(hotkey) || settings.commands.values().any(|p| p.hotkey.eq_ignore_ascii_case(hotkey));
        if !settings.commands.contains_key(key) && !taken {
            settings.commands.insert(key.into(), CommandPrefs { hotkey: hotkey.into(), ..Default::default() });
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            hotkey: "Alt+Space".into(),
            show_in_tray: true,
            show_on: ShowOn::default(),
            escape_key_behavior: EscapeKeyBehavior::default(),
            escape_key_closes_window: true,
            appearance: Appearance::default(),
            theme_dark_id: "bundled-raycast-dark".into(),
            theme_light_id: "bundled-raycast-light".into(),
            notch_enabled: true,
            notch_show_in_fullscreen: false,
            commands: HashMap::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct Visit {
    pub score: f64,
    pub last_visit_ms: u64,
}

pub fn decayed(visit: Visit, now_ms: u64) -> f64 {
    let elapsed = now_ms.saturating_sub(visit.last_visit_ms) as f64;
    (visit.score * 2f64.powf(-elapsed / HALF_LIFE_MS)).max(1.0)
}

pub fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

pub(crate) fn read_json<T: for<'de> Deserialize<'de> + Default>(path: &Path) -> T {
    let Ok(text) = fs::read_to_string(path) else { return T::default() };
    serde_json::from_str(text.trim_start_matches('\u{feff}')).unwrap_or_else(|e| {
        eprintln!("openray: {} is unreadable ({e}); keeping a .bak copy", path.display());
        let _ = fs::rename(path, path.with_extension("json.bak"));
        T::default()
    })
}

pub(crate) fn write_json<T: Serialize>(path: &Path, value: &T) -> std::io::Result<()> {
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, serde_json::to_vec_pretty(value)?)?;
    fs::rename(tmp, path)
}

pub struct Store {
    dir: PathBuf,
    settings: Mutex<Settings>,
    frecency: Mutex<HashMap<String, Visit>>,
    favorites: Mutex<Vec<String>>,
}

impl Store {
    pub fn load(dir: PathBuf) -> Self {
        let _ = fs::create_dir_all(&dir);
        let settings_path = dir.join("settings.json");
        let mut settings: Settings = read_json(&settings_path);
        add_default_hotkeys(&mut settings);
        if !settings_path.exists() {
            let _ = write_json(&settings_path, &settings);
        }
        let frecency = Mutex::new(read_json(&dir.join("frecency.json")));
        let favorites = Mutex::new(read_json(&dir.join("favorites.json")));
        Self { dir, settings: Mutex::new(settings), frecency, favorites }
    }

    pub fn settings(&self) -> Settings {
        self.settings.lock().unwrap().clone()
    }

    pub fn set_settings(&self, value: Settings) -> std::io::Result<()> {
        let mut settings = self.settings.lock().unwrap();
        write_json(&self.dir.join("settings.json"), &value)?;
        *settings = value;
        Ok(())
    }

    pub fn visit(&self, key: &str, now_ms: u64) -> std::io::Result<f64> {
        let mut map = self.frecency.lock().unwrap();
        let score = map.get(key).map_or(1.0, |v| decayed(*v, now_ms) + 1.0);
        map.insert(key.to_owned(), Visit { score, last_visit_ms: now_ms });
        write_json(&self.dir.join("frecency.json"), &*map)?;
        Ok(score)
    }

    pub fn reset(&self, key: &str) -> std::io::Result<()> {
        let mut map = self.frecency.lock().unwrap();
        if map.remove(key).is_some() {
            write_json(&self.dir.join("frecency.json"), &*map)?;
        }
        Ok(())
    }

    pub fn favorites(&self) -> Vec<String> {
        self.favorites.lock().unwrap().clone()
    }

    pub fn set_favorites(&self, keys: Vec<String>) -> std::io::Result<()> {
        let mut favorites = self.favorites.lock().unwrap();
        write_json(&self.dir.join("favorites.json"), &keys)?;
        *favorites = keys;
        Ok(())
    }

    pub fn data(&self, name: &str) -> Result<serde_json::Value, String> {
        Ok(read_json(&self.data_path(name)?))
    }

    pub fn set_data(&self, name: &str, value: &serde_json::Value) -> Result<(), String> {
        write_json(&self.data_path(name)?, value).map_err(|e| e.to_string())
    }

    fn data_path(&self, name: &str) -> Result<PathBuf, String> {
        DATA_NAMES.contains(&name).then(|| self.dir.join(format!("{name}.json"))).ok_or_else(|| format!("unknown data file {name}"))
    }

    pub fn scores(&self, now_ms: u64) -> HashMap<String, f64> {
        let map = self.frecency.lock().unwrap();
        map.iter().map(|(k, v)| (k.clone(), decayed(*v, now_ms))).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DAY: u64 = 86_400_000;

    #[test]
    fn score_halves_every_ten_days_with_floor_one() {
        let v = Visit { score: 8.0, last_visit_ms: 0 };
        assert_eq!(decayed(v, 0), 8.0);
        assert!((decayed(v, 10 * DAY) - 4.0).abs() < 1e-9);
        assert!((decayed(v, 20 * DAY) - 2.0).abs() < 1e-9);
        assert_eq!(decayed(v, 100 * DAY), 1.0);
    }

    #[test]
    fn visits_accumulate_and_persist_across_reload() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::load(dir.path().to_path_buf());
        assert_eq!(store.visit("applications/a", 0).unwrap(), 1.0);
        assert_eq!(store.visit("applications/a", 0).unwrap(), 2.0);
        store.visit("applications/b", 0).unwrap();

        let reloaded = Store::load(dir.path().to_path_buf());
        let scores = reloaded.scores(0);
        assert_eq!(scores["applications/a"], 2.0);
        assert_eq!(scores["applications/b"], 1.0);
        assert!((reloaded.visit("applications/a", 10 * DAY).unwrap() - 2.0).abs() < 1e-9);
    }

    #[test]
    fn writes_default_settings_on_first_run_and_reads_edits() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(Store::load(dir.path().to_path_buf()).settings().hotkey, "Alt+Space");
        fs::write(dir.path().join("settings.json"), r#"{"hotkey":"Ctrl+Space","showOn":"primary-screen"}"#).unwrap();
        let settings = Store::load(dir.path().to_path_buf()).settings();
        assert_eq!(settings.hotkey, "Ctrl+Space");
        assert_eq!(settings.show_on, ShowOn::PrimaryScreen);
        assert!(settings.show_in_tray && settings.escape_key_closes_window);
    }

    #[test]
    fn saved_settings_survive_reload() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::load(dir.path().to_path_buf());
        let mut settings = store.settings();
        settings.commands.insert("applications/a".into(), CommandPrefs { alias: "a".into(), hotkey: "Ctrl+Alt+A".into(), disabled: true });
        settings.escape_key_behavior = EscapeKeyBehavior::CloseAndPopToRoot;
        store.set_settings(settings.clone()).unwrap();
        assert_eq!(Store::load(dir.path().to_path_buf()).settings(), settings);
    }

    #[test]
    fn adds_print_screen_defaults_without_overriding_choices() {
        let mut settings = Settings::default();
        add_default_hotkeys(&mut settings);
        assert_eq!(settings.commands["screenshots/region"].hotkey, "PrintScreen");
        assert_eq!(settings.commands["screenshots/window"].hotkey, "Alt+PrintScreen");
        let mut cleared = Settings::default();
        cleared.commands.insert("screenshots/region".into(), CommandPrefs::default());
        cleared.commands.insert("applications/x".into(), CommandPrefs { hotkey: "alt+printscreen".into(), ..Default::default() });
        add_default_hotkeys(&mut cleared);
        assert_eq!(cleared.commands["screenshots/region"].hotkey, "");
        assert!(!cleared.commands.contains_key("screenshots/window"));
    }

    #[test]
    fn files_saved_with_a_byte_order_mark_still_load() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("favorites.json"), "\u{feff}[\"applications/a\"]").unwrap();
        assert_eq!(Store::load(dir.path().to_path_buf()).favorites(), ["applications/a"]);
    }

    #[test]
    fn corrupt_file_is_backed_up_not_lost() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("frecency.json"), "{not json").unwrap();
        let store = Store::load(dir.path().to_path_buf());
        assert!(store.scores(0).is_empty());
        assert_eq!(fs::read_to_string(dir.path().join("frecency.json.bak")).unwrap(), "{not json");
    }

    #[test]
    fn reset_removes_ranking_and_persists() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::load(dir.path().to_path_buf());
        store.visit("applications/a", 0).unwrap();
        store.visit("applications/b", 0).unwrap();
        store.reset("applications/a").unwrap();
        store.reset("applications/missing").unwrap();
        let scores = Store::load(dir.path().to_path_buf()).scores(0);
        assert!(!scores.contains_key("applications/a"));
        assert_eq!(scores["applications/b"], 1.0);
    }

    #[test]
    fn data_files_round_trip_and_reject_unknown_names() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::load(dir.path().to_path_buf());
        assert_eq!(store.data("quicklinks").unwrap(), serde_json::Value::Null);
        store.set_data("quicklinks", &serde_json::json!([{"name": "google"}])).unwrap();
        assert_eq!(Store::load(dir.path().to_path_buf()).data("quicklinks").unwrap()[0]["name"], "google");
        assert!(store.set_data("../settings", &serde_json::Value::Null).is_err());
    }

    #[test]
    fn favorites_keep_their_order_across_reload() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::load(dir.path().to_path_buf());
        assert!(store.favorites().is_empty());
        store.set_favorites(vec!["applications/b".into(), "applications/a".into()]).unwrap();
        assert_eq!(Store::load(dir.path().to_path_buf()).favorites(), ["applications/b", "applications/a"]);
    }
}
