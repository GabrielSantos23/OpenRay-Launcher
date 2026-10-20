use std::os::windows::process::CommandExt;
use windows::core::{HSTRING, PCWSTR};
use windows::Win32::System::Registry::{RegCloseKey, RegEnumKeyExW, RegGetValueW, RegOpenKeyExW, HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, RRF_RT_REG_DWORD, RRF_RT_REG_SZ};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

const APPS_FOLDER: &str = r"shell:AppsFolder\";
const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const ROOTS: [(HKEY, &str); 3] = [
    (HKEY_LOCAL_MACHINE, r"Software\Microsoft\Windows\CurrentVersion\Uninstall"),
    (HKEY_LOCAL_MACHINE, r"Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall"),
    (HKEY_CURRENT_USER, r"Software\Microsoft\Windows\CurrentVersion\Uninstall"),
];

#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub name: String,
    pub location: String,
    pub icon: String,
    pub command: String,
}

fn text(key: HKEY, name: &str) -> String {
    let name = HSTRING::from(name);
    let mut size = 0u32;
    unsafe {
        if RegGetValueW(key, PCWSTR::null(), &name, RRF_RT_REG_SZ, None, None, Some(&mut size)).is_err() || size < 2 {
            return String::new();
        }
        let mut buffer = vec![0u16; size as usize / 2];
        if RegGetValueW(key, PCWSTR::null(), &name, RRF_RT_REG_SZ, None, Some(buffer.as_mut_ptr() as _), Some(&mut size)).is_err() {
            return String::new();
        }
        String::from_utf16_lossy(&buffer[..buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len())])
    }
}

fn dword(key: HKEY, name: &str) -> u32 {
    let mut value = 0u32;
    let mut size = 4u32;
    unsafe {
        let _ = RegGetValueW(key, PCWSTR::null(), &HSTRING::from(name), RRF_RT_REG_DWORD, None, Some(&mut value as *mut u32 as _), Some(&mut size));
    }
    value
}

fn entries() -> Vec<Entry> {
    let mut list = Vec::new();
    for (hive, path) in ROOTS {
        let mut root = HKEY::default();
        if unsafe { RegOpenKeyExW(hive, &HSTRING::from(path), None, KEY_READ, &mut root) }.is_err() {
            continue;
        }
        for i in 0.. {
            let mut name = [0u16; 256];
            let mut len = name.len() as u32;
            if unsafe { RegEnumKeyExW(root, i, Some(windows::core::PWSTR(name.as_mut_ptr())), &mut len, None, None, None, None) }.is_err() {
                break;
            }
            let mut key = HKEY::default();
            if unsafe { RegOpenKeyExW(root, PCWSTR(name.as_ptr()), None, KEY_READ, &mut key) }.is_err() {
                continue;
            }
            let entry = Entry { name: text(key, "DisplayName"), location: text(key, "InstallLocation"), icon: text(key, "DisplayIcon"), command: text(key, "UninstallString") };
            if !entry.name.is_empty() && !entry.command.is_empty() && dword(key, "SystemComponent") != 1 {
                list.push(entry);
            }
            unsafe {
                let _ = RegCloseKey(key);
            }
        }
        unsafe {
            let _ = RegCloseKey(root);
        }
    }
    list
}

fn clean(path: &str) -> String {
    let path = path.trim().trim_matches('"');
    let path = path.rsplit_once(',').filter(|(_, index)| index.trim().parse::<i32>().is_ok()).map_or(path, |(p, _)| p);
    path.trim().trim_matches('"').trim_end_matches('\\').to_lowercase()
}

pub fn pick<'a>(entries: &'a [Entry], app_name: &str, target: Option<&str>) -> Option<&'a Entry> {
    let target = target.map(clean).filter(|t| !t.is_empty());
    let name = app_name.trim().to_lowercase();
    let by_icon = target.as_ref().and_then(|t| entries.iter().find(|e| clean(&e.icon) == *t));
    let by_location = target.as_ref().and_then(|t| entries.iter().filter(|e| !clean(&e.location).is_empty() && t.starts_with(&(clean(&e.location) + "\\"))).max_by_key(|e| clean(&e.location).len()));
    let by_name = entries.iter().find(|e| e.name.to_lowercase() == name).or_else(|| entries.iter().find(|e| e.name.to_lowercase().starts_with(&name)));
    by_icon.or(by_location).or(by_name)
}

pub fn split_command(command: &str) -> (String, String) {
    let command = command.trim();
    if let Some(rest) = command.strip_prefix('"') {
        if let Some((program, args)) = rest.split_once('"') {
            return (program.to_string(), args.trim().to_string());
        }
    }
    let lower = command.to_lowercase();
    match lower.find(".exe") {
        Some(end) => (command[..end + 4].to_string(), command[end + 4..].trim().to_string()),
        None => (command.to_string(), String::new()),
    }
}

fn shell(program: &str, args: &str) -> Result<(), String> {
    let result = unsafe { ShellExecuteW(None, &HSTRING::from("open"), &HSTRING::from(program), &HSTRING::from(args), PCWSTR::null(), SW_SHOWNORMAL) };
    if result.0 as isize > 32 {
        Ok(())
    } else {
        Err(format!("Could not start the uninstaller (code {})", result.0 as isize))
    }
}

pub enum Target {
    Package(String),
    Program(Entry),
    Settings,
}

pub fn resolve(name: &str, path: &str) -> Result<Target, String> {
    if let Some(id) = path.strip_prefix(APPS_FOLDER) {
        let family = id.split('!').next().unwrap_or_default();
        if family.is_empty() || !family.chars().all(|c| c.is_ascii_alphanumeric() || "._-".contains(c)) {
            return Err("Unknown package".into());
        }
        return Ok(Target::Package(family.to_string()));
    }
    let target = crate::focus::shortcut_target(path).or_else(|| path.to_lowercase().ends_with(".exe").then(|| path.to_string()));
    Ok(pick(&entries(), name, target.as_deref()).cloned().map_or(Target::Settings, Target::Program))
}

pub fn describe(name: &str, path: &str) -> Result<Option<String>, String> {
    Ok(match resolve(name, path)? {
        Target::Package(_) => Some(String::new()),
        Target::Program(entry) => Some(entry.name),
        Target::Settings => None,
    })
}

pub fn uninstall(name: &str, path: &str) -> Result<&'static str, String> {
    match resolve(name, path)? {
        Target::Package(family) => {
            let status = std::process::Command::new("powershell.exe")
                .args(["-NoProfile", "-NonInteractive", "-Command", &format!("$ErrorActionPreference='Stop'; Get-AppxPackage -PackageFamilyName '{family}' | Remove-AppxPackage")])
                .creation_flags(CREATE_NO_WINDOW)
                .status()
                .map_err(|e| e.to_string())?;
            if status.success() { Ok("removed") } else { Err("Windows did not allow removing this app".into()) }
        }
        Target::Program(entry) => {
            let (program, args) = split_command(&entry.command);
            shell(&program, &args).map(|_| "requested")
        }
        Target::Settings => shell("ms-settings:appsfeatures", "").map(|_| "settings"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, location: &str, icon: &str) -> Entry {
        Entry { name: name.into(), location: location.into(), icon: icon.into(), command: "x".into() }
    }

    #[test]
    fn picks_by_icon_then_location_then_name() {
        let list = vec![
            entry("Notepad++ (64-bit x64)", r"C:\Program Files\Notepad++", ""),
            entry("Discord", "", r"C:\Users\me\AppData\Local\Discord\app.ico"),
            entry("7-Zip 24.08", "", r#""C:\Program Files\7-Zip\7zFM.exe",0"#),
        ];
        assert_eq!(pick(&list, "7-Zip File Manager", Some(r"C:\Program Files\7-Zip\7zFM.exe")).unwrap().name, "7-Zip 24.08");
        assert_eq!(pick(&list, "Notepad++", Some(r"C:\Program Files\Notepad++\notepad++.exe")).unwrap().name, "Notepad++ (64-bit x64)");
        assert_eq!(pick(&list, "Discord", Some(r"C:\Users\me\AppData\Local\Discord\Update.exe")).unwrap().name, "Discord");
        assert!(pick(&list, "Paint", Some(r"C:\Windows\paint.exe")).is_none());
    }

    #[test]
    fn splits_uninstall_commands() {
        assert_eq!(split_command(r#""C:\Program Files\App\unins000.exe" /SILENT"#), (r"C:\Program Files\App\unins000.exe".into(), "/SILENT".into()));
        assert_eq!(split_command("MsiExec.exe /X{1234}"), ("MsiExec.exe".into(), "/X{1234}".into()));
        assert_eq!(split_command(r"C:\Apps\uninstall.exe"), (r"C:\Apps\uninstall.exe".into(), String::new()));
    }

    #[test]
    #[ignore = "reads the real uninstall registry and Start Menu"]
    fn dry_run_matches_installed_apps() {
        let list = entries();
        for root in crate::apps::start_menu_roots() {
            for app in crate::apps::scan(&[root], Vec::new()).into_iter().take(40) {
                let target = crate::focus::shortcut_target(&app.path);
                println!("{} -> {:?}", app.name, pick(&list, &app.name, target.as_deref()).map(|e| (&e.name, &e.command)));
            }
        }
    }
}
