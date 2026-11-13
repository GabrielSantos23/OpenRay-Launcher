use std::ffi::{c_void, CString};
use windows::core::{PCSTR, PWSTR};
use windows::Win32::Globalization::*;

fn locale_id(locale: &str) -> CString {
    CString::new(locale.replace('-', "_")).unwrap_or_default()
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().collect()
}

fn ok(status: UErrorCode) -> bool {
    status.0 <= 0
}

fn read(mut fill: impl FnMut(*mut u16, i32, &mut UErrorCode) -> i32) -> Option<String> {
    let mut buffer = vec![0u16; 256];
    let mut status = U_ZERO_ERROR;
    let len = fill(buffer.as_mut_ptr(), buffer.len() as i32, &mut status);
    if !ok(status) || len < 0 {
        return None;
    }
    Some(String::from_utf16_lossy(&buffer[..(len as usize).min(buffer.len())]).replace(['\u{202F}', '\u{00A0}'], " "))
}

pub fn languages() -> Vec<String> {
    let (mut count, mut len) = (0u32, 0u32);
    let mut list = Vec::new();
    unsafe {
        if GetUserPreferredUILanguages(MUI_LANGUAGE_NAME, &mut count, None, &mut len).is_ok() && len > 0 {
            let mut buffer = vec![0u16; len as usize];
            if GetUserPreferredUILanguages(MUI_LANGUAGE_NAME, &mut count, Some(PWSTR(buffer.as_mut_ptr())), &mut len).is_ok() {
                list = buffer.split(|c| *c == 0).filter(|s| !s.is_empty()).map(String::from_utf16_lossy).collect();
            }
        }
    }
    if list.is_empty() {
        list.push("en-US".into());
    }
    list
}

fn collect(enumeration: *mut UEnumeration, status: UErrorCode) -> Vec<String> {
    let mut out = Vec::new();
    if enumeration.is_null() || !ok(status) {
        return out;
    }
    unsafe {
        loop {
            let (mut len, mut status) = (0i32, U_ZERO_ERROR);
            let next = uenum_unext(enumeration, &mut len, &mut status);
            if next.is_null() || !ok(status) {
                break;
            }
            out.push(String::from_utf16_lossy(std::slice::from_raw_parts(next, len as usize)));
        }
        uenum_close(enumeration);
    }
    out
}

pub fn zones(region: Option<&str>) -> Vec<String> {
    let region = region.map(|r| CString::new(r).unwrap_or_default());
    let mut status = U_ZERO_ERROR;
    let enumeration = unsafe { ucal_openTimeZoneIDEnumeration(UCAL_ZONE_TYPE_CANONICAL_LOCATION, region.as_ref().map_or(PCSTR::null(), |r| PCSTR(r.as_ptr() as _)), std::ptr::null(), &mut status) };
    let mut list = collect(enumeration, status);
    list.sort();
    list
}

pub fn country_name(code: &str, display: &str) -> Option<String> {
    let locale = CString::new(format!("und_{code}")).ok()?;
    let display = locale_id(display);
    let name = read(|buf, cap, status| unsafe { uloc_getDisplayCountry(PCSTR(locale.as_ptr() as _), PCSTR(display.as_ptr() as _), buf, cap, status) })?;
    (!name.is_empty() && name != code).then_some(name)
}

pub fn offset_minutes(ms: f64, zone: &str) -> i32 {
    let zone = wide(zone);
    let mut status = U_ZERO_ERROR;
    unsafe {
        let cal = ucal_open(zone.as_ptr(), zone.len() as i32, PCSTR::null(), UCAL_GREGORIAN, &mut status);
        if cal.is_null() || !ok(status) {
            return 0;
        }
        ucal_setMillis(cal, ms, &mut status);
        let total = ucal_get(cal as _, UCAL_ZONE_OFFSET, &mut status) + ucal_get(cal as _, UCAL_DST_OFFSET, &mut status);
        ucal_close(cal);
        if ok(status) {
            total / 60_000
        } else {
            0
        }
    }
}

pub fn local_offset_minutes(ms: f64) -> i32 {
    let zone = read(|buf, cap, status| unsafe { ucal_getDefaultTimeZone(buf, cap, status) }).unwrap_or_else(|| "UTC".into());
    offset_minutes(ms, &zone)
}

fn format_with(ms: f64, locale: &str, time: UDateFormatStyle, date: UDateFormatStyle, pattern: Option<&str>, zone: Option<&str>) -> String {
    let locale = locale_id(locale);
    let zone = zone.map(wide);
    let pattern = pattern.map(wide);
    let mut status = U_ZERO_ERROR;
    unsafe {
        let format = udat_open(
            time,
            date,
            PCSTR(locale.as_ptr() as _),
            zone.as_ref().map_or(std::ptr::null(), |z| z.as_ptr()),
            zone.as_ref().map_or(-1, |z| z.len() as i32),
            pattern.as_ref().map_or(std::ptr::null(), |p| p.as_ptr()),
            pattern.as_ref().map_or(-1, |p| p.len() as i32),
            &mut status,
        );
        if format.is_null() || !ok(status) {
            return String::new();
        }
        let text = read(|buf, cap, status| udat_format(format as _, ms, buf, cap, std::ptr::null_mut(), status)).unwrap_or_default();
        udat_close(format);
        text
    }
}

#[derive(Clone, Copy)]
pub enum Style {
    None,
    Full,
    #[cfg_attr(not(test), allow(dead_code))]
    Long,
    Medium,
    Short,
}

fn style(style: Style) -> UDateFormatStyle {
    match style {
        Style::None => UDAT_NONE,
        Style::Full => UDAT_FULL,
        Style::Long => UDAT_LONG,
        Style::Medium => UDAT_MEDIUM,
        Style::Short => UDAT_SHORT,
    }
}

pub fn format_date(ms: f64, locale: &str, date: Style, time: Style, zone: Option<&str>) -> String {
    format_with(ms, locale, style(time), style(date), None, zone)
}

pub fn format_pattern(ms: f64, locale: &str, pattern: &str, zone: Option<&str>) -> String {
    format_with(ms, locale, UDAT_PATTERN, UDAT_PATTERN, Some(pattern), zone)
}

pub fn format_skeleton(ms: f64, locale: &str, skeleton: &str, zone: Option<&str>) -> String {
    thread_local! {
        static PATTERNS: std::cell::RefCell<std::collections::HashMap<(String, String), Option<String>>> = Default::default();
    }
    let key = (locale.to_owned(), skeleton.to_owned());
    if let Some(pattern) = PATTERNS.with(|p| p.borrow().get(&key).cloned()) {
        return pattern.map_or_else(String::new, |p| format_pattern(ms, locale, &p, zone));
    }
    let id = locale_id(locale);
    let skeleton = wide(skeleton);
    let mut status = U_ZERO_ERROR;
    let pattern = unsafe {
        let generator = udatpg_open(PCSTR(id.as_ptr() as _), &mut status);
        if generator.is_null() || !ok(status) {
            return String::new();
        }
        let pattern = read(|buf, cap, status| udatpg_getBestPattern(generator, skeleton.as_ptr(), skeleton.len() as i32, buf, cap, status));
        udatpg_close(generator);
        pattern
    };
    PATTERNS.with(|p| p.borrow_mut().insert(key, pattern.clone()));
    pattern.map_or_else(String::new, |p| format_pattern(ms, locale, &p, zone))
}

pub fn display_locale() -> String {
    languages().into_iter().next().unwrap_or_else(|| "en-US".into())
}

pub fn currency_name(code: &str, locale: &str) -> String {
    let mut currency = wide(code);
    currency.push(0);
    let locale = locale_id(locale);
    let (mut choice, mut len, mut status) = (0i8, 0i32, U_ZERO_ERROR);
    let name = unsafe { ucurr_getName(currency.as_ptr(), PCSTR(locale.as_ptr() as _), UCURR_LONG_NAME, &mut choice, &mut len, &mut status) };
    if name.is_null() || !ok(status) {
        return code.to_owned();
    }
    String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(name, len as usize) })
}

pub fn format_money(amount: f64, code: &str, locale: &str, max_fraction: i32) -> Option<String> {
    let locale = locale_id(locale);
    let mut currency = wide(code);
    currency.push(0);
    let mut status = U_ZERO_ERROR;
    unsafe {
        let format = unum_open(UNUM_CURRENCY, std::ptr::null(), 0, PCSTR(locale.as_ptr() as _), std::ptr::null_mut(), &mut status);
        if format.is_null() || !ok(status) {
            return None;
        }
        unum_setAttribute(format, UNUM_MAX_FRACTION_DIGITS, max_fraction);
        let text = read(|buf, cap, status| unum_formatDoubleCurrency(format as *const *const c_void, amount, currency.as_mut_ptr(), buf, cap, std::ptr::null_mut(), status));
        unum_close(format);
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_like_intl() {
        let ms = 1_792_362_720_000.0;
        assert_eq!(format_date(ms, "en-US", Style::Long, Style::Short, Some("Asia/Shanghai")), "October 19, 2026 at 6:32 AM");
        assert_eq!(format_pattern(ms, "en-US", "O", Some("Asia/Shanghai")), "GMT+8");
        assert_eq!(offset_minutes(ms, "Asia/Kolkata"), 330);
        assert_eq!(zones(Some("CN")).first().map(String::as_str), Some("Asia/Shanghai"));
        assert_eq!(country_name("BR", "pt").as_deref(), Some("Brasil"));
        assert_eq!(country_name("BR", "en").as_deref(), Some("Brazil"));
        assert_eq!(currency_name("BRL", "en-US"), "Brazilian Real");
        assert_eq!(format_money(1234.5, "USD", "en-US", 2).as_deref(), Some("$1,234.50"));
        assert!(zones(None).len() > 300);
    }
}
