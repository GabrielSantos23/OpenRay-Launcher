use crate::launcher;
use std::os::windows::process::CommandExt;
use tauri::AppHandle;
use windows::core::{w, BOOL, PCWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED};
use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
use windows::Win32::Media::Audio::{eCapture, eMultimedia, eRender, EDataFlow, IMMDeviceEnumerator, MMDeviceEnumerator, DEVICE_STATE_ACTIVE};
use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_APARTMENTTHREADED};
use windows::Win32::System::Power::SetSuspendState;
use windows::Win32::System::Registry::{RegGetValueW, RegSetKeyValueW, HKEY_CURRENT_USER, REG_DWORD, RRF_RT_REG_DWORD};
use windows::Win32::System::Shutdown::{ExitWindowsEx, LockWorkStation, EWX_LOGOFF, SHTDN_REASON_FLAG_PLANNED};
use windows::Win32::System::Threading::GetCurrentProcessId;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VIRTUAL_KEY, VK_D, VK_LWIN, VK_MEDIA_NEXT_TRACK, VK_MEDIA_PLAY_PAUSE,
    VK_MEDIA_PREV_TRACK,
};
use windows::Win32::UI::Shell::{SHChangeNotify, SHEmptyRecycleBinW, SHCNE_ASSOCCHANGED, SHCNF_IDLIST, SHERB_NOSOUND};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetClassNameW, GetDesktopWindow, GetWindow, GetWindowLongPtrW, GetWindowTextLengthW, GetWindowThreadProcessId, IsWindowVisible,
    PostMessageW, SendMessageTimeoutW, GWL_EXSTYLE, GW_OWNER, HWND_BROADCAST, SC_MONITORPOWER, SMTO_ABORTIFHUNG,
    WM_CLOSE, WM_SETTINGCHANGE, WM_SYSCOMMAND, WS_EX_TOOLWINDOW,
};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const SC_SCREENSAVE: usize = 0xF140;
const PERSONALIZE: PCWSTR = w!(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize");
const EXPLORER_ADVANCED: PCWSTR = w!(r"Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced");
const SHELL_CLASSES: [&str; 3] = ["Progman", "WorkerW", "Shell_TrayWnd"];

fn message(e: windows::core::Error) -> String {
    e.message()
}

pub fn volume_preset(action: &str) -> Option<f32> {
    let percent: u8 = action.strip_prefix("set-volume-")?.parse().ok()?;
    (percent <= 100).then(|| f32::from(percent) / 100.0)
}

pub fn next_mute(states: &[bool]) -> bool {
    !states.iter().all(|&muted| muted)
}

fn keys(sequence: &[VIRTUAL_KEY]) {
    let key = |vk: VIRTUAL_KEY, up: bool| INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT { wVk: vk, dwFlags: if up { KEYEVENTF_KEYUP } else { Default::default() }, dwExtraInfo: crate::hotkey::MARKER, ..Default::default() },
        },
    };
    let inputs: Vec<INPUT> = sequence.iter().map(|&vk| key(vk, false)).chain(sequence.iter().rev().map(|&vk| key(vk, true))).collect();
    unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
}

fn shutdown(flag: &str) -> Result<(), String> {
    std::process::Command::new("shutdown.exe").args([flag, "/t", "0"]).creation_flags(CREATE_NO_WINDOW).spawn().map(|_| ()).map_err(|e| e.to_string())
}

fn after_hide(app: &AppHandle, action: impl FnOnce() + Send + 'static) {
    launcher::hide(app, false);
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(250));
        action();
    });
}

fn enumerator() -> Result<IMMDeviceEnumerator, String> {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).map_err(message)
    }
}

fn default_volume(flow: EDataFlow) -> Result<IAudioEndpointVolume, String> {
    unsafe {
        let device = enumerator()?.GetDefaultAudioEndpoint(flow, eMultimedia).map_err(|_| "No audio device found".to_string())?;
        device.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None).map_err(message)
    }
}

fn speaker(change: impl FnOnce(&IAudioEndpointVolume) -> windows::core::Result<()>) -> Result<(), String> {
    change(&default_volume(eRender)?).map_err(message)
}

fn toggle_microphones() -> Result<(), String> {
    let mut volumes = Vec::new();
    unsafe {
        let devices = enumerator()?.EnumAudioEndpoints(eCapture, DEVICE_STATE_ACTIVE).map_err(message)?;
        for i in 0..devices.GetCount().map_err(message)? {
            volumes.push(devices.Item(i).and_then(|d| d.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None)).map_err(message)?);
        }
    }
    if volumes.is_empty() {
        return Err("No microphone found".into());
    }
    let states: Vec<bool> = volumes.iter().map(|v| unsafe { v.GetMute() }.is_ok_and(|m| m.as_bool())).collect();
    let mute = next_mute(&states);
    for volume in &volumes {
        unsafe { volume.SetMute(mute, std::ptr::null()) }.map_err(message)?;
    }
    Ok(())
}

fn read_dword(key: PCWSTR, name: PCWSTR) -> Option<u32> {
    let mut value = 0u32;
    let mut size = 4u32;
    unsafe { RegGetValueW(HKEY_CURRENT_USER, key, name, RRF_RT_REG_DWORD, None, Some(&mut value as *mut u32 as _), Some(&mut size)) }.ok().ok()?;
    Some(value)
}

fn write_dword(key: PCWSTR, name: PCWSTR, value: u32) -> Result<(), String> {
    unsafe { RegSetKeyValueW(HKEY_CURRENT_USER, key, name, REG_DWORD.0, Some(&value as *const u32 as _), 4) }.ok().map_err(message)
}

fn broadcast_setting(area: PCWSTR) {
    unsafe {
        let _ = SendMessageTimeoutW(HWND_BROADCAST, WM_SETTINGCHANGE, WPARAM(0), LPARAM(area.0 as isize), SMTO_ABORTIFHUNG, 200, None);
    }
}

fn toggle_dark_mode() -> Result<(), String> {
    let light = read_dword(PERSONALIZE, w!("AppsUseLightTheme")).unwrap_or(1);
    let next = u32::from(light == 0);
    write_dword(PERSONALIZE, w!("AppsUseLightTheme"), next)?;
    write_dword(PERSONALIZE, w!("SystemUsesLightTheme"), next)?;
    broadcast_setting(w!("ImmersiveColorSet"));
    Ok(())
}

fn toggle_hidden_files() -> Result<(), String> {
    let shown = read_dword(EXPLORER_ADVANCED, w!("Hidden")) == Some(1);
    write_dword(EXPLORER_ADVANCED, w!("Hidden"), if shown { 2 } else { 1 })?;
    unsafe { SHChangeNotify(SHCNE_ASSOCCHANGED, SHCNF_IDLIST, None, None) };
    broadcast_setting(PCWSTR::null());
    Ok(())
}

fn process_of(hwnd: HWND) -> u32 {
    let mut pid = 0;
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    pid
}

fn app_windows() -> Vec<HWND> {
    unsafe extern "system" fn collect(hwnd: HWND, data: LPARAM) -> BOOL {
        let list = unsafe { &mut *(data.0 as *mut Vec<HWND>) };
        list.push(hwnd);
        true.into()
    }
    let mut all: Vec<HWND> = Vec::new();
    unsafe {
        let _ = EnumWindows(Some(collect), LPARAM(&mut all as *mut _ as isize));
    }
    let own = unsafe { GetCurrentProcessId() };
    all.into_iter()
        .filter(|&hwnd| unsafe {
            let mut cloaked = 0u32;
            let _ = DwmGetWindowAttribute(hwnd, DWMWA_CLOAKED, &mut cloaked as *mut u32 as _, 4);
            let mut class = [0u16; 64];
            let len = GetClassNameW(hwnd, &mut class) as usize;
            let class = String::from_utf16_lossy(&class[..len]);
            IsWindowVisible(hwnd).as_bool()
                && cloaked == 0
                && GetWindow(hwnd, GW_OWNER).is_err()
                && GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32 & WS_EX_TOOLWINDOW.0 == 0
                && GetWindowTextLengthW(hwnd) > 0
                && !SHELL_CLASSES.contains(&class.as_str())
                && process_of(hwnd) != own
        })
        .collect()
}

fn close(hwnd: HWND) {
    unsafe {
        let _ = PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0));
    }
}

fn frontmost_process() -> Option<u32> {
    let hwnd = launcher::active_window();
    (!hwnd.is_invalid()).then(|| process_of(hwnd))
}

fn quit_all(keep: Option<u32>) {
    app_windows().into_iter().filter(|&hwnd| keep != Some(process_of(hwnd))).for_each(close);
}

pub fn run(app: &AppHandle, action: &str) -> Result<(), String> {
    if let Some(level) = volume_preset(action) {
        return speaker(|v| unsafe {
            v.SetMute(false, std::ptr::null())?;
            v.SetMasterVolumeLevelScalar(level, std::ptr::null())
        });
    }
    match action {
        "lock-screen" => unsafe { LockWorkStation() }.map_err(message),
        "log-out" => unsafe { ExitWindowsEx(EWX_LOGOFF, SHTDN_REASON_FLAG_PLANNED) }.map_err(message),
        "restart" => shutdown("/r"),
        "shut-down" => shutdown("/s"),
        "sleep" | "hibernate" => {
            let hibernate = action == "hibernate";
            after_hide(app, move || unsafe {
                let _ = SetSuspendState(hibernate, false, false);
            });
            Ok(())
        }
        "sleep-displays" => {
            after_hide(app, || unsafe {
                let _ = PostMessageW(Some(HWND_BROADCAST), WM_SYSCOMMAND, WPARAM(SC_MONITORPOWER as usize), LPARAM(2));
            });
            Ok(())
        }
        "show-screen-saver" => {
            after_hide(app, || unsafe {
                let _ = PostMessageW(Some(GetDesktopWindow()), WM_SYSCOMMAND, WPARAM(SC_SCREENSAVE), LPARAM(0));
            });
            Ok(())
        }
        "show-desktop" => {
            after_hide(app, || keys(&[VK_LWIN, VK_D]));
            Ok(())
        }
        "open-recycle-bin" => crate::apps::open_url("shell:RecycleBinFolder", None),
        "empty-recycle-bin" => match unsafe { SHEmptyRecycleBinW(None, PCWSTR::null(), SHERB_NOSOUND) } {
            Err(e) if e.code().0 as u32 == 0x8000_FFFF => Ok(()),
            result => result.map_err(message),
        },
        "quit-active-app" => {
            let hwnd = launcher::active_window();
            if hwnd.is_invalid() {
                return Err("No active window".into());
            }
            close(hwnd);
            Ok(())
        }
        "quit-all-apps" => {
            quit_all(None);
            Ok(())
        }
        "quit-all-apps-except-frontmost" => {
            quit_all(frontmost_process());
            Ok(())
        }
        "turn-volume-up" => speaker(|v| unsafe { v.VolumeStepUp(std::ptr::null()) }),
        "turn-volume-down" => speaker(|v| unsafe { v.VolumeStepDown(std::ptr::null()) }),
        "toggle-mute" => speaker(|v| unsafe { v.SetMute(!v.GetMute()?.as_bool(), std::ptr::null()) }),
        "toggle-microphone-mute" => toggle_microphones(),
        "play-pause" => Ok(keys(&[VK_MEDIA_PLAY_PAUSE])),
        "next-track" => Ok(keys(&[VK_MEDIA_NEXT_TRACK])),
        "previous-track" => Ok(keys(&[VK_MEDIA_PREV_TRACK])),
        "toggle-dark-mode" => toggle_dark_mode(),
        "toggle-hidden-files" => toggle_hidden_files(),
        _ => Err(format!("Unknown system action {action}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn volume_presets_parse_percentages() {
        assert_eq!(volume_preset("set-volume-0"), Some(0.0));
        assert_eq!(volume_preset("set-volume-75"), Some(0.75));
        assert_eq!(volume_preset("set-volume-100"), Some(1.0));
        assert_eq!(volume_preset("set-volume-101"), None);
        assert_eq!(volume_preset("toggle-mute"), None);
    }

    #[test]
    fn microphones_mute_unless_all_are_already_muted() {
        assert!(next_mute(&[false, false]));
        assert!(next_mute(&[true, false]));
        assert!(!next_mute(&[true, true]));
    }

    #[test]
    #[ignore = "changes the real speaker volume and microphone mute, then restores them"]
    fn speaker_and_microphone_round_trip() {
        let speaker_volume = default_volume(eRender).unwrap();
        let (level, muted) = unsafe { (speaker_volume.GetMasterVolumeLevelScalar().unwrap(), speaker_volume.GetMute().unwrap().as_bool()) };
        speaker(|v| unsafe { v.SetMasterVolumeLevelScalar(0.25, std::ptr::null()) }).unwrap();
        assert!((unsafe { speaker_volume.GetMasterVolumeLevelScalar() }.unwrap() - 0.25).abs() < 0.01);
        speaker(|v| unsafe { v.SetMute(!v.GetMute()?.as_bool(), std::ptr::null()) }).unwrap();
        assert_eq!(unsafe { speaker_volume.GetMute() }.unwrap().as_bool(), !muted);
        unsafe {
            speaker_volume.SetMasterVolumeLevelScalar(level, std::ptr::null()).unwrap();
            speaker_volume.SetMute(muted, std::ptr::null()).unwrap();
        }
        if let Ok(mic) = default_volume(eCapture) {
            let before = unsafe { mic.GetMute() }.unwrap().as_bool();
            toggle_microphones().unwrap();
            toggle_microphones().unwrap();
            assert_eq!(unsafe { mic.GetMute() }.unwrap().as_bool(), before);
        }
    }

    #[test]
    #[ignore = "toggles dark mode and hidden files twice on the real system"]
    fn registry_toggles_round_trip() {
        let theme = read_dword(PERSONALIZE, w!("AppsUseLightTheme"));
        let hidden = read_dword(EXPLORER_ADVANCED, w!("Hidden"));
        toggle_dark_mode().unwrap();
        assert_ne!(read_dword(PERSONALIZE, w!("AppsUseLightTheme")), theme);
        toggle_dark_mode().unwrap();
        assert_eq!(read_dword(PERSONALIZE, w!("AppsUseLightTheme")), theme);
        toggle_hidden_files().unwrap();
        toggle_hidden_files().unwrap();
        assert_eq!(read_dword(EXPLORER_ADVANCED, w!("Hidden")).map(|v| v == 1), hidden.map(|v| v == 1));
    }

    #[test]
    #[ignore = "lists the real top-level windows"]
    fn app_windows_skip_the_shell_and_tool_windows() {
        let windows = app_windows();
        assert!(!windows.is_empty());
        for hwnd in windows {
            let mut class = [0u16; 64];
            let len = unsafe { GetClassNameW(hwnd, &mut class) } as usize;
            assert!(!SHELL_CLASSES.contains(&String::from_utf16_lossy(&class[..len]).as_str()));
        }
    }
}
