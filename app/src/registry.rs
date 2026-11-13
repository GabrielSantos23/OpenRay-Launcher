use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::ERROR_FILE_NOT_FOUND;
use windows::Win32::System::Registry::{
    RegDeleteKeyValueW, RegGetValueW, RegSetKeyValueW, HKEY_CURRENT_USER, REG_SZ, RRF_RT_REG_BINARY, RRF_RT_REG_SZ,
};

const RUN_KEY: PCWSTR = w!(r"Software\Microsoft\Windows\CurrentVersion\Run");
const VALUE: PCWSTR = w!("Openray");

fn command() -> Result<String, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    Ok(format!("\"{}\"", exe.display()))
}

pub fn login_item_enabled() -> bool {
    let mut buffer = [0u16; 1024];
    let mut size = (buffer.len() * 2) as u32;
    let read = unsafe { RegGetValueW(HKEY_CURRENT_USER, RUN_KEY, VALUE, RRF_RT_REG_SZ, None, Some(buffer.as_mut_ptr() as _), Some(&mut size)) };
    let len = (size as usize / 2).saturating_sub(1);
    read.is_ok() && command().is_ok_and(|expected| String::from_utf16_lossy(&buffer[..len]) == expected)
}

pub fn set_login_item(enabled: bool) -> Result<(), String> {
    let result = if enabled {
        let value: Vec<u16> = command()?.encode_utf16().chain(std::iter::once(0)).collect();
        unsafe { RegSetKeyValueW(HKEY_CURRENT_USER, RUN_KEY, VALUE, REG_SZ.0, Some(value.as_ptr() as _), (value.len() * 2) as u32) }
    } else {
        match unsafe { RegDeleteKeyValueW(HKEY_CURRENT_USER, RUN_KEY, VALUE) } {
            e if e == ERROR_FILE_NOT_FOUND => return Ok(()),
            e => e,
        }
    };
    result.ok().map_err(|e| e.message())
}

pub fn accent_color() -> Option<String> {
    let mut palette = [0u8; 32];
    let mut size = palette.len() as u32;
    let read = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            w!(r"Software\Microsoft\Windows\CurrentVersion\Explorer\Accent"),
            w!("AccentPalette"),
            RRF_RT_REG_BINARY,
            None,
            Some(palette.as_mut_ptr() as _),
            Some(&mut size),
        )
    };
    (read.is_ok() && size >= 16).then(|| format!("#{:02X}{:02X}{:02X}", palette[12], palette[13], palette[14]))
}
