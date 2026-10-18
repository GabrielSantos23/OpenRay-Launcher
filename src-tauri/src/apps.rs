use serde::Serialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct InstalledApp {
    pub name: String,
    pub path: String,
}

const EXTENSIONS: [&str; 3] = ["lnk", "url", "appref-ms"];

pub fn start_menu_roots() -> Vec<PathBuf> {
    ["ProgramData", "APPDATA"]
        .iter()
        .filter_map(|var| std::env::var_os(var))
        .map(|base| PathBuf::from(base).join(r"Microsoft\Windows\Start Menu\Programs"))
        .collect()
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(&path, out);
        } else if path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| EXTENSIONS.iter().any(|x| x.eq_ignore_ascii_case(e)))
        {
            out.push(path);
        }
    }
}

fn is_uninstaller(name: &str) -> bool {
    name.to_lowercase().starts_with("uninstall")
}

pub fn scan(roots: &[PathBuf]) -> Vec<InstalledApp> {
    let mut by_name = BTreeMap::new();
    for root in roots {
        let mut files = Vec::new();
        walk(root, &mut files);
        for path in files {
            let Some(name) = path.file_stem().and_then(|s| s.to_str()).map(str::to_owned) else { continue };
            if is_uninstaller(&name) {
                continue;
            }
            by_name.insert(name.to_lowercase(), InstalledApp { name, path: path.to_string_lossy().into_owned() });
        }
    }
    by_name.into_values().collect()
}

#[cfg(windows)]
pub fn launch(path: &str) -> Result<(), String> {
    use windows::core::{w, HSTRING, PCWSTR};
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    if !Path::new(path).exists() {
        return Err(format!("{path} no longer exists"));
    }
    let result = unsafe { ShellExecuteW(None, w!("open"), &HSTRING::from(path), PCWSTR::null(), PCWSTR::null(), SW_SHOWNORMAL) };
    if result.0 as isize > 32 {
        Ok(())
    } else {
        Err(format!("ShellExecute failed with code {}", result.0 as isize))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn touch(path: &Path) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, b"").unwrap();
    }

    #[test]
    fn scans_shortcuts_recursively_dedupes_and_sorts() {
        let common = tempfile::tempdir().unwrap();
        let user = tempfile::tempdir().unwrap();
        touch(&common.path().join("Zed.lnk"));
        touch(&common.path().join(r"Accessories\Notepad.lnk"));
        touch(&common.path().join("Docs.url"));
        touch(&common.path().join("ClickOnce.appref-ms"));
        touch(&common.path().join("readme.txt"));
        touch(&common.path().join(r"Tool\Uninstall Tool.lnk"));
        touch(&user.path().join("notepad.LNK"));

        let apps = scan(&[common.path().to_path_buf(), user.path().to_path_buf()]);
        let names: Vec<_> = apps.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(names, ["ClickOnce", "Docs", "notepad", "Zed"]);
        assert!(apps[2].path.starts_with(&*user.path().to_string_lossy()));
    }

    #[test]
    fn missing_roots_are_ignored() {
        assert!(scan(&[PathBuf::from(r"Z:\does\not\exist")]).is_empty());
    }
}
