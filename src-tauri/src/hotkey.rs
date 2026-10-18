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
        _ => return None,
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

    const MARKER: usize = 0x4F50_524E;
    const VK_UNASSIGNED: u8 = 0xE8;

    struct State {
        hotkey: Hotkey,
        matcher: Matcher,
        fired: Sender<()>,
    }

    static STATE: OnceLock<Mutex<State>> = OnceLock::new();

    fn pressed(vk: i32) -> bool {
        unsafe { GetAsyncKeyState(vk) as u16 & 0x8000 != 0 }
    }

    fn current_modifiers() -> Modifiers {
        Modifiers { ctrl: pressed(0x11), alt: pressed(0x12), shift: pressed(0x10), win: pressed(0x5B) || pressed(0x5C) }
    }

    unsafe extern "system" fn proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if code >= 0 {
            let event = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
            if event.dwExtraInfo != MARKER {
                let down = wparam.0 as u32 == WM_KEYDOWN || wparam.0 as u32 == WM_SYSKEYDOWN;
                if let Some(state) = STATE.get() {
                    let mut state = state.lock().unwrap();
                    let hotkey = state.hotkey;
                    match state.matcher.on_key(&hotkey, event.vkCode, down, current_modifiers()) {
                        Decision::PassThrough => {}
                        Decision::Swallow => return LRESULT(1),
                        Decision::Fire => {
                            let _ = state.fired.send(());
                            return LRESULT(1);
                        }
                    }
                }
            }
        }
        unsafe { CallNextHookEx(None, code, wparam, lparam) }
    }

    pub fn install(hotkey: Hotkey, on_fire: Box<dyn Fn() + Send>) -> Result<(), String> {
        let (fired, fired_rx) = channel();
        STATE
            .set(Mutex::new(State { hotkey, matcher: Matcher::default(), fired }))
            .map_err(|_| "keyboard hook already installed".to_string())?;
        std::thread::Builder::new()
            .name("openray-hotkey-dispatch".into())
            .spawn(move || {
                for () in fired_rx {
                    if hotkey.modifiers.alt || hotkey.modifiers.win {
                        unsafe {
                            keybd_event(VK_UNASSIGNED, 0, Default::default(), MARKER);
                            keybd_event(VK_UNASSIGNED, 0, KEYEVENTF_KEYUP, MARKER);
                        }
                    }
                    on_fire();
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

pub use hook::install;

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
        assert!(parse("Alt+").is_err());
        assert!(parse("Alt+Banana").is_err());
        assert!(parse("Alt+A+B").is_err());
        assert!(parse("Alt+F25").is_err());
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
