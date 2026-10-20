use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;
use windows::core::HSTRING;
use windows::Win32::System::Registry::{RegCloseKey, RegOpenKeyExW, HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ};
use windows::Win32::UI::Shell::{FOLDERID_Documents, SHGetKnownFolderPath, KF_FLAG_DEFAULT};

const UNINSTALL_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall\82E6AC09-0FEF-4390-AD9F-0DD3F5561EFC_is1";
const MEDIA: [&str; 9] = ["png", "jpg", "jpeg", "gif", "bmp", "webp", "tif", "mp4", "webm"];
const LIMIT: usize = 3000;

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Screenshot {
    pub path: String,
    pub name: String,
    pub modified: u64,
    pub size: u64,
    pub video: bool,
}

fn install_dir() -> Option<PathBuf> {
    for hive in [HKEY_LOCAL_MACHINE, HKEY_CURRENT_USER] {
        let mut key = HKEY::default();
        if unsafe { RegOpenKeyExW(hive, &HSTRING::from(UNINSTALL_KEY), None, KEY_READ, &mut key) }.is_ok() {
            let location = crate::uninstall::text(key, "InstallLocation");
            unsafe {
                let _ = RegCloseKey(key);
            }
            if Path::new(&location).join("ShareX.exe").exists() {
                return Some(PathBuf::from(location));
            }
        }
    }
    let candidates = [
        std::env::var_os("ProgramFiles").map(|p| PathBuf::from(p).join("ShareX")),
        crate::games::steam_path().map(|p| p.join("steamapps").join("common").join("ShareX")),
    ];
    candidates.into_iter().flatten().find(|p| p.join("ShareX.exe").exists())
}

pub fn exe() -> Result<PathBuf, String> {
    install_dir().map(|d| d.join("ShareX.exe")).ok_or_else(|| "ShareX isn't installed. Get it from getsharex.com.".into())
}

fn documents() -> PathBuf {
    unsafe { SHGetKnownFolderPath(&FOLDERID_Documents, KF_FLAG_DEFAULT, None) }
        .ok()
        .and_then(|p| unsafe { p.to_string() }.ok())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(std::env::var("USERPROFILE").unwrap_or_default()).join("Documents"))
}

pub fn personal_folder(install: Option<&Path>) -> PathBuf {
    let custom = install.and_then(|d| std::fs::read_to_string(d.join("PersonalPath.cfg")).ok()).map(|t| t.trim().to_string()).filter(|t| !t.is_empty());
    match custom {
        Some(path) => {
            let path = PathBuf::from(crate::run::expand(&path));
            if path.is_absolute() {
                path
            } else {
                install.map(|d| d.join(&path)).unwrap_or(path)
            }
        }
        None => documents().join("ShareX"),
    }
}

pub fn screenshots_folder_from(personal: &Path, config: Option<&str>) -> PathBuf {
    let custom = config.and_then(|text| serde_json::from_str::<serde_json::Value>(text).ok()).and_then(|json| {
        let enabled = json["UseCustomScreenshotsPath"].as_bool().unwrap_or(false);
        let path = json["CustomScreenshotsPath"].as_str().unwrap_or("").trim().to_string();
        (enabled && !path.is_empty()).then(|| PathBuf::from(crate::run::expand(&path)))
    });
    custom.unwrap_or_else(|| personal.join("Screenshots"))
}

pub fn screenshots_folder() -> PathBuf {
    let personal = personal_folder(install_dir().as_deref());
    let config = std::fs::read_to_string(personal.join("ApplicationConfig.json")).ok();
    screenshots_folder_from(&personal, config.as_deref().map(|t| t.trim_start_matches('\u{feff}')))
}

pub fn screenshots() -> Vec<Screenshot> {
    let mut list = Vec::new();
    let mut stack = vec![(screenshots_folder(), 0)];
    while let Some((dir, depth)) = stack.pop() {
        for entry in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
            let Ok(meta) = entry.metadata() else { continue };
            let path = entry.path();
            if meta.is_dir() {
                if depth < 3 {
                    stack.push((path, depth + 1));
                }
                continue;
            }
            let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
            if !MEDIA.contains(&ext.as_str()) {
                continue;
            }
            let modified = meta.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map_or(0, |d| d.as_millis() as u64);
            list.push(Screenshot { name: entry.file_name().to_string_lossy().into_owned(), path: path.to_string_lossy().into_owned(), modified, size: meta.len(), video: ext == "mp4" || ext == "webm" });
        }
    }
    list.sort_by(|a, b| b.modified.cmp(&a.modified));
    list.truncate(LIMIT);
    list
}

pub fn latest_image() -> Option<Screenshot> {
    screenshots().into_iter().find(|s| !s.video)
}

pub fn run(job: &str, file: Option<&str>) -> Result<(), String> {
    if job.is_empty() || !job.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(format!("Unknown ShareX task {job}"));
    }
    let mut command = std::process::Command::new(exe()?);
    command.arg("-silent").arg(format!("-{job}"));
    if let Some(file) = file {
        if !Path::new(file).exists() {
            return Err(format!("{file} no longer exists"));
        }
        command.arg(file);
    }
    command.spawn().map(|_| ()).map_err(|e| format!("Couldn't start ShareX: {e}"))
}

pub fn upload(file: &str) -> Result<(), String> {
    if !Path::new(file).exists() {
        return Err(format!("{file} no longer exists"));
    }
    std::process::Command::new(exe()?).arg("-silent").arg(file).spawn().map(|_| ()).map_err(|e| format!("Couldn't start ShareX: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_the_screenshots_folder() {
        let personal = Path::new(r"C:\Users\me\Documents\ShareX");
        assert_eq!(screenshots_folder_from(personal, None), personal.join("Screenshots"));
        assert_eq!(screenshots_folder_from(personal, Some(r#"{"UseCustomScreenshotsPath":false,"CustomScreenshotsPath":"D:\\Shots"}"#)), personal.join("Screenshots"));
        assert_eq!(screenshots_folder_from(personal, Some(r#"{"UseCustomScreenshotsPath":true,"CustomScreenshotsPath":"D:\\Shots"}"#)), PathBuf::from(r"D:\Shots"));
        assert_eq!(screenshots_folder_from(personal, Some("not json")), personal.join("Screenshots"));
    }

    #[test]
    fn rejects_unsafe_task_names() {
        assert!(run("RectangleRegion -x", None).is_err());
        assert!(run("", None).is_err());
    }

    #[test]
    #[ignore = "reads the real ShareX install"]
    fn lists_real_screenshots() {
        println!("{:?} {:?}", exe(), screenshots_folder());
        for s in screenshots().iter().take(5) {
            println!("{} {}", s.path, s.modified);
        }
    }
}
