use crate::{clipboard, launcher, window_switcher};
use serde::Serialize;
use std::collections::BTreeMap;
use windows::core::Interface;
use windows::Win32::Media::Audio::{
    eMultimedia, eRender, AudioSessionStateActive, IAudioSessionControl2, IAudioSessionManager2, IMMDeviceEnumerator, ISimpleAudioVolume, MMDeviceEnumerator,
};
use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_APARTMENTTHREADED};

const SYSTEM: &str = "system";

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AppAudio {
    id: String,
    name: String,
    path: Option<String>,
    volume: f32,
    is_muted: bool,
    is_playing: bool,
}

struct Session {
    key: String,
    name: String,
    path: Option<String>,
    volume: ISimpleAudioVolume,
    playing: bool,
}

fn message(e: windows::core::Error) -> String {
    e.message()
}

fn sessions() -> Result<Vec<Session>, String> {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let enumerator: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).map_err(message)?;
        let device = enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia).map_err(|_| "No audio device found".to_string())?;
        let manager = device.Activate::<IAudioSessionManager2>(CLSCTX_ALL, None).map_err(message)?;
        let list = manager.GetSessionEnumerator().map_err(message)?;
        let mut result = Vec::new();
        for i in 0..list.GetCount().map_err(message)? {
            let Ok(control) = list.GetSession(i) else { continue };
            let Ok(control) = control.cast::<IAudioSessionControl2>() else { continue };
            let Ok(volume) = control.cast::<ISimpleAudioVolume>() else { continue };
            let playing = control.GetState().is_ok_and(|s| s == AudioSessionStateActive);
            if control.IsSystemSoundsSession() == windows::Win32::Foundation::S_OK {
                result.push(Session { key: SYSTEM.into(), name: "System Sounds".into(), path: None, volume, playing });
                continue;
            }
            let Ok(pid) = control.GetProcessId() else { continue };
            let Some((name, path)) = clipboard::process_app(pid) else { continue };
            result.push(Session { key: path.to_lowercase(), name, path: Some(path), volume, playing });
        }
        Ok(result)
    }
}

fn group(sessions: &[Session]) -> Vec<AppAudio> {
    let mut apps: BTreeMap<String, AppAudio> = BTreeMap::new();
    for s in sessions {
        let (level, muted) = unsafe { (s.volume.GetMasterVolume().unwrap_or(0.0), s.volume.GetMute().is_ok_and(|m| m.as_bool())) };
        let app = apps.entry(s.key.clone()).or_insert_with(|| AppAudio {
            id: s.key.clone(),
            name: s.name.clone(),
            path: s.path.clone(),
            volume: 0.0,
            is_muted: true,
            is_playing: false,
        });
        app.volume = app.volume.max((level * 100.0).round());
        app.is_muted &= muted;
        app.is_playing |= s.playing;
    }
    apps.into_values().collect()
}

pub fn list() -> Result<Vec<AppAudio>, String> {
    Ok(group(&sessions()?))
}

pub fn set(id: &str, volume: Option<f32>, muted: Option<bool>) -> Result<(), String> {
    let matching: Vec<Session> = sessions()?.into_iter().filter(|s| s.key == id).collect();
    if matching.is_empty() {
        return Err("The app is no longer playing audio".into());
    }
    for s in &matching {
        unsafe {
            if let Some(volume) = volume {
                if volume > 0.0 {
                    s.volume.SetMute(false, std::ptr::null()).map_err(message)?;
                }
                s.volume.SetMasterVolume((volume / 100.0).clamp(0.0, 1.0), std::ptr::null()).map_err(message)?;
            }
            if let Some(muted) = muted {
                s.volume.SetMute(muted, std::ptr::null()).map_err(message)?;
            }
        }
    }
    Ok(())
}

pub fn step(volume: f32, up: bool) -> f32 {
    let fifths = volume / 5.0;
    let next = if up { (fifths.floor() + 1.0) * 5.0 } else { (fifths.ceil() - 1.0) * 5.0 };
    next.clamp(0.0, 100.0)
}

pub fn change_active(action: &str) -> Result<String, String> {
    let hwnd = launcher::active_window();
    let (app_name, path) = (!hwnd.is_invalid()).then(|| clipboard::process_app(window_switcher::app_pid(hwnd))).flatten().ok_or("The active app has no audio session")?;
    let app = list()?.into_iter().find(|a| a.id == path.to_lowercase()).ok_or("The active app has no audio session")?;
    match action {
        "mute" => {
            set(&app.id, None, Some(!app.is_muted))?;
            Ok(format!("{app_name} {}", if app.is_muted { "unmuted" } else { "muted" }))
        }
        "up" | "down" => {
            let volume = step(app.volume, action == "up");
            set(&app.id, Some(volume), None)?;
            Ok(format!("{app_name} volume set to {volume}%"))
        }
        _ => Err(format!("Unknown action {action}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_snap_to_multiples_of_five() {
        assert_eq!(step(80.0, true), 85.0);
        assert_eq!(step(82.0, true), 85.0);
        assert_eq!(step(82.0, false), 80.0);
        assert_eq!(step(80.0, false), 75.0);
        assert_eq!(step(98.0, true), 100.0);
        assert_eq!(step(100.0, true), 100.0);
        assert_eq!(step(0.0, false), 0.0);
    }

    #[test]
    #[ignore = "reads the live audio sessions"]
    fn lists_audio_sessions() {
        for app in list().unwrap() {
            println!("{} | {:?} | vol={} muted={} playing={}", app.name, app.path, app.volume, app.is_muted, app.is_playing);
        }
    }
}
