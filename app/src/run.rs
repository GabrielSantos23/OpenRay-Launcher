use serde::Serialize;
use std::collections::HashSet;
use std::path::Path;
use windows::core::{w, HSTRING, PCWSTR, PWSTR};
use windows::Win32::Foundation::{GetLastError, ERROR_CANCELLED};
use windows::Win32::System::Environment::ExpandEnvironmentStringsW;
use windows::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegEnumKeyExW, RegOpenKeyExW, RegSetValueExW, HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WRITE, REG_OPTION_NON_VOLATILE, REG_SZ,
};
use windows::Win32::UI::Shell::{ShellExecuteExW, SEE_MASK_FLAG_NO_UI, SEE_MASK_NOASYNC, SHELLEXECUTEINFOW};
use windows::Win32::UI::WindowsAndMessaging::{AllowSetForegroundWindow, ASFW_ANY, SW_SHOWNORMAL};

const RUN_MRU: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\RunMRU";
const APP_PATHS: &str = r"Software\Microsoft\Windows\CurrentVersion\App Paths";

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub kind: &'static str,
    pub command: String,
    pub description: String,
}

pub fn expand(text: &str) -> String {
    let source = HSTRING::from(text);
    let mut buffer = vec![0u16; 4096];
    let len = unsafe { ExpandEnvironmentStringsW(&source, Some(&mut buffer)) } as usize;
    if len == 0 || len > buffer.len() {
        return text.to_string();
    }
    String::from_utf16_lossy(&buffer[..len - 1])
}

pub fn split(text: &str) -> (String, String) {
    let text = text.trim();
    if let Some(rest) = text.strip_prefix('"') {
        return match rest.split_once('"') {
            Some((command, args)) => (command.to_string(), args.trim().to_string()),
            None => (rest.to_string(), String::new()),
        };
    }
    match text.split_once(char::is_whitespace) {
        Some((command, args)) => (command.to_string(), args.trim().to_string()),
        None => (text.to_string(), String::new()),
    }
}

fn open_key(hive: HKEY, path: &str) -> Option<HKEY> {
    let mut key = HKEY::default();
    unsafe { RegOpenKeyExW(hive, &HSTRING::from(path), None, KEY_READ, &mut key) }.is_ok().then_some(key)
}

fn subkeys(key: HKEY) -> Vec<String> {
    let mut names = Vec::new();
    for i in 0.. {
        let mut name = [0u16; 256];
        let mut len = name.len() as u32;
        if unsafe { RegEnumKeyExW(key, i, Some(PWSTR(name.as_mut_ptr())), &mut len, None, None, None, None) }.is_err() {
            break;
        }
        names.push(String::from_utf16_lossy(&name[..len as usize]));
    }
    names
}

fn files_with(dir: &Path, extensions: &[String]) -> Vec<(String, String)> {
    std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let ext = Path::new(&name).extension()?.to_string_lossy().to_lowercase();
            (extensions.contains(&ext) && e.file_type().ok()?.is_file()).then(|| (name, e.path().to_string_lossy().into_owned()))
        })
        .collect()
}

pub fn catalog() -> Vec<Entry> {
    let mut list = Vec::new();
    let mut seen = HashSet::new();
    let mut push = |list: &mut Vec<Entry>, kind, command: String, description: String| {
        if seen.insert(command.to_lowercase()) {
            list.push(Entry { kind, command, description });
        }
    };
    let system = expand("%SystemRoot%\\System32");
    for (name, path) in files_with(Path::new(&system), &["cpl".into()]) {
        push(&mut list, "controlPanelApplet", name, path);
    }
    for (name, path) in files_with(Path::new(&system), &["msc".into()]) {
        push(&mut list, "managementConsole", name, path);
    }
    for hive in [HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE] {
        let Some(root) = open_key(hive, APP_PATHS) else { continue };
        for name in subkeys(root) {
            let key = open_key(root, &name);
            let path = key.map(|k| crate::uninstall::text(k, "")).unwrap_or_default();
            if let Some(k) = key {
                unsafe {
                    let _ = RegCloseKey(k);
                }
            }
            let path = path.trim().trim_matches('"').to_string();
            if !path.is_empty() && Path::new(&expand(&path)).exists() {
                push(&mut list, "executable", name, path);
            }
        }
        unsafe {
            let _ = RegCloseKey(root);
        }
    }
    let extensions: Vec<String> = std::env::var("PATHEXT").unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".into()).split(';').map(|e| e.trim_start_matches('.').to_lowercase()).filter(|e| !e.is_empty()).collect();
    for dir in std::env::var("PATH").unwrap_or_default().split(';').filter(|d| !d.trim().is_empty()) {
        for (name, path) in files_with(Path::new(&expand(dir.trim())), &extensions) {
            push(&mut list, "executable", name, path);
        }
    }
    let mut vars: Vec<(String, String)> = std::env::vars().filter(|(k, _)| !k.starts_with('=')).collect();
    vars.sort_by_key(|(k, _)| k.to_lowercase());
    for (name, value) in vars {
        push(&mut list, "environmentVariable", format!("%{name}%"), value);
    }
    list
}

pub fn complete(text: &str) -> Vec<Entry> {
    let expanded = expand(text.trim().trim_start_matches('"'));
    let Some(cut) = expanded.rfind(['\\', '/']) else { return Vec::new() };
    let (dir, partial) = (&expanded[..=cut], expanded[cut + 1..].to_lowercase());
    let shown = &text.trim().trim_start_matches('"')[..text.trim().trim_start_matches('"').rfind(['\\', '/']).map_or(0, |i| i + 1)];
    let mut list: Vec<Entry> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            if !name.to_lowercase().starts_with(&partial) {
                return None;
            }
            let is_dir = e.file_type().ok()?.is_dir();
            let path = e.path().to_string_lossy().into_owned();
            Some(Entry { kind: "path", command: format!("{shown}{name}{}", if is_dir { "\\" } else { "" }), description: path })
        })
        .collect();
    list.sort_by_key(|e| (!e.command.ends_with('\\'), e.command.to_lowercase()));
    list.truncate(50);
    list
}

fn mru_order(key: HKEY) -> String {
    crate::uninstall::text(key, "MRUList")
}

pub fn parse_mru(order: &str, value: impl Fn(char) -> String) -> Vec<String> {
    order.chars().map(|c| value(c)).map(|v| v.strip_suffix("\\1").map(str::to_string).unwrap_or(v)).filter(|v| !v.is_empty()).collect()
}

pub fn history() -> Vec<String> {
    let Some(key) = open_key(HKEY_CURRENT_USER, RUN_MRU) else { return Vec::new() };
    let list = parse_mru(&mru_order(key), |c| crate::uninstall::text(key, &c.to_string()));
    unsafe {
        let _ = RegCloseKey(key);
    }
    list
}

fn set_text(key: HKEY, name: &str, value: &str) -> Result<(), String> {
    let wide: Vec<u16> = value.encode_utf16().chain(Some(0)).collect();
    let bytes = unsafe { std::slice::from_raw_parts(wide.as_ptr() as *const u8, wide.len() * 2) };
    unsafe { RegSetValueExW(key, &HSTRING::from(name), None, REG_SZ, Some(bytes)) }.ok().map_err(|e| e.to_string())
}

fn with_mru<T>(f: impl FnOnce(HKEY) -> Result<T, String>) -> Result<T, String> {
    let mut key = HKEY::default();
    unsafe { RegCreateKeyExW(HKEY_CURRENT_USER, &HSTRING::from(RUN_MRU), None, PCWSTR::null(), REG_OPTION_NON_VOLATILE, KEY_READ | KEY_WRITE, None, &mut key, None) }
        .ok()
        .map_err(|e| e.to_string())?;
    let result = f(key);
    unsafe {
        let _ = RegCloseKey(key);
    }
    result
}

pub fn next_mru(order: &str, existing: Option<char>) -> (char, String) {
    let letter = existing.or_else(|| ('a'..='z').find(|c| !order.contains(*c))).unwrap_or_else(|| order.chars().last().unwrap_or('a'));
    (letter, std::iter::once(letter).chain(order.chars().filter(|&c| c != letter)).collect())
}

pub fn remember(command: &str) -> Result<(), String> {
    with_mru(|key| {
        let order = mru_order(key);
        let existing = order.chars().find(|c| crate::uninstall::text(key, &c.to_string()).strip_suffix("\\1").is_some_and(|v| v.eq_ignore_ascii_case(command)));
        let (letter, order) = next_mru(&order, existing);
        set_text(key, &letter.to_string(), &format!("{command}\\1"))?;
        set_text(key, "MRUList", &order)
    })
}

pub fn forget(command: &str) -> Result<(), String> {
    with_mru(|key| {
        let order = mru_order(key);
        let Some(letter) = order.chars().find(|c| crate::uninstall::text(key, &c.to_string()).strip_suffix("\\1").is_some_and(|v| v == command)) else { return Ok(()) };
        unsafe {
            let _ = RegDeleteValueW(key, &HSTRING::from(letter.to_string()));
        }
        set_text(key, "MRUList", &order.replace(letter, ""))
    })
}

pub fn execute(command: &str, args: &str, admin: bool, in_shell: bool) -> Result<(), String> {
    let command = command.trim();
    if command.is_empty() {
        return Err("Type a command to run".into());
    }
    let (file, params) = if in_shell { ("cmd.exe".to_string(), format!("/k {}", join(command, args))) } else { (expand(command), expand(args)) };
    let file = HSTRING::from(file);
    let params = HSTRING::from(params);
    let mut info = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_FLAG_NO_UI | SEE_MASK_NOASYNC,
        lpVerb: if admin { w!("runas") } else { w!("open") },
        lpFile: PCWSTR(file.as_ptr()),
        lpParameters: if params.is_empty() { PCWSTR::null() } else { PCWSTR(params.as_ptr()) },
        nShow: SW_SHOWNORMAL.0,
        ..Default::default()
    };
    unsafe {
        let _ = AllowSetForegroundWindow(ASFW_ANY);
    }
    if unsafe { ShellExecuteExW(&mut info) }.is_err() {
        let error = unsafe { GetLastError() };
        if error == ERROR_CANCELLED {
            return Err("The operation was canceled".into());
        }
        return Err(format!("Windows cannot find '{command}'. Make sure you typed the name correctly, and then try again."));
    }
    let _ = remember(&join(command, args));
    Ok(())
}

pub fn join(command: &str, args: &str) -> String {
    let command = if command.contains(' ') && !command.starts_with('"') { format!("\"{command}\"") } else { command.to_string() };
    if args.trim().is_empty() {
        command
    } else {
        format!("{command} {}", args.trim())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_commands_and_arguments() {
        assert_eq!(split("notepad C:\\a.txt"), ("notepad".into(), "C:\\a.txt".into()));
        assert_eq!(split("\"C:\\Program Files\\x.exe\" -a  -b"), ("C:\\Program Files\\x.exe".into(), "-a  -b".into()));
        assert_eq!(split("  cmd  "), ("cmd".into(), String::new()));
        assert_eq!(join("C:\\Program Files\\x.exe", "-a"), "\"C:\\Program Files\\x.exe\" -a");
        assert_eq!(join("cmd", " "), "cmd");
    }

    #[test]
    fn reads_and_orders_the_run_history() {
        let values = |c: char| match c {
            'a' => "cmd\\1".to_string(),
            'b' => "regedit\\1".to_string(),
            _ => String::new(),
        };
        assert_eq!(parse_mru("bac", values), vec!["regedit".to_string(), "cmd".to_string()]);
        assert_eq!(next_mru("ba", None), ('c', "cba".into()));
        assert_eq!(next_mru("ba", Some('a')), ('a', "ab".into()));
        let full: String = ('a'..='z').collect();
        assert_eq!(next_mru(&full, None).0, 'z');
    }

    #[test]
    fn expands_environment_variables() {
        assert!(!expand("%SystemRoot%").contains('%'));
        assert_eq!(expand("plain"), "plain");
    }
}
