use serde::Serialize;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    GetMenu, GetMenuItemCount, GetMenuItemInfoW, PostMessageW, SendMessageTimeoutW, SetForegroundWindow, HMENU, MENUITEMINFOW, MFS_CHECKED, MFS_DISABLED, MFT_SEPARATOR, MIIM_FTYPE, MIIM_ID, MIIM_STATE, MIIM_STRING,
    MIIM_SUBMENU, SMTO_ABORTIFHUNG, WM_COMMAND, WM_INITMENUPOPUP,
};

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MenuItem {
    pub id: u32,
    pub title: String,
    pub path: Vec<String>,
    pub shortcut: String,
    pub checked: bool,
}

pub fn clean_label(raw: &str) -> (String, String) {
    let (label, shortcut) = raw.split_once('\t').unwrap_or((raw, ""));
    let mut text = String::new();
    let mut chars = label.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '&' {
            if chars.peek() == Some(&'&') {
                text.push('&');
                chars.next();
            }
            continue;
        }
        text.push(c);
    }
    (text.trim().trim_end_matches("...").trim_end_matches('…').trim().to_string(), shortcut.trim().to_string())
}

fn walk(hwnd: HWND, menu: HMENU, path: &mut Vec<String>, out: &mut Vec<MenuItem>, depth: usize) {
    unsafe {
        let _ = SendMessageTimeoutW(hwnd, WM_INITMENUPOPUP, WPARAM(menu.0 as usize), LPARAM(0), SMTO_ABORTIFHUNG, 200, None);
    }
    let count = unsafe { GetMenuItemCount(Some(menu)) };
    for i in 0..count.max(0) as u32 {
        let mut buffer = [0u16; 512];
        let mut info = MENUITEMINFOW {
            cbSize: std::mem::size_of::<MENUITEMINFOW>() as u32,
            fMask: MIIM_STRING | MIIM_ID | MIIM_SUBMENU | MIIM_STATE | MIIM_FTYPE,
            dwTypeData: windows::core::PWSTR(buffer.as_mut_ptr()),
            cch: buffer.len() as u32 - 1,
            ..Default::default()
        };
        if unsafe { GetMenuItemInfoW(menu, i, true, &mut info) }.is_err() || info.fType.contains(MFT_SEPARATOR) {
            continue;
        }
        let (title, shortcut) = clean_label(&String::from_utf16_lossy(&buffer[..info.cch as usize]));
        if title.is_empty() || info.fState.contains(MFS_DISABLED) {
            continue;
        }
        if !info.hSubMenu.is_invalid() {
            if depth < 4 {
                path.push(title);
                walk(hwnd, info.hSubMenu, path, out, depth + 1);
                path.pop();
            }
        } else {
            out.push(MenuItem { id: info.wID, title, path: path.clone(), shortcut, checked: info.fState.contains(MFS_CHECKED) });
        }
    }
}

pub fn list(hwnd: HWND) -> Vec<MenuItem> {
    let menu = unsafe { GetMenu(hwnd) };
    let mut out = Vec::new();
    if !menu.is_invalid() {
        walk(hwnd, menu, &mut Vec::new(), &mut out, 0);
    }
    out
}

pub fn invoke(hwnd: HWND, id: u32) -> Result<(), String> {
    unsafe {
        let _ = SetForegroundWindow(hwnd);
        PostMessageW(Some(hwnd), WM_COMMAND, WPARAM(id as usize), LPARAM(0)).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleans_menu_labels() {
        assert_eq!(clean_label("&Save As...\tCtrl+Shift+S"), ("Save As".into(), "Ctrl+Shift+S".into()));
        assert_eq!(clean_label("Fish && &Chips"), ("Fish & Chips".into(), String::new()));
    }
}
