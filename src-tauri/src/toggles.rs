use windows::core::{w, PCWSTR};
use windows::Win32::Devices::Display::{
    DisplayConfigGetDeviceInfo, DisplayConfigSetDeviceInfo, GetDisplayConfigBufferSizes, QueryDisplayConfig, DISPLAYCONFIG_DEVICE_INFO_GET_ADVANCED_COLOR_INFO,
    DISPLAYCONFIG_DEVICE_INFO_HEADER, DISPLAYCONFIG_DEVICE_INFO_SET_ADVANCED_COLOR_STATE, DISPLAYCONFIG_GET_ADVANCED_COLOR_INFO, DISPLAYCONFIG_MODE_INFO,
    DISPLAYCONFIG_PATH_INFO, DISPLAYCONFIG_SET_ADVANCED_COLOR_STATE, QDC_ONLY_ACTIVE_PATHS,
};
use windows::Win32::Foundation::ERROR_SUCCESS;
use windows::Win32::System::Registry::{RegGetValueW, RegSetKeyValueW, HKEY_CURRENT_USER, REG_BINARY, REG_SZ, RRF_RT_REG_BINARY, RRF_RT_REG_SZ};

const NIGHT_LIGHT: PCWSTR = w!(
    r"Software\Microsoft\Windows\CurrentVersion\CloudStore\Store\DefaultAccount\Current\default$windows.data.bluelightreduction.bluelightreductionstate\windows.data.bluelightreduction.bluelightreductionstate"
);
const WEBCAM: PCWSTR = w!(r"Software\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\webcam");
const HEADER: [u8; 4] = [0x43, 0x42, 0x01, 0x00];
const ON_MARK: [u8; 2] = [0x02, 0x01];

fn read_value(key: PCWSTR, name: PCWSTR, kind: windows::Win32::System::Registry::REG_ROUTINE_FLAGS) -> Option<Vec<u8>> {
    let mut size = 0u32;
    unsafe {
        if RegGetValueW(HKEY_CURRENT_USER, key, name, kind, None, None, Some(&mut size)) != ERROR_SUCCESS {
            return None;
        }
        let mut data = vec![0u8; size as usize];
        (RegGetValueW(HKEY_CURRENT_USER, key, name, kind, None, Some(data.as_mut_ptr() as _), Some(&mut size)) == ERROR_SUCCESS).then(|| {
            data.truncate(size as usize);
            data
        })
    }
}

fn varint(mut value: u64) -> Vec<u8> {
    let mut out = Vec::new();
    loop {
        let byte = (value & 0x7f) as u8;
        value >>= 7;
        if value == 0 {
            out.push(byte);
            return out;
        }
        out.push(byte | 0x80);
    }
}

pub fn toggle_night_light_blob(data: &[u8], now_secs: u64) -> Option<(Vec<u8>, bool)> {
    if data.len() < 12 || data[..4] != HEADER || data[8..10] != [0x2a, 0x06] {
        return None;
    }
    let stamp_end = 10 + data[10..].iter().position(|b| b & 0x80 == 0)? + 1;
    let rest = &data[stamp_end..];
    if rest.len() < 10 || rest[..3] != [0x2a, 0x2b, 0x0e] || rest[4..8] != HEADER {
        return None;
    }
    let enabled = rest[8..10] == ON_MARK;
    let mut out = data[..10].to_vec();
    out.extend(varint(now_secs));
    out.extend(&rest[..3]);
    out.push(if enabled { rest[3].checked_sub(2)? } else { rest[3] + 2 });
    out.extend(&rest[4..8]);
    if !enabled {
        out.extend(ON_MARK);
    }
    out.extend(&rest[if enabled { 10 } else { 8 }..]);
    Some((out, !enabled))
}

fn toggle_night_light() -> Result<String, String> {
    let data = read_value(NIGHT_LIGHT, w!("Data"), RRF_RT_REG_BINARY).ok_or("Night Light has never been set up on this PC")?;
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|e| e.to_string())?.as_secs();
    let (next, on) = toggle_night_light_blob(&data, now).ok_or("Unknown Night Light format")?;
    let status = unsafe { RegSetKeyValueW(HKEY_CURRENT_USER, NIGHT_LIGHT, w!("Data"), REG_BINARY.0, Some(next.as_ptr() as _), next.len() as u32) };
    if status != ERROR_SUCCESS {
        return Err("Unable to toggle Night Light".into());
    }
    Ok(if on { "Night Light On" } else { "Night Light Off" }.into())
}

fn toggle_camera() -> Result<String, String> {
    let current = read_value(WEBCAM, w!("Value"), RRF_RT_REG_SZ).map(|b| String::from_utf16_lossy(&b.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).take_while(|&c| c != 0).collect::<Vec<_>>()));
    let mute = current.as_deref() != Some("Deny");
    let value: Vec<u16> = if mute { "Deny" } else { "Allow" }.encode_utf16().chain([0]).collect();
    let status = unsafe { RegSetKeyValueW(HKEY_CURRENT_USER, WEBCAM, w!("Value"), REG_SZ.0, Some(value.as_ptr() as _), (value.len() * 2) as u32) };
    if status != ERROR_SUCCESS {
        return Err("Unable to toggle camera mute".into());
    }
    Ok(if mute { "Camera muted" } else { "Camera unmuted" }.into())
}

fn toggle_hdr() -> Result<String, String> {
    let (mut paths, mut modes) = (0u32, 0u32);
    let mut targets = Vec::new();
    unsafe {
        if GetDisplayConfigBufferSizes(QDC_ONLY_ACTIVE_PATHS, &mut paths, &mut modes) != ERROR_SUCCESS {
            return Err("Unable to toggle HDR".into());
        }
        let mut path_info = vec![DISPLAYCONFIG_PATH_INFO::default(); paths as usize];
        let mut mode_info = vec![DISPLAYCONFIG_MODE_INFO::default(); modes as usize];
        if QueryDisplayConfig(QDC_ONLY_ACTIVE_PATHS, &mut paths, path_info.as_mut_ptr(), &mut modes, mode_info.as_mut_ptr(), None) != ERROR_SUCCESS {
            return Err("Unable to toggle HDR".into());
        }
        for path in &path_info[..paths as usize] {
            let mut info = DISPLAYCONFIG_GET_ADVANCED_COLOR_INFO {
                header: DISPLAYCONFIG_DEVICE_INFO_HEADER {
                    r#type: DISPLAYCONFIG_DEVICE_INFO_GET_ADVANCED_COLOR_INFO,
                    size: std::mem::size_of::<DISPLAYCONFIG_GET_ADVANCED_COLOR_INFO>() as u32,
                    adapterId: path.targetInfo.adapterId,
                    id: path.targetInfo.id,
                },
                ..Default::default()
            };
            if DisplayConfigGetDeviceInfo(&mut info.header) == 0 {
                let bits = info.Anonymous.value;
                if bits & 1 != 0 {
                    targets.push((path.targetInfo.adapterId, path.targetInfo.id, bits & 2 != 0));
                }
            }
        }
    }
    if targets.is_empty() {
        return Err("No HDR display found".into());
    }
    let enable = !targets.iter().any(|t| t.2);
    for (adapter, id, _) in targets {
        let mut state = DISPLAYCONFIG_SET_ADVANCED_COLOR_STATE {
            header: DISPLAYCONFIG_DEVICE_INFO_HEADER {
                r#type: DISPLAYCONFIG_DEVICE_INFO_SET_ADVANCED_COLOR_STATE,
                size: std::mem::size_of::<DISPLAYCONFIG_SET_ADVANCED_COLOR_STATE>() as u32,
                adapterId: adapter,
                id,
            },
            ..Default::default()
        };
        state.Anonymous.value = enable as u32;
        if unsafe { DisplayConfigSetDeviceInfo(&state.header) } != 0 {
            return Err("Unable to toggle HDR".into());
        }
    }
    Ok(if enable { "HDR On" } else { "HDR Off" }.into())
}

pub fn run(action: &str) -> Result<String, String> {
    match action {
        "toggle-night-light" => toggle_night_light(),
        "toggle-camera-mute" => toggle_camera(),
        "toggle-hdr" => toggle_hdr(),
        _ => Err(format!("Unknown toggle {action}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const OFF: [u8; 38] = [
        0x43, 0x42, 0x01, 0x00, 0x0a, 0x02, 0x01, 0x00, 0x2a, 0x06, 0xba, 0x96, 0xde, 0xd6, 0x06, 0x2a, 0x2b, 0x0e, 0x10, 0x43, 0x42, 0x01, 0x00, 0xc6, 0x14, 0xd7, 0x95, 0xc8, 0xb8, 0x93,
        0x95, 0xd8, 0xee, 0x01, 0x00, 0x00, 0x00, 0x00,
    ];

    #[test]
    fn night_light_blob_round_trips() {
        let (on, enabled) = toggle_night_light_blob(&OFF, 1_792_500_000).unwrap();
        assert!(enabled);
        assert_eq!(on.len(), OFF.len() + 2);
        assert_eq!(on[18], 0x12);
        assert_eq!(&on[19..25], &[0x43, 0x42, 0x01, 0x00, 0x02, 0x01]);
        let (off, enabled) = toggle_night_light_blob(&on, 1_792_500_001).unwrap();
        assert!(!enabled);
        assert_eq!(off[15..], OFF[15..]);
        assert_eq!(off[10..15], varint(1_792_500_001)[..]);
        assert!(toggle_night_light_blob(&[0u8; 20], 0).is_none());
    }

    #[test]
    fn varints_use_seven_bit_groups() {
        assert_eq!(varint(1), vec![1]);
        assert_eq!(varint(300), vec![0xac, 0x02]);
    }
}
