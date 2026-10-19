#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Modifiers {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub win: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hotkey {
    pub modifiers: Modifiers,
    pub vk: u32,
}

fn key_code(name: &str) -> Option<u32> {
    let upper = name.to_ascii_uppercase();
    if upper.len() == 1 {
        let c = upper.as_bytes()[0];
        if c.is_ascii_alphanumeric() {
            return Some(c as u32);
        }
    }
    if let Some(n) = upper.strip_prefix('F').and_then(|n| n.parse::<u32>().ok()) {
        if (1..=24).contains(&n) {
            return Some(0x6F + n);
        }
    }
    Some(match upper.as_str() {
        "SPACE" => 0x20,
        "ENTER" | "RETURN" => 0x0D,
        "TAB" => 0x09,
        "BACKQUOTE" | "`" => 0xC0,
        "PERIOD" | "." => 0xBE,
        "COMMA" | "," => 0xBC,
        "SLASH" | "/" => 0xBF,
        "SEMICOLON" | ";" => 0xBA,
        "MINUS" | "-" => 0xBD,
        "EQUAL" | "=" => 0xBB,
        "BRACKETLEFT" | "[" => 0xDB,
        "BACKSLASH" | "\\" => 0xDC,
        "BRACKETRIGHT" | "]" => 0xDD,
        "QUOTE" | "'" => 0xDE,
        "BACKSPACE" => 0x08,
        "PAGEUP" => 0x21,
        "PAGEDOWN" => 0x22,
        "END" => 0x23,
        "HOME" => 0x24,
        "ARROWLEFT" | "LEFT" => 0x25,
        "ARROWUP" | "UP" => 0x26,
        "ARROWRIGHT" | "RIGHT" => 0x27,
        "ARROWDOWN" | "DOWN" => 0x28,
        "INSERT" => 0x2D,
        "DELETE" => 0x2E,
        _ => match upper.strip_prefix("NUMPAD").and_then(|n| n.parse::<u32>().ok()) {
            Some(n) if n <= 9 => 0x60 + n,
            _ => return None,
        },
    })
}

pub fn parse(text: &str) -> Result<Hotkey, String> {
    let mut modifiers = Modifiers::default();
    let mut vk = None;
    for part in text.split('+').map(str::trim) {
        match part.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => modifiers.ctrl = true,
            "alt" => modifiers.alt = true,
            "shift" => modifiers.shift = true,
            "win" | "super" | "meta" | "cmd" => modifiers.win = true,
            _ if vk.is_none() => vk = Some(key_code(part).ok_or_else(|| format!("unknown key \"{part}\" in hotkey \"{text}\""))?),
            _ => return Err(format!("hotkey \"{text}\" has more than one non-modifier key")),
        }
    }
    let vk = vk.ok_or_else(|| format!("hotkey \"{text}\" has no key"))?;
    if modifiers == Modifiers::default() {
        return Err(format!("hotkey \"{text}\" needs at least one modifier"));
    }
    Ok(Hotkey { modifiers, vk })
}

#[derive(Debug, PartialEq, Eq)]
pub enum Decision {
    PassThrough,
    Fire,
    Swallow,
}

#[derive(Default)]
pub struct Matcher {
    held: bool,
}

impl Matcher {
    pub fn on_key(&mut self, hotkey: &Hotkey, vk: u32, down: bool, modifiers: Modifiers) -> Decision {
        if vk != hotkey.vk {
            return Decision::PassThrough;
        }
        if !down {
            return if std::mem::take(&mut self.held) { Decision::Swallow } else { Decision::PassThrough };
        }
        if self.held {
            return Decision::Swallow;
        }
        if modifiers != hotkey.modifiers {
            return Decision::PassThrough;
        }
        self.held = true;
        Decision::Fire
    }
}

mod hook {
    use super::{Decision, Hotkey, Matcher, Modifiers};
    use std::sync::mpsc::{channel, Sender};
    use std::sync::{Mutex, OnceLock};
    use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::Input::KeyboardAndMouse::{keybd_event, GetAsyncKeyState, KEYEVENTF_KEYUP};
    use windows::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, GetMessageW, SetWindowsHookExW, KBDLLHOOKSTRUCT, MSG, WH_KEYBOARD_LL, WM_KEYDOWN, WM_SYSKEYDOWN,
    };

    pub const MARKER: usize = 0x4F50_524E;
    const VK_UNASSIGNED: u8 = 0xE8;

    struct State {
        bindings: Vec<(String, Hotkey, Matcher)>,
        suspended: bool,
        fired: Sender<(String, Hotkey)>,
    }

    static STATE: OnceLock<Mutex<State>> = OnceLock::new();

    fn pressed(vk: i32) -> bool {
        unsafe { GetAsyncKeyState(vk) as u16 & 0x8000 != 0 }
    }

    fn current_modifiers() -> Modifiers {
        Modifiers { ctrl: pressed(0x11), alt: pressed(0x12), shift: pressed(0x10), win: pressed(0x5B) || pressed(0x5C) }
    }

    fn decide(state: &mut State, vk: u32, down: bool) -> bool {
        if state.suspended {
            return false;
        }
        let modifiers = current_modifiers();
        for (id, hotkey, matcher) in state.bindings.iter_mut() {
            match matcher.on_key(hotkey, vk, down, modifiers) {
                Decision::PassThrough => {}
                Decision::Swallow => return true,
                Decision::Fire => {
                    let _ = state.fired.send((id.clone(), *hotkey));
                    return true;
                }
            }
        }
        false
    }

    unsafe extern "system" fn proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if code >= 0 {
            let event = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
            if event.dwExtraInfo != MARKER {
                let down = wparam.0 as u32 == WM_KEYDOWN || wparam.0 as u32 == WM_SYSKEYDOWN;
                if let Some(state) = STATE.get() {
                    if decide(&mut state.lock().unwrap(), event.vkCode, down) {
                        return LRESULT(1);
                    }
                }
            }
        }
        unsafe { CallNextHookEx(None, code, wparam, lparam) }
    }

    pub fn set_bindings(bindings: Vec<(String, Hotkey)>) {
        if let Some(state) = STATE.get() {
            state.lock().unwrap().bindings = bindings.into_iter().map(|(id, hotkey)| (id, hotkey, Matcher::default())).collect();
        }
    }

    pub fn suspend(suspended: bool) {
        if let Some(state) = STATE.get() {
            state.lock().unwrap().suspended = suspended;
        }
    }

    pub fn install(on_fire: Box<dyn Fn(String) + Send>) -> Result<(), String> {
        let (fired, fired_rx) = channel::<(String, Hotkey)>();
        STATE
            .set(Mutex::new(State { bindings: Vec::new(), suspended: false, fired }))
            .map_err(|_| "keyboard hook already installed".to_string())?;
        std::thread::Builder::new()
            .name("openray-hotkey-dispatch".into())
            .spawn(move || {
                for (id, hotkey) in fired_rx {
                    if hotkey.modifiers.alt || hotkey.modifiers.win {
                        unsafe {
                            keybd_event(VK_UNASSIGNED, 0, Default::default(), MARKER);
                            keybd_event(VK_UNASSIGNED, 0, KEYEVENTF_KEYUP, MARKER);
                        }
                    }
                    on_fire(id);
                }
            })
            .map_err(|e| e.to_string())?;
        let (tx, rx) = channel();
        std::thread::Builder::new()
            .name("openray-keyboard-hook".into())
            .spawn(move || unsafe {
                let module = GetModuleHandleW(None).ok();
                match SetWindowsHookExW(WH_KEYBOARD_LL, Some(proc), module.map(Into::into), 0) {
                    Ok(_) => {
                        let _ = tx.send(Ok(()));
                        let mut msg = MSG::default();
                        while GetMessageW(&mut msg, None, 0, 0).as_bool() {}
                    }
                    Err(e) => {
                        let _ = tx.send(Err(e.to_string()));
                    }
                }
            })
            .map_err(|e| e.to_string())?;
        rx.recv().map_err(|e| e.to_string())?
    }
}

pub fn bindings(pairs: &[(String, String)]) -> Result<Vec<(String, Hotkey)>, String> {
    let mut result: Vec<(String, Hotkey)> = Vec::new();
    for (id, text) in pairs.iter().filter(|(_, text)| !text.trim().is_empty()) {
        let hotkey = parse(text)?;
        if result.iter().any(|(_, existing)| *existing == hotkey) {
            return Err(format!("{text} is already in use"));
        }
        result.push((id.clone(), hotkey));
    }
    Ok(result)
}

pub use hook::{install, set_bindings, suspend, MARKER};

#[cfg(test)]
mod tests {
    use super::*;

    fn mods(ctrl: bool, alt: bool, shift: bool, win: bool) -> Modifiers {
        Modifiers { ctrl, alt, shift, win }
    }

    #[test]
    fn parses_modifiers_and_keys() {
        assert_eq!(parse("Alt+Space").unwrap(), Hotkey { modifiers: mods(false, true, false, false), vk: 0x20 });
        assert_eq!(parse("ctrl + alt + shift + o").unwrap(), Hotkey { modifiers: mods(true, true, true, false), vk: 0x4F });
        assert_eq!(parse("Win+F12").unwrap(), Hotkey { modifiers: mods(false, false, false, true), vk: 0x7B });
        assert_eq!(parse("Control+1").unwrap().vk, 0x31);
        assert_eq!(parse("Alt+`").unwrap().vk, 0xC0);
    }

    #[test]
    fn rejects_invalid_hotkeys() {
        assert!(parse("Space").is_err());
        assert_eq!(parse("Ctrl+Shift+ArrowRight").unwrap(), Hotkey { modifiers: mods(true, false, true, false), vk: 0x27 });
        assert_eq!(parse("Alt+Numpad7").unwrap().vk, 0x67);
        assert_eq!(parse("Ctrl+[").unwrap().vk, 0xDB);
        assert!(parse("Alt+").is_err());
        assert!(parse("Alt+Banana").is_err());
        assert!(parse("Alt+A+B").is_err());
        assert!(parse("Alt+F25").is_err());
    }

    #[test]
    fn bindings_skip_empty_and_reject_duplicates() {
        let pairs = |list: &[(&str, &str)]| list.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect::<Vec<_>>();
        let ok = bindings(&pairs(&[("launcher", "Alt+Space"), ("applications/a", ""), ("applications/b", "Ctrl+Alt+B")])).unwrap();
        assert_eq!(ok.iter().map(|(id, _)| id.as_str()).collect::<Vec<_>>(), ["launcher", "applications/b"]);
        assert!(bindings(&pairs(&[("launcher", "Alt+Space"), ("applications/a", "alt + space")])).is_err());
        assert!(bindings(&pairs(&[("applications/a", "Banana")])).is_err());
    }

    #[test]
    fn fires_once_and_swallows_repeats_and_release() {
        let hotkey = parse("Alt+Space").unwrap();
        let alt = mods(false, true, false, false);
        let mut m = Matcher::default();
        assert_eq!(m.on_key(&hotkey, 0x20, true, alt), Decision::Fire);
        assert_eq!(m.on_key(&hotkey, 0x20, true, alt), Decision::Swallow);
        assert_eq!(m.on_key(&hotkey, 0x20, false, alt), Decision::Swallow);
        assert_eq!(m.on_key(&hotkey, 0x20, false, alt), Decision::PassThrough);
        assert_eq!(m.on_key(&hotkey, 0x20, true, alt), Decision::Fire);
    }

    #[test]
    fn requires_exact_modifiers_and_ignores_other_keys() {
        let hotkey = parse("Alt+Space").unwrap();
        let mut m = Matcher::default();
        assert_eq!(m.on_key(&hotkey, 0x20, true, Modifiers::default()), Decision::PassThrough);
        assert_eq!(m.on_key(&hotkey, 0x20, true, mods(true, true, false, false)), Decision::PassThrough);
        assert_eq!(m.on_key(&hotkey, 0x41, true, mods(false, true, false, false)), Decision::PassThrough);
        assert_eq!(m.on_key(&hotkey, 0x20, false, Modifiers::default()), Decision::PassThrough);
    }
}
