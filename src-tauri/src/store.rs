use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

const HALF_LIFE_MS: f64 = 864e6;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Settings {
    pub hotkey: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self { hotkey: "Alt+Space".into() }
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

fn read_json<T: for<'de> Deserialize<'de> + Default>(path: &Path) -> T {
    let Ok(text) = fs::read_to_string(path) else { return T::default() };
    serde_json::from_str(&text).unwrap_or_else(|e| {
        eprintln!("openray: {} is unreadable ({e}); keeping a .bak copy", path.display());
        let _ = fs::rename(path, path.with_extension("json.bak"));
        T::default()
    })
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> std::io::Result<()> {
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, serde_json::to_vec_pretty(value)?)?;
    fs::rename(tmp, path)
}

pub struct Store {
    dir: PathBuf,
    pub settings: Settings,
    frecency: Mutex<HashMap<String, Visit>>,
    favorites: Mutex<Vec<String>>,
}

impl Store {
    pub fn load(dir: PathBuf) -> Self {
        let _ = fs::create_dir_all(&dir);
        let settings_path = dir.join("settings.json");
        let settings: Settings = read_json(&settings_path);
        if !settings_path.exists() {
            let _ = write_json(&settings_path, &settings);
        }
        let frecency = Mutex::new(read_json(&dir.join("frecency.json")));
        let favorites = Mutex::new(read_json(&dir.join("favorites.json")));
        Self { dir, settings, frecency, favorites }
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
        assert_eq!(Store::load(dir.path().to_path_buf()).settings.hotkey, "Alt+Space");
        fs::write(dir.path().join("settings.json"), r#"{"hotkey":"Ctrl+Space"}"#).unwrap();
        assert_eq!(Store::load(dir.path().to_path_buf()).settings.hotkey, "Ctrl+Space");
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
    fn favorites_keep_their_order_across_reload() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::load(dir.path().to_path_buf());
        assert!(store.favorites().is_empty());
        store.set_favorites(vec!["applications/b".into(), "applications/a".into()]).unwrap();
        assert_eq!(Store::load(dir.path().to_path_buf()).favorites(), ["applications/b", "applications/a"]);
    }
}
