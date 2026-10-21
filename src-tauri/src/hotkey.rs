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
        "PRINTSCREEN" | "PRTSC" | "SNAPSHOT" => 0x2C,
        "PAUSE" => 0x13,
        "SCROLLLOCK" => 0x91,
        "DELETE" => 0x2E,
        _ => match upper.strip_prefix("NUMPAD").and_then(|n| n.parse::<u32>().ok()) {
            Some(n) if n <= 9 => 0x60 + n,
            _ => return None,
        },
    })
}

pub const START_MENU: u32 = 0x5B;

pub fn standalone(vk: u32) -> bool {
    matches!(vk, 0x2C | 0x13 | 0x91 | 0x7C..=0x87)
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
    if vk.is_none() && modifiers == (Modifiers { win: true, ..Default::default() }) {
        return Ok(Hotkey { modifiers: Modifiers::default(), vk: START_MENU });
    }
    let vk = vk.ok_or_else(|| format!("hotkey \"{text}\" has no key"))?;
    if modifiers == Modifiers::default() && !standalone(vk) {
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
    use super::{Decision, Hotkey, Matcher, Modifiers, START_MENU};
    use std::io::{BufRead, BufReader, Write};
    use std::os::windows::process::CommandExt;
    use std::process::{ChildStdin, ChildStdout, Command, Stdio};
    use std::sync::mpsc::{channel, Sender};
    use std::sync::{Mutex, OnceLock};
    use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::System::Threading::{GetCurrentThread, SetThreadPriority, THREAD_PRIORITY_TIME_CRITICAL};
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP, VIRTUAL_KEY,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, GetMessageW, SetWindowsHookExW, HOOKPROC, KBDLLHOOKSTRUCT, MSG, WH_KEYBOARD_LL, WM_KEYDOWN, WM_SYSKEYDOWN,
    };

    pub const MARKER: usize = 0x4F50_524E;
    pub const HELPER_ARG: &str = "--keyboard-hook";
    const VK_UNASSIGNED: u16 = 0xE8;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    struct State {
        bindings: Vec<(String, Hotkey, Matcher)>,
        suspended: bool,
        win: WinTap,
        fired: Sender<(String, Hotkey)>,
    }

    #[derive(Default, PartialEq)]
    enum WinTap {
        #[default]
        Idle,
        Pending,
        Combo,
    }

    fn is_win(vk: u32) -> bool {
        vk == 0x5B || vk == 0x5C
    }

    fn start_menu(state: &mut State, event: &KBDLLHOOKSTRUCT, down: bool) -> bool {
        let Some(id) = state.bindings.iter().find(|(_, h, _)| h.vk == START_MENU && h.modifiers == Modifiers::default()).map(|(id, _, _)| id.clone()) else {
            state.win = WinTap::Idle;
            return false;
        };
        match (is_win(event.vkCode), down, &state.win) {
            (true, true, WinTap::Idle) => {
                state.win = WinTap::Pending;
                send(&[key(0xFF, KEYBD_EVENT_FLAGS(0)), key(0xFF, KEYEVENTF_KEYUP)]);
                true
            }
            (true, true, WinTap::Pending) => true,
            (false, true, WinTap::Pending) => {
                state.win = WinTap::Combo;
                send(&[key(0x5B, KEYBD_EVENT_FLAGS(0)), key(event.vkCode as u16, KEYBD_EVENT_FLAGS(0))]);
                true
            }
            (true, false, WinTap::Pending) => {
                state.win = WinTap::Idle;
                let _ = state.fired.send((id, Hotkey { modifiers: Modifiers::default(), vk: START_MENU }));
                true
            }
            (true, false, WinTap::Combo) => {
                state.win = WinTap::Idle;
                false
            }
            _ => false,
        }
    }

    fn key(vk: u16, flags: KEYBD_EVENT_FLAGS) -> INPUT {
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: VIRTUAL_KEY(vk), dwFlags: flags, dwExtraInfo: MARKER, ..Default::default() } },
        }
    }

    fn send(inputs: &[INPUT]) {
        unsafe { SendInput(inputs, std::mem::size_of::<INPUT>() as i32) };
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
        for (id, hotkey, matcher) in state.bindings.iter_mut().filter(|(_, h, _)| h.vk != START_MENU) {
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

    fn event<'a>(code: i32, wparam: WPARAM, lparam: LPARAM) -> Option<(&'a KBDLLHOOKSTRUCT, bool)> {
        let event = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
        (code >= 0 && event.dwExtraInfo != MARKER).then(|| (event, wparam.0 as u32 == WM_KEYDOWN || wparam.0 as u32 == WM_SYSKEYDOWN))
    }

    unsafe extern "system" fn hotkey_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if let (Some((event, down)), Some(state)) = (event(code, wparam, lparam), STATE.get()) {
            let mut state = state.lock().unwrap();
            if (!state.suspended && start_menu(&mut state, event, down)) || decide(&mut state, event.vkCode, down) {
                return LRESULT(1);
            }
        }
        unsafe { CallNextHookEx(None, code, wparam, lparam) }
    }

    unsafe extern "system" fn snippet_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if let Some((event, true)) = event(code, wparam, lparam) {
            crate::snippets::on_key(event.vkCode, event.scanCode);
        }
        unsafe { CallNextHookEx(None, code, wparam, lparam) }
    }

    fn hook_thread(name: &str, proc: HOOKPROC) -> Result<(), String> {
        let (tx, rx) = channel();
        std::thread::Builder::new()
            .name(name.into())
            .spawn(move || unsafe {
                let _ = SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_TIME_CRITICAL);
                let module = GetModuleHandleW(None).ok();
                match SetWindowsHookExW(WH_KEYBOARD_LL, proc, module.map(Into::into), 0) {
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

    fn encode(bindings: &[(String, Hotkey)]) -> String {
        let rows: Vec<_> = bindings
            .iter()
            .map(|(id, h)| serde_json::json!([id, [h.modifiers.ctrl, h.modifiers.alt, h.modifiers.shift, h.modifiers.win], h.vk]))
            .collect();
        format!("B{}", serde_json::Value::Array(rows))
    }

    fn decode(json: &str) -> Vec<(String, Hotkey)> {
        let rows: Vec<(String, [bool; 4], u32)> = serde_json::from_str(json).unwrap_or_default();
        rows.into_iter()
            .map(|(id, [ctrl, alt, shift, win], vk)| (id, Hotkey { modifiers: Modifiers { ctrl, alt, shift, win }, vk }))
            .collect()
    }

    #[cfg(test)]
    #[test]
    fn bindings_survive_the_helper_protocol() {
        let bindings = vec![("launcher".to_string(), Hotkey { modifiers: Modifiers::default(), vk: START_MENU }), ("a/b".to_string(), super::parse("Ctrl+Shift+K").unwrap())];
        assert_eq!(decode(encode(&bindings).strip_prefix('B').unwrap()), bindings);
    }

    pub fn run_helper() {
        let (fired, fired_rx) = channel::<(String, Hotkey)>();
        let _ = STATE.set(Mutex::new(State { bindings: Vec::new(), suspended: false, win: WinTap::Idle, fired }));
        std::thread::spawn(move || {
            let mut out = std::io::stdout();
            for (id, hotkey) in fired_rx {
                if hotkey.modifiers.alt || hotkey.modifiers.win {
                    send(&[key(VK_UNASSIGNED, KEYBD_EVENT_FLAGS(0)), key(VK_UNASSIGNED, KEYEVENTF_KEYUP)]);
                }
                if writeln!(out, "{id}").and_then(|()| out.flush()).is_err() {
                    std::process::exit(0);
                }
            }
        });
        if hook_thread("openray-keyboard-hook", Some(hotkey_proc)).is_err() {
            std::process::exit(1);
        }
        for line in std::io::stdin().lock().lines().map_while(Result::ok) {
            let line = line.trim_start_matches('\u{feff}').trim();
            let mut state = STATE.get().unwrap().lock().unwrap();
            if let Some(json) = line.strip_prefix('B') {
                state.bindings = decode(json).into_iter().map(|(id, hotkey)| (id, hotkey, Matcher::default())).collect();
            } else if let Some(flag) = line.strip_prefix('S') {
                state.suspended = flag == "1";
            }
        }
    }

    struct Helper {
        stdin: Option<ChildStdin>,
        bindings: String,
        suspended: bool,
    }

    static HELPER: Mutex<Helper> = Mutex::new(Helper { stdin: None, bindings: String::new(), suspended: false });

    fn write(line: &str) {
        let mut helper = HELPER.lock().unwrap();
        if let Some(stdin) = helper.stdin.as_mut() {
            if writeln!(stdin, "{line}").and_then(|()| stdin.flush()).is_err() {
                helper.stdin = None;
            }
        }
    }

    pub fn set_bindings(bindings: Vec<(String, Hotkey)>) {
        let line = encode(&bindings);
        HELPER.lock().unwrap().bindings = line.clone();
        write(&line);
    }

    pub fn suspend(suspended: bool) {
        HELPER.lock().unwrap().suspended = suspended;
        write(if suspended { "S1" } else { "S0" });
    }

    // Windows skips an in-process low-level hook for about a second after this process takes the foreground, so the
    // swallowing hook runs in a helper process, like the reference's UIAccess helper.
    fn spawn_helper(exe: &std::path::Path) -> std::io::Result<ChildStdout> {
        let mut child = Command::new(exe).arg(HELPER_ARG).stdin(Stdio::piped()).stdout(Stdio::piped()).creation_flags(CREATE_NO_WINDOW).spawn()?;
        let stdout = child.stdout.take().ok_or_else(|| std::io::Error::other("no helper stdout"))?;
        let replay = {
            let mut helper = HELPER.lock().unwrap();
            helper.stdin = child.stdin.take();
            [helper.bindings.clone(), (if helper.suspended { "S1" } else { "S0" }).to_string()]
        };
        for line in replay.iter().filter(|l| !l.is_empty()) {
            write(line);
        }
        Ok(stdout)
    }

    pub fn install(on_fire: Box<dyn Fn(String) + Send>) -> Result<(), String> {
        hook_thread("openray-snippet-hook", Some(snippet_proc))?;
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let first = spawn_helper(&exe).map_err(|e| e.to_string())?;
        std::thread::Builder::new()
            .name("openray-hotkey-helper".into())
            .spawn(move || {
                let mut stdout = Some(first);
                loop {
                    if let Some(out) = stdout.take() {
                        for id in BufReader::new(out).lines().map_while(Result::ok) {
                            on_fire(id);
                        }
                    }
                    std::thread::sleep(std::time::Duration::from_secs(1));
                    stdout = spawn_helper(&exe).ok();
                }
            })
            .map_err(|e| e.to_string())?;
        Ok(())
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

pub use hook::{install, run_helper, set_bindings, suspend, HELPER_ARG, MARKER};

#[cfg(test)]
mod tests {
    use super::*;

    fn mods(ctrl: bool, alt: bool, shift: bool, win: bool) -> Modifiers {
        Modifiers { ctrl, alt, shift, win }
    }

    #[test]
    fn win_alone_means_replace_start_menu() {
        assert_eq!(parse("Win").unwrap(), Hotkey { modifiers: Modifiers::default(), vk: START_MENU });
        assert!(parse("Ctrl").is_err());
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
        assert!(parse("F1").is_err());
    }

    #[test]
    fn accepts_print_screen_with_or_without_modifiers() {
        assert_eq!(parse("PrintScreen").unwrap(), Hotkey { modifiers: Modifiers::default(), vk: 0x2C });
        assert_eq!(parse("Alt+PrintScreen").unwrap(), Hotkey { modifiers: mods(false, true, false, false), vk: 0x2C });
        assert_eq!(parse("F13").unwrap().vk, 0x7C);
        assert_eq!(parse("Pause").unwrap().vk, 0x13);
        let plain = parse("PrintScreen").unwrap();
        let mut m = Matcher::default();
        assert_eq!(m.on_key(&plain, 0x2C, true, mods(false, true, false, false)), Decision::PassThrough);
        assert_eq!(m.on_key(&plain, 0x2C, true, Modifiers::default()), Decision::Fire);
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
