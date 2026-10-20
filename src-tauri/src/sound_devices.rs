use serde::Serialize;
use std::ffi::c_void;
use windows::core::{interface, IUnknown, IUnknown_Vtbl, GUID, HRESULT, PCWSTR, PWSTR};
use windows::Win32::Foundation::PROPERTYKEY;
use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
use windows::Win32::Media::Audio::{
    eCapture, eCommunications, eConsole, eMultimedia, eRender, EDataFlow, ERole, IMMDevice, IMMDeviceEnumerator, MMDeviceEnumerator, DEVICE_STATE,
    DEVICE_STATE_ACTIVE, DEVICE_STATE_DISABLED, DEVICE_STATE_UNPLUGGED,
};
use windows::Win32::System::Com::StructuredStorage::{PropVariantToStringAlloc, PropVariantToUInt32};
use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CoTaskMemFree, CLSCTX_ALL, COINIT_APARTMENTTHREADED, STGM_READ};

#[interface("f8679f50-850a-41cf-9c72-430f290290c8")]
unsafe trait IPolicyConfig: IUnknown {
    fn get_mix_format(&self, id: PCWSTR, format: *mut *mut c_void) -> HRESULT;
    fn get_device_format(&self, id: PCWSTR, default: i32, format: *mut *mut c_void) -> HRESULT;
    fn reset_device_format(&self, id: PCWSTR) -> HRESULT;
    fn set_device_format(&self, id: PCWSTR, endpoint: *mut c_void, mix: *mut c_void) -> HRESULT;
    fn get_processing_period(&self, id: PCWSTR, default: i32, default_period: *mut i64, min_period: *mut i64) -> HRESULT;
    fn set_processing_period(&self, id: PCWSTR, period: *mut i64) -> HRESULT;
    fn get_share_mode(&self, id: PCWSTR, mode: *mut c_void) -> HRESULT;
    fn set_share_mode(&self, id: PCWSTR, mode: *mut c_void) -> HRESULT;
    fn get_property_value(&self, id: PCWSTR, fx_store: i32, key: *const PROPERTYKEY, value: *mut c_void) -> HRESULT;
    fn set_property_value(&self, id: PCWSTR, fx_store: i32, key: *const PROPERTYKEY, value: *mut c_void) -> HRESULT;
    fn set_default_endpoint(&self, id: PCWSTR, role: ERole) -> HRESULT;
    fn set_endpoint_visibility(&self, id: PCWSTR, visible: i32) -> HRESULT;
}

const POLICY_CONFIG: GUID = GUID::from_u128(0x870af99c_171d_4f9e_af0d_e63df40c2bc9);
const FRIENDLY_NAME: PROPERTYKEY = PROPERTYKEY { fmtid: GUID::from_u128(0xa45c254e_df1c_4efd_8020_67d146a850e0), pid: 14 };
const FORM_FACTOR: PROPERTYKEY = PROPERTYKEY { fmtid: GUID::from_u128(0x1da5d803_d492_4edd_8c23_e0c0ffee7f0e), pid: 0 };

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SoundDevice {
    id: String,
    name: String,
    state: &'static str,
    form_factor: u32,
    is_default: bool,
    volume: f32,
    is_muted: bool,
}

fn message(e: windows::core::Error) -> String {
    e.message()
}

fn flow(direction: &str) -> Result<EDataFlow, String> {
    match direction {
        "output" => Ok(eRender),
        "input" => Ok(eCapture),
        _ => Err(format!("Unknown direction {direction}")),
    }
}

fn enumerator() -> Result<IMMDeviceEnumerator, String> {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).map_err(message)
    }
}

fn device_id(device: &IMMDevice) -> Option<String> {
    unsafe {
        let raw: PWSTR = device.GetId().ok()?;
        let id = raw.to_string().ok();
        CoTaskMemFree(Some(raw.0 as _));
        id
    }
}

fn state_name(state: DEVICE_STATE) -> &'static str {
    if state == DEVICE_STATE_ACTIVE {
        "active"
    } else if state == DEVICE_STATE_DISABLED {
        "disabled"
    } else {
        "disconnected"
    }
}

fn describe(device: &IMMDevice, default_id: Option<&str>) -> Option<SoundDevice> {
    let id = device_id(device)?;
    unsafe {
        let store = device.OpenPropertyStore(STGM_READ).ok()?;
        let name = store.GetValue(&FRIENDLY_NAME).ok().and_then(|v| PropVariantToStringAlloc(&v).ok()).map(|raw| {
            let text = raw.to_string().unwrap_or_default();
            CoTaskMemFree(Some(raw.0 as _));
            text
        })?;
        let form_factor = store.GetValue(&FORM_FACTOR).ok().and_then(|v| PropVariantToUInt32(&v).ok()).unwrap_or(10);
        let state = device.GetState().ok()?;
        let volume = (state == DEVICE_STATE_ACTIVE).then(|| device.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None).ok()).flatten();
        Some(SoundDevice {
            is_default: default_id == Some(id.as_str()),
            id,
            name,
            state: state_name(state),
            form_factor,
            volume: volume.as_ref().and_then(|v| v.GetMasterVolumeLevelScalar().ok()).map_or(0.0, |v| (v * 100.0).round()),
            is_muted: volume.as_ref().and_then(|v| v.GetMute().ok()).is_some_and(|m| m.as_bool()),
        })
    }
}

pub fn list(direction: &str) -> Result<Vec<SoundDevice>, String> {
    let flow = flow(direction)?;
    let enumerator = enumerator()?;
    unsafe {
        let default_id = enumerator.GetDefaultAudioEndpoint(flow, eConsole).ok().as_ref().and_then(device_id);
        let mask = DEVICE_STATE(DEVICE_STATE_ACTIVE.0 | DEVICE_STATE_DISABLED.0 | DEVICE_STATE_UNPLUGGED.0);
        let devices = enumerator.EnumAudioEndpoints(flow, mask).map_err(message)?;
        let count = devices.GetCount().map_err(message)?;
        Ok((0..count).filter_map(|i| devices.Item(i).ok()).filter_map(|d| describe(&d, default_id.as_deref())).collect())
    }
}

fn volume_of(id: &str) -> Result<IAudioEndpointVolume, String> {
    let wide: Vec<u16> = id.encode_utf16().chain([0]).collect();
    unsafe {
        let device = enumerator()?.GetDevice(PCWSTR(wide.as_ptr())).map_err(message)?;
        device.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None).map_err(message)
    }
}

pub fn set_default(id: &str) -> Result<(), String> {
    let wide: Vec<u16> = id.encode_utf16().chain([0]).collect();
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let policy: IPolicyConfig = CoCreateInstance(&POLICY_CONFIG, None, CLSCTX_ALL).map_err(message)?;
        for role in [eConsole, eMultimedia, eCommunications] {
            policy.set_default_endpoint(PCWSTR(wide.as_ptr()), role).ok().map_err(message)?;
        }
    }
    Ok(())
}

pub fn set_volume(id: &str, percent: f32) -> Result<(), String> {
    let volume = volume_of(id)?;
    unsafe {
        if percent > 0.0 {
            volume.SetMute(false, std::ptr::null()).map_err(message)?;
        }
        volume.SetMasterVolumeLevelScalar((percent / 100.0).clamp(0.0, 1.0), std::ptr::null()).map_err(message)
    }
}

pub fn set_muted(id: &str, muted: bool) -> Result<(), String> {
    unsafe { volume_of(id)?.SetMute(muted, std::ptr::null()).map_err(message) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn states_map_to_sections() {
        assert_eq!(state_name(DEVICE_STATE_ACTIVE), "active");
        assert_eq!(state_name(DEVICE_STATE_DISABLED), "disabled");
        assert_eq!(state_name(DEVICE_STATE_UNPLUGGED), "disconnected");
        assert!(flow("sideways").is_err());
    }

    #[test]
    #[ignore = "reads and changes the real default output device"]
    fn lists_devices_and_restores_the_default() {
        let devices = list("output").unwrap();
        for d in &devices {
            println!("{} | {} | default={} vol={} muted={} ff={}", d.name, d.state, d.is_default, d.volume, d.is_muted, d.form_factor);
        }
        let current = devices.iter().find(|d| d.is_default).expect("a default output");
        set_default(&current.id).unwrap();
        assert!(list("output").unwrap().iter().any(|d| d.is_default && d.id == current.id));
    }
}
