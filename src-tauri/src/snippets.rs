use serde_json::Value;
use std::sync::mpsc::{channel, Sender};
use std::sync::{LazyLock, Mutex, OnceLock};
use tauri::{AppHandle, Emitter};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, GetKeyState, GetKeyboardLayout, SendInput, ToUnicodeEx, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE,
    VIRTUAL_KEY, VK_BACK, VK_LEFT, VK_RETURN, VK_TAB,
};
use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

const MAX_TYPED: usize = 64;
const RESET_KEYS: [u32; 13] = [0x09, 0x0D, 0x1B, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28, 0x2D, 0x2E];
const MODIFIER_KEYS: [u32; 11] = [0x10, 0x11, 0x12, 0x14, 0xA0, 0xA1, 0xA2, 0xA3, 0xA4, 0xA5, 0x5D];

#[derive(Default)]
struct State {
    keywords: Vec<(String, String)>,
    typed: String,
    window: isize,
}

static STATE: LazyLock<Mutex<State>> = LazyLock::new(Default::default);
static WORKER: OnceLock<Sender<(String, usize)>> = OnceLock::new();

pub fn init(app: &AppHandle, data: &Value) {
    load(data);
    let (tx, rx) = channel::<(String, usize)>();
    if WORKER.set(tx).is_err() {
        return;
    }
    let app = app.clone();
    std::thread::spawn(move || {
        for (id, typed) in rx {
            send(&vec![VK_BACK; typed].into_iter().flat_map(|vk| [key(vk, false), key(vk, true)]).collect::<Vec<_>>());
            let _ = app.emit_to(crate::launcher::LABEL, "snippet-expand", id);
        }
    });
}

pub fn load(data: &Value) {
    let keywords = data
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|snippet| {
            let keyword = snippet.get("keyword")?.as_str()?.trim();
            let id = snippet.get("id")?.as_str()?;
            (!keyword.is_empty()).then(|| (id.to_string(), keyword.to_string()))
        })
        .collect();
    let mut state = STATE.lock().unwrap();
    state.keywords = keywords;
    state.typed.clear();
}

fn pressed(vk: i32) -> bool {
    unsafe { GetAsyncKeyState(vk) as u16 & 0x8000 != 0 }
}

fn typed_char(vk: u32, scan: u32, foreground: HWND, altgr: bool) -> Option<char> {
    let mut keys = [0u8; 256];
    if pressed(0x10) {
        keys[0x10] = 0x80;
    }
    if unsafe { GetKeyState(0x14) } & 1 != 0 {
        keys[0x14] = 1;
    }
    if altgr {
        keys[0x11] = 0x80;
        keys[0x12] = 0x80;
    }
    let layout = unsafe { GetKeyboardLayout(GetWindowThreadProcessId(foreground, None)) };
    let mut buffer = [0u16; 8];
    let written = unsafe { ToUnicodeEx(vk, scan, &keys, &mut buffer, 0x4, Some(layout)) };
    if written != 1 {
        return None;
    }
    char::from_u32(buffer[0] as u32).filter(|c| !c.is_control())
}

fn own_window(window: HWND) -> bool {
    let mut pid = 0;
    unsafe { GetWindowThreadProcessId(window, Some(&mut pid)) };
    pid == std::process::id()
}

pub fn on_key(vk: u32, scan: u32) {
    let mut state = STATE.lock().unwrap();
    if state.keywords.is_empty() || MODIFIER_KEYS.contains(&vk) {
        return;
    }
    let foreground = unsafe { GetForegroundWindow() };
    if foreground.0 as isize != state.window {
        state.window = foreground.0 as isize;
        state.typed.clear();
    }
    let (ctrl, alt, win) = (pressed(0x11), pressed(0x12), pressed(0x5B) || pressed(0x5C));
    let altgr = ctrl && alt;
    if vk == VK_BACK.0 as u32 && !ctrl && !alt {
        state.typed.pop();
        return;
    }
    if RESET_KEYS.contains(&vk) || win || (ctrl != alt) || own_window(foreground) {
        state.typed.clear();
        return;
    }
    let Some(c) = typed_char(vk, scan, foreground, altgr) else { return };
    state.typed.push(c);
    let excess = state.typed.chars().count().saturating_sub(MAX_TYPED);
    if excess > 0 {
        state.typed = state.typed.chars().skip(excess).collect();
    }
    if let Some((id, length)) = matched(&state.typed, &state.keywords) {
        state.typed.clear();
        if let Some(worker) = WORKER.get() {
            let _ = worker.send((id, length));
        }
    }
}

fn matched(typed: &str, keywords: &[(String, String)]) -> Option<(String, usize)> {
    keywords
        .iter()
        .filter(|(_, keyword)| {
            typed.strip_suffix(keyword.as_str()).is_some_and(|before| before.chars().last().is_none_or(|c| !c.is_alphanumeric()))
        })
        .max_by_key(|(_, keyword)| keyword.chars().count())
        .map(|(id, keyword)| (id.clone(), keyword.chars().count()))
}

fn key(vk: VIRTUAL_KEY, up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: vk, dwFlags: if up { KEYEVENTF_KEYUP } else { Default::default() }, dwExtraInfo: crate::hotkey::MARKER, ..Default::default() } },
    }
}

fn unit(code: u16, up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT { wScan: code, dwFlags: if up { KEYEVENTF_UNICODE | KEYEVENTF_KEYUP } else { KEYEVENTF_UNICODE }, dwExtraInfo: crate::hotkey::MARKER, ..Default::default() },
        },
    }
}

fn send(inputs: &[INPUT]) {
    for chunk in inputs.chunks(200) {
        unsafe { SendInput(chunk, size_of::<INPUT>() as i32) };
    }
}

fn keystrokes(text: &str, cursor_back: usize) -> Vec<INPUT> {
    let mut inputs = Vec::new();
    for c in text.replace("\r\n", "\n").chars() {
        match c {
            '\n' => inputs.extend([key(VK_RETURN, false), key(VK_RETURN, true)]),
            '\t' => inputs.extend([key(VK_TAB, false), key(VK_TAB, true)]),
            c => {
                let mut units = [0u16; 2];
                for code in c.encode_utf16(&mut units).iter() {
                    inputs.extend([unit(*code, false), unit(*code, true)]);
                }
            }
        }
    }
    for _ in 0..cursor_back {
        inputs.extend([key(VK_LEFT, false), key(VK_LEFT, true)]);
    }
    inputs
}

pub fn type_text(text: &str, cursor_back: usize) {
    send(&keystrokes(text, cursor_back));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keywords() -> Vec<(String, String)> {
        vec![("sig".into(), ";sig".into()), ("sig2".into(), ";sig2".into()), ("addr".into(), "addr".into())]
    }

    #[test]
    fn matches_keywords_at_word_boundaries() {
        assert_eq!(matched("hello ;sig", &keywords()), Some(("sig".into(), 4)));
        assert_eq!(matched(";sig2", &keywords()), Some(("sig2".into(), 5)));
        assert_eq!(matched("myaddr", &keywords()), None);
        assert_eq!(matched("my addr", &keywords()), Some(("addr".into(), 4)));
        assert_eq!(matched("a;si", &keywords()), None);
    }

    #[test]
    fn loads_only_snippets_with_keywords() {
        load(&serde_json::json!([{ "id": "a", "keyword": " ;a " }, { "id": "b", "keyword": "" }, { "id": "c" }]));
        assert_eq!(STATE.lock().unwrap().keywords, vec![("a".to_string(), ";a".to_string())]);
    }

    #[test]
    fn types_newlines_and_moves_the_cursor_back() {
        assert_eq!(keystrokes("a\r\nb", 1).len(), 8);
    }
}
