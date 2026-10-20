use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use windows::core::w;
use windows::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_SZ};

const TOOLS: [u32; 3] = [228_980, 1_070_560, 1_391_110];
const SKIPPED_EXES: [&str; 11] = ["unins", "crash", "redist", "setup", "vcredist", "dxsetup", "anticheat", "installer", "settings", "hlds", "server"];

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Game {
    pub id: String,
    pub name: String,
    pub launcher: &'static str,
    pub install_dir: String,
    pub exe: Option<String>,
    pub icon: Option<String>,
    pub launch: String,
    pub uninstall: String,
}

pub fn quoted_values(text: &str, key: &str) -> Vec<String> {
    let needle = format!("\"{key}\"");
    text.lines()
        .filter_map(|line| {
            let rest = line.trim().strip_prefix(needle.as_str())?;
            let value = rest.trim().strip_prefix('"')?.strip_suffix('"')?;
            Some(value.replace("\\\\", "\\"))
        })
        .collect()
}

fn first(text: &str, key: &str) -> Option<String> {
    quoted_values(text, key).into_iter().next()
}

pub fn parse_manifest(text: &str, library: &Path) -> Option<Game> {
    let appid: u32 = first(text, "appid")?.parse().ok()?;
    let name = first(text, "name")?;
    let installdir = first(text, "installdir")?;
    let flags: u32 = first(text, "StateFlags").and_then(|f| f.parse().ok()).unwrap_or(0);
    if flags & 4 == 0 || TOOLS.contains(&appid) || name.starts_with("Proton") || name.starts_with("Steam Linux Runtime") || name.starts_with("Steamworks") {
        return None;
    }
    let dir = library.join("steamapps").join("common").join(&installdir);
    Some(Game {
        id: format!("steam:{appid}"),
        name,
        launcher: "Steam",
        install_dir: dir.to_string_lossy().into_owned(),
        exe: None,
        icon: None,
        launch: format!("steam://rungameid/{appid}"),
        uninstall: format!("steam://uninstall/{appid}"),
    })
}

fn steam_icon(root: &Path, appid: &str) -> Option<String> {
    fs::read_dir(root.join("appcache").join("librarycache").join(appid)).ok()?.flatten().map(|e| e.path()).find(|p| {
        let stem = p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        p.extension().is_some_and(|e| e.eq_ignore_ascii_case("jpg")) && stem.len() == 40 && stem.chars().all(|c| c.is_ascii_hexdigit())
    }).map(|p| p.to_string_lossy().into_owned())
}

fn main_exe(dir: &Path) -> Option<String> {
    let mut best: Option<(u64, PathBuf)> = None;
    let mut visit = |folder: &Path| {
        for entry in fs::read_dir(folder).into_iter().flatten().flatten() {
            let path = entry.path();
            let name = path.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
            if !name.ends_with(".exe") || SKIPPED_EXES.iter().any(|s| name.contains(s)) {
                continue;
            }
            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
            if best.as_ref().is_none_or(|(b, _)| size > *b) {
                best = Some((size, path));
            }
        }
    };
    visit(dir);
    for sub in ["bin", "Bin", "Binaries\\Win64", "x64", "game", "Game"] {
        visit(&dir.join(sub));
    }
    best.map(|(_, p)| p.to_string_lossy().into_owned())
}

pub fn steam_path() -> Option<PathBuf> {
    let mut buffer = [0u16; 512];
    let mut size = (buffer.len() * 2) as u32;
    unsafe {
        RegGetValueW(HKEY_CURRENT_USER, w!(r"Software\Valve\Steam"), w!("SteamPath"), RRF_RT_REG_SZ, None, Some(buffer.as_mut_ptr() as _), Some(&mut size)).ok().ok()?;
    }
    let len = buffer.iter().position(|&c| c == 0).unwrap_or(0);
    let path = PathBuf::from(String::from_utf16_lossy(&buffer[..len]).replace('/', "\\"));
    path.exists().then_some(path)
}

pub fn steam_games() -> Vec<Game> {
    let Some(root) = steam_path() else { return Vec::new() };
    let mut libraries = vec![root.clone()];
    if let Ok(text) = fs::read_to_string(root.join("steamapps").join("libraryfolders.vdf")) {
        for path in quoted_values(&text, "path") {
            let path = PathBuf::from(path);
            if !libraries.iter().any(|l| l.to_string_lossy().eq_ignore_ascii_case(&path.to_string_lossy())) {
                libraries.push(path);
            }
        }
    }
    let mut games = Vec::new();
    for library in libraries {
        for entry in fs::read_dir(library.join("steamapps")).into_iter().flatten().flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if !(name.starts_with("appmanifest_") && name.ends_with(".acf")) {
                continue;
            }
            if let Some(mut game) = fs::read_to_string(entry.path()).ok().and_then(|t| parse_manifest(&t, &library)) {
                game.exe = main_exe(Path::new(&game.install_dir));
                game.icon = steam_icon(&root, game.id.trim_start_matches("steam:"));
                games.push(game);
            }
        }
    }
    games.sort_by_key(|g| g.name.to_lowercase());
    games
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Launcher {
    name: &'static str,
    path: String,
}

pub fn launchers() -> Vec<Launcher> {
    steam_path().map(|p| p.join("steam.exe")).filter(|p| p.exists()).map(|p| Launcher { name: "Steam", path: p.to_string_lossy().into_owned() }).into_iter().collect()
}

pub fn is_steam_shortcut(url_file: &str, games: &[Game]) -> bool {
    fs::read_to_string(url_file).ok().is_some_and(|text| {
        text.lines().filter_map(|l| l.trim().strip_prefix("URL=steam://rungameid/")).any(|id| games.iter().any(|g| g.id == format!("steam:{}", id.trim())))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST: &str = "\"AppState\"\n{\n\t\"appid\"\t\t\"1238860\"\n\t\"name\"\t\t\"Battlefield 4™ \"\n\t\"StateFlags\"\t\t\"4\"\n\t\"installdir\"\t\t\"Battlefield 4\"\n}\n";

    #[test]
    fn parses_installed_manifests_and_skips_tools() {
        let game = parse_manifest(MANIFEST, Path::new(r"E:\SteamLibrary")).unwrap();
        assert_eq!(game.id, "steam:1238860");
        assert_eq!(game.name, "Battlefield 4™ ");
        assert_eq!(game.install_dir, r"E:\SteamLibrary\steamapps\common\Battlefield 4");
        assert_eq!(game.launch, "steam://rungameid/1238860");
        assert!(parse_manifest(&MANIFEST.replace("\"4\"", "\"1026\""), Path::new("E:")).is_none());
        assert!(parse_manifest(&MANIFEST.replace("1238860", "228980"), Path::new("E:")).is_none());
    }

    #[test]
    fn reads_library_paths() {
        let vdf = "\"libraryfolders\"\n{\n\t\"0\"\n\t{\n\t\t\"path\"\t\t\"C:\\\\Program Files (x86)\\\\Steam\"\n\t}\n\t\"1\"\n\t{\n\t\t\"path\"\t\t\"E:\\\\SteamLibrary\"\n\t}\n}\n";
        assert_eq!(quoted_values(vdf, "path"), vec![r"C:\Program Files (x86)\Steam".to_string(), r"E:\SteamLibrary".to_string()]);
    }

    #[test]
    #[ignore = "reads the real Steam libraries"]
    fn lists_installed_steam_games() {
        for g in steam_games() {
            println!("{} | {} | {:?}", g.name, g.install_dir, g.exe);
        }
    }
}
