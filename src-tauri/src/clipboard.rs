use crate::store::{read_json, write_json};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::Mutex;

pub const RETENTION_MS: u64 = 7 * 86_400_000;
const SAME_COPY_MS: u64 = 1_000;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Content {
    Text { text: String },
    Files { paths: Vec<String> },
    #[serde(rename_all = "camelCase")]
    Image { width: u32, height: u32, bytes: u64, hash: u64 },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub id: String,
    #[serde(flatten)]
    pub content: Content,
    pub source_name: String,
    pub source_path: String,
    pub copy_count: u32,
    pub created_at: u64,
    pub updated_at: u64,
    pub pinned: Option<u64>,
}

pub enum Captured {
    Text(String),
    Files(Vec<String>),
    Image { png: Vec<u8>, width: u32, height: u32 },
}

pub struct History {
    dir: PathBuf,
    entries: Mutex<Vec<Entry>>,
}

fn hash_bytes(bytes: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    hasher.finish()
}

impl History {
    pub fn load(dir: PathBuf, now_ms: u64) -> Self {
        let _ = fs::create_dir_all(&dir);
        let entries: Vec<Entry> = read_json(&dir.join("history.json"));
        let history = Self { dir, entries: Mutex::new(entries) };
        history.prune(now_ms);
        history
    }

    pub fn image_path(&self, id: &str) -> Option<PathBuf> {
        id.chars().all(|c| c.is_ascii_alphanumeric()).then(|| self.dir.join(format!("{id}.png")))
    }

    fn save(&self, entries: &[Entry]) -> Result<(), String> {
        write_json(&self.dir.join("history.json"), &entries).map_err(|e| e.to_string())
    }

    fn remove_files(&self, removed: &[Entry]) {
        for entry in removed.iter().filter(|e| matches!(e.content, Content::Image { .. })) {
            if let Some(path) = self.image_path(&entry.id) {
                let _ = fs::remove_file(path);
            }
        }
    }

    fn retain(&self, keep: impl Fn(&Entry) -> bool) -> Result<(), String> {
        let mut entries = self.entries.lock().unwrap();
        let (kept, removed): (Vec<Entry>, Vec<Entry>) = entries.drain(..).partition(|e| keep(e));
        *entries = kept;
        self.remove_files(&removed);
        self.save(&entries)
    }

    pub fn prune(&self, now_ms: u64) {
        let _ = self.retain(|e| e.pinned.is_some() || now_ms.saturating_sub(e.updated_at) < RETENTION_MS);
    }

    pub fn list(&self) -> Vec<Entry> {
        let mut entries = self.entries.lock().unwrap().clone();
        entries.sort_by(|a, b| match (a.pinned, b.pinned) {
            (Some(x), Some(y)) => x.cmp(&y),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => b.updated_at.cmp(&a.updated_at),
        });
        entries
    }

    pub fn get(&self, id: &str) -> Option<Entry> {
        self.entries.lock().unwrap().iter().find(|e| e.id == id).cloned()
    }

    pub fn record(&self, captured: Captured, source_name: String, source_path: String, now_ms: u64) -> Result<(), String> {
        let (content, png) = match captured {
            Captured::Text(text) if text.trim().is_empty() => return Ok(()),
            Captured::Text(text) => (Content::Text { text }, None),
            Captured::Files(paths) if paths.is_empty() => return Ok(()),
            Captured::Files(paths) => (Content::Files { paths }, None),
            Captured::Image { png, width, height } => {
                (Content::Image { width, height, bytes: png.len() as u64, hash: hash_bytes(&png) }, Some(png))
            }
        };
        let mut entries = self.entries.lock().unwrap();
        if let Some(existing) = entries.iter_mut().find(|e| e.content == content) {
            if now_ms.saturating_sub(existing.updated_at) >= SAME_COPY_MS {
                existing.copy_count += 1;
            }
            existing.updated_at = now_ms;
            existing.source_name = source_name;
            existing.source_path = source_path;
            return self.save(&entries);
        }
        let id = format!("{now_ms:x}{:x}", entries.len());
        if let Some(png) = png {
            fs::write(self.dir.join(format!("{id}.png")), png).map_err(|e| e.to_string())?;
        }
        entries.push(Entry { id, content, source_name, source_path, copy_count: 1, created_at: now_ms, updated_at: now_ms, pinned: None });
        self.save(&entries)
    }

    pub fn delete(&self, id: &str) -> Result<(), String> {
        self.retain(|e| e.id != id)
    }

    pub fn delete_all(&self) -> Result<(), String> {
        self.retain(|e| e.pinned.is_some())
    }

    pub fn delete_since(&self, since_ms: u64) -> Result<(), String> {
        self.retain(|e| e.pinned.is_some() || e.updated_at < since_ms)
    }

    pub fn set_pinned(&self, id: &str, pinned: bool, now_ms: u64) -> Result<(), String> {
        let mut entries = self.entries.lock().unwrap();
        let entry = entries.iter_mut().find(|e| e.id == id).ok_or("entry no longer exists")?;
        entry.pinned = pinned.then_some(now_ms);
        self.save(&entries)
    }

    pub fn image_png(&self, entry: &Entry) -> Result<Vec<u8>, String> {
        let path = self.image_path(&entry.id).ok_or("invalid entry id")?;
        fs::read(path).map_err(|e| e.to_string())
    }
}

#[cfg(windows)]
mod win {
    use super::Captured;
    use std::sync::OnceLock;
    use windows::core::{w, HSTRING, PWSTR};
    use windows::Win32::Foundation::{CloseHandle, HANDLE, HGLOBAL, HWND, LPARAM, LRESULT, WPARAM};
    use windows::Win32::Graphics::Gdi::BITMAPINFOHEADER;
    use windows::Win32::Storage::FileSystem::{GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW};
    use windows::Win32::System::DataExchange::{
        AddClipboardFormatListener, CloseClipboard, EmptyClipboard, GetClipboardData, GetClipboardOwner, IsClipboardFormatAvailable,
        OpenClipboard, RegisterClipboardFormatW, SetClipboardData,
    };
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock, GMEM_MOVEABLE};
    use windows::Win32::System::Threading::{OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION};
    use windows::Win32::UI::Input::KeyboardAndMouse::{SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VIRTUAL_KEY, VK_C, VK_CONTROL, VK_V};
    use windows::Win32::UI::Shell::{DragQueryFileW, DROPFILES, HDROP};
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, GetWindowThreadProcessId, RegisterClassW, HWND_MESSAGE, MSG,
        WINDOW_EX_STYLE, WINDOW_STYLE, WM_CLIPBOARDUPDATE, WNDCLASSW,
    };

    const CF_UNICODETEXT: u32 = 13;
    const CF_HDROP: u32 = 15;
    const CF_DIB: u32 = 8;

    const SETTLE: std::time::Duration = std::time::Duration::from_millis(100);

    static ON_CHANGE: OnceLock<Box<dyn Fn() + Send + Sync>> = OnceLock::new();

    unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if msg == WM_CLIPBOARDUPDATE {
            std::thread::sleep(SETTLE);
            if let Some(callback) = ON_CHANGE.get() {
                callback();
            }
            return LRESULT(0);
        }
        unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
    }

    pub fn listen(on_change: Box<dyn Fn() + Send + Sync>) -> Result<(), String> {
        ON_CHANGE.set(on_change).map_err(|_| "clipboard listener already running".to_string())?;
        std::thread::Builder::new()
            .name("openray-clipboard".into())
            .spawn(|| unsafe {
                let Ok(instance) = GetModuleHandleW(None) else { return };
                let class = WNDCLASSW { lpfnWndProc: Some(wndproc), hInstance: instance.into(), lpszClassName: w!("OpenrayClipboard"), ..Default::default() };
                RegisterClassW(&class);
                let Ok(hwnd) = CreateWindowExW(
                    WINDOW_EX_STYLE(0),
                    w!("OpenrayClipboard"),
                    w!(""),
                    WINDOW_STYLE(0),
                    0,
                    0,
                    0,
                    0,
                    Some(HWND_MESSAGE),
                    None,
                    Some(instance.into()),
                    None,
                ) else {
                    return;
                };
                if AddClipboardFormatListener(hwnd).is_err() {
                    return;
                }
                let mut msg = MSG::default();
                while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                    DispatchMessageW(&msg);
                }
            })
            .map(drop)
            .map_err(|e| e.to_string())
    }

    struct Open;

    impl Open {
        fn new() -> Result<Self, String> {
            for _ in 0..10 {
                if unsafe { OpenClipboard(None) }.is_ok() {
                    return Ok(Open);
                }
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            Err("the clipboard is in use by another app".into())
        }
    }

    impl Drop for Open {
        fn drop(&mut self) {
            let _ = unsafe { CloseClipboard() };
        }
    }

    fn format(name: &str) -> u32 {
        unsafe { RegisterClipboardFormatW(&HSTRING::from(name)) }
    }

    fn available(format: u32) -> bool {
        format != 0 && unsafe { IsClipboardFormatAvailable(format) }.is_ok()
    }

    fn global_bytes(format: u32) -> Option<Vec<u8>> {
        let handle = unsafe { GetClipboardData(format) }.ok()?;
        let global = HGLOBAL(handle.0);
        unsafe {
            let ptr = GlobalLock(global) as *const u8;
            if ptr.is_null() {
                return None;
            }
            let bytes = std::slice::from_raw_parts(ptr, GlobalSize(global)).to_vec();
            let _ = GlobalUnlock(global);
            Some(bytes)
        }
    }

    fn excluded() -> bool {
        if available(format("ExcludeClipboardContentFromMonitorProcessing")) || available(format("Clipboard Viewer Ignore")) {
            return true;
        }
        let allowed = format("CanIncludeInClipboardHistory");
        available(allowed) && global_bytes(allowed).is_some_and(|b| b.len() >= 4 && u32::from_le_bytes([b[0], b[1], b[2], b[3]]) == 0)
    }

    fn read_files() -> Option<Vec<String>> {
        let handle = unsafe { GetClipboardData(CF_HDROP) }.ok()?;
        let drop = HDROP(handle.0);
        let count = unsafe { DragQueryFileW(drop, u32::MAX, None) };
        Some(
            (0..count)
                .filter_map(|i| {
                    let len = unsafe { DragQueryFileW(drop, i, None) } as usize;
                    let mut buffer = vec![0u16; len + 1];
                    let written = unsafe { DragQueryFileW(drop, i, Some(&mut buffer)) } as usize;
                    (written > 0).then(|| String::from_utf16_lossy(&buffer[..written]))
                })
                .collect(),
        )
    }

    fn read_text() -> Option<String> {
        let bytes = global_bytes(CF_UNICODETEXT)?;
        let units: Vec<u16> = bytes.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).take_while(|&u| u != 0).collect();
        Some(String::from_utf16_lossy(&units))
    }

    pub fn dib_to_png(dib: &[u8]) -> Option<(Vec<u8>, u32, u32)> {
        let read_u32 = |at: usize| dib.get(at..at + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]));
        let header_size = read_u32(0)? as usize;
        let width = read_u32(4)? as i32;
        let height = read_u32(8)? as i32;
        let bits = u16::from_le_bytes([*dib.get(14)?, *dib.get(15)?]);
        let compression = read_u32(16)?;
        let colors_used = read_u32(32)? as usize;
        if width <= 0 || height == 0 || !matches!(bits, 24 | 32) || !matches!(compression, 0 | 3) {
            return None;
        }
        let masks = if compression == 3 && header_size == 40 { 12 } else { 0 };
        let offset = header_size + masks + colors_used * 4;
        let (w, h) = (width as usize, height.unsigned_abs() as usize);
        let stride = (w * bits as usize / 8).div_ceil(4) * 4;
        let pixels = dib.get(offset..offset + stride * h)?;
        let has_alpha = bits == 32 && pixels.chunks_exact(4).any(|p| p[3] != 0);
        let mut rgba = Vec::with_capacity(w * h * 4);
        for y in 0..h {
            let row = if height > 0 { h - 1 - y } else { y };
            let line = &pixels[row * stride..];
            for x in 0..w {
                let p = &line[x * bits as usize / 8..];
                rgba.extend_from_slice(&[p[2], p[1], p[0], if has_alpha { p[3] } else { 255 }]);
            }
        }
        let mut png = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut png, w as u32, h as u32);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder.write_header().ok()?.write_image_data(&rgba).ok()?;
        }
        Some((png, w as u32, h as u32))
    }

    fn png_to_dib(png_bytes: &[u8]) -> Result<Vec<u8>, String> {
        let mut decoder = png::Decoder::new(png_bytes);
        decoder.set_transformations(png::Transformations::normalize_to_color8() | png::Transformations::ALPHA);
        let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
        let mut buffer = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buffer).map_err(|e| e.to_string())?;
        let channels = info.color_type.samples();
        let (w, h) = (info.width as usize, info.height as usize);
        let header = BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: w as i32,
            biHeight: h as i32,
            biPlanes: 1,
            biBitCount: 32,
            biSizeImage: (w * h * 4) as u32,
            ..Default::default()
        };
        let mut dib = unsafe { std::slice::from_raw_parts(&header as *const _ as *const u8, std::mem::size_of::<BITMAPINFOHEADER>()) }.to_vec();
        for y in (0..h).rev() {
            for x in 0..w {
                let p = &buffer[(y * w + x) * channels..];
                let (r, g, b, a) = if channels >= 3 { (p[0], p[1], p[2], if channels == 4 { p[3] } else { 255 }) } else { (p[0], p[0], p[0], if channels == 2 { p[1] } else { 255 }) };
                dib.extend_from_slice(&[b, g, r, a]);
            }
        }
        Ok(dib)
    }

    pub fn read_clipboard_text() -> Option<String> {
        let _open = Open::new().ok()?;
        available(CF_UNICODETEXT).then(read_text).flatten()
    }

    pub fn capture() -> Option<Captured> {
        let _open = Open::new().ok()?;
        if excluded() {
            return None;
        }
        if available(CF_HDROP) {
            if let Some(files) = read_files() {
                return Some(Captured::Files(files));
            }
        }
        if available(CF_UNICODETEXT) {
            return read_text().map(Captured::Text);
        }
        if available(CF_DIB) {
            let (png, width, height) = global_bytes(CF_DIB).and_then(|dib| dib_to_png(&dib))?;
            return Some(Captured::Image { png, width, height });
        }
        None
    }

    fn set(format: u32, bytes: &[u8]) -> Result<(), String> {
        unsafe {
            let global = GlobalAlloc(GMEM_MOVEABLE, bytes.len()).map_err(|e| e.to_string())?;
            let ptr = GlobalLock(global) as *mut u8;
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len());
            let _ = GlobalUnlock(global);
            SetClipboardData(format, Some(HANDLE(global.0))).map(drop).map_err(|e| e.to_string())
        }
    }

    fn utf16z(text: &str) -> Vec<u8> {
        text.encode_utf16().chain([0]).flat_map(u16::to_le_bytes).collect()
    }

    pub fn write_text(text: &str) -> Result<(), String> {
        let _open = Open::new()?;
        unsafe { EmptyClipboard() }.map_err(|e| e.to_string())?;
        set(CF_UNICODETEXT, &utf16z(text))
    }

    pub fn write_files(paths: &[String]) -> Result<(), String> {
        let header = DROPFILES { pFiles: std::mem::size_of::<DROPFILES>() as u32, fWide: true.into(), ..Default::default() };
        let mut bytes = unsafe { std::slice::from_raw_parts(&header as *const _ as *const u8, std::mem::size_of::<DROPFILES>()) }.to_vec();
        for path in paths {
            bytes.extend(utf16z(path));
        }
        bytes.extend([0, 0]);
        let _open = Open::new()?;
        unsafe { EmptyClipboard() }.map_err(|e| e.to_string())?;
        set(CF_HDROP, &bytes)
    }

    fn html_fragment(html: &str) -> Vec<u8> {
        let prefix = "<html><body><!--StartFragment-->";
        let suffix = "<!--EndFragment--></body></html>";
        let header_len = "Version:0.9
StartHTML:0000000000
EndHTML:0000000000
StartFragment:0000000000
EndFragment:0000000000
".len();
        let start_fragment = header_len + prefix.len();
        let end_fragment = start_fragment + html.len();
        let end_html = end_fragment + suffix.len();
        let mut bytes = format!("Version:0.9
StartHTML:{header_len:010}
EndHTML:{end_html:010}
StartFragment:{start_fragment:010}
EndFragment:{end_fragment:010}
{prefix}{html}{suffix}").into_bytes();
        bytes.push(0);
        bytes
    }

    pub fn write_rich(text: Option<&str>, html: Option<&str>, paths: Option<&[String]>) -> Result<(), String> {
        if let Some(paths) = paths.filter(|p| !p.is_empty()) {
            write_files(paths)?;
        }
        let _open = Open::new()?;
        if paths.is_none_or(|p| p.is_empty()) {
            unsafe { EmptyClipboard() }.map_err(|e| e.to_string())?;
        }
        if let Some(text) = text {
            set(CF_UNICODETEXT, &utf16z(text))?;
        }
        if let Some(html) = html {
            set(format("HTML Format"), &html_fragment(html))?;
        }
        Ok(())
    }

    fn read_html() -> Option<String> {
        let bytes = global_bytes(format("HTML Format"))?;
        let raw = String::from_utf8_lossy(&bytes).into_owned();
        let offset = |name: &str| raw.lines().find_map(|line| line.strip_prefix(name)).and_then(|v| v.trim().parse::<usize>().ok());
        let (start, end) = (offset("StartFragment:")?, offset("EndFragment:")?);
        bytes.get(start..end).map(|slice| String::from_utf8_lossy(slice).into_owned())
    }

    pub fn read_rich() -> (Option<String>, Option<String>, Option<Vec<String>>) {
        let Ok(_open) = Open::new() else { return (None, None, None) };
        let text = available(CF_UNICODETEXT).then(read_text).flatten();
        let html = available(format("HTML Format")).then(read_html).flatten();
        let paths = available(CF_HDROP).then(read_files).flatten();
        (text, html, paths)
    }

    pub fn write_image(png_bytes: &[u8]) -> Result<(), String> {
        let dib = png_to_dib(png_bytes)?;
        let _open = Open::new()?;
        unsafe { EmptyClipboard() }.map_err(|e| e.to_string())?;
        set(CF_DIB, &dib)?;
        set(format("PNG"), png_bytes)
    }

    fn file_description(path: &str) -> Option<String> {
        let wide = HSTRING::from(path);
        unsafe {
            let size = GetFileVersionInfoSizeW(&wide, None);
            if size == 0 {
                return None;
            }
            let mut data = vec![0u8; size as usize];
            GetFileVersionInfoW(&wide, None, size, data.as_mut_ptr() as _).ok()?;
            let mut ptr = std::ptr::null_mut();
            let mut len = 0;
            if !VerQueryValueW(data.as_ptr() as _, w!("\\VarFileInfo\\Translation"), &mut ptr, &mut len).as_bool() || len < 4 {
                return None;
            }
            let translation = *(ptr as *const [u16; 2]);
            let key = format!("\\StringFileInfo\\{:04x}{:04x}\\FileDescription", translation[0], translation[1]);
            if !VerQueryValueW(data.as_ptr() as _, &HSTRING::from(key), &mut ptr, &mut len).as_bool() || len == 0 {
                return None;
            }
            let text = String::from_utf16_lossy(std::slice::from_raw_parts(ptr as *const u16, len as usize - 1)).trim().to_string();
            (!text.is_empty()).then_some(text)
        }
    }

    pub fn window_app(hwnd: HWND) -> Option<(String, String)> {
        let mut pid = 0;
        unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
        if pid == 0 {
            return None;
        }
        let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }.ok()?;
        let mut buffer = [0u16; 1024];
        let mut len = buffer.len() as u32;
        let result = unsafe { QueryFullProcessImageNameW(process, PROCESS_NAME_WIN32, PWSTR(buffer.as_mut_ptr()), &mut len) };
        let _ = unsafe { CloseHandle(process) };
        result.ok()?;
        let path = String::from_utf16_lossy(&buffer[..len as usize]);
        let stem = std::path::Path::new(&path).file_stem()?.to_string_lossy().into_owned();
        Some((file_description(&path).unwrap_or(stem), path))
    }

    pub fn owner_app() -> Option<(String, String)> {
        let owner = unsafe { GetClipboardOwner() }.ok()?;
        window_app(owner)
    }

    pub fn send_paste() {
        send_ctrl(VK_V)
    }

    pub fn send_copy() {
        send_ctrl(VK_C)
    }

    fn send_ctrl(letter: VIRTUAL_KEY) {
        let key = |vk, up: bool| INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT { wVk: vk, dwFlags: if up { KEYEVENTF_KEYUP } else { Default::default() }, dwExtraInfo: crate::hotkey::MARKER, ..Default::default() },
            },
        };
        let inputs = [key(VK_CONTROL, false), key(letter, false), key(letter, true), key(VK_CONTROL, true)];
        unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
    }
}

#[cfg(windows)]
pub use win::{capture, listen, owner_app, read_clipboard_text, read_rich, send_copy, send_paste, window_app, write_files, write_image, write_rich, write_text};

pub fn write_entry(history: &History, entry: &Entry) -> Result<(), String> {
    match &entry.content {
        Content::Text { text } => write_text(text),
        Content::Files { paths } => write_files(paths),
        Content::Image { .. } => write_image(&history.image_png(entry)?),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DAY: u64 = 86_400_000;

    fn text(t: &str) -> Captured {
        Captured::Text(t.into())
    }

    #[test]
    fn repeated_copies_bump_the_entry_instead_of_duplicating() {
        let dir = tempfile::tempdir().unwrap();
        let history = History::load(dir.path().to_path_buf(), 0);
        history.record(text("a"), "Notepad".into(), String::new(), 1).unwrap();
        history.record(text("b"), "Notepad".into(), String::new(), 2).unwrap();
        history.record(text("a"), "Code".into(), String::new(), 3_000).unwrap();
        history.record(text("a"), "Code".into(), String::new(), 3_400).unwrap();
        history.record(text("   "), "Code".into(), String::new(), 5_000).unwrap();
        let list = history.list();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].content, Content::Text { text: "a".into() });
        assert_eq!((list[0].copy_count, list[0].created_at, list[0].updated_at), (2, 1, 3_400));
        assert_eq!(list[0].source_name, "Code");
    }

    #[test]
    fn pinned_entries_lead_and_survive_retention_and_delete_all() {
        let dir = tempfile::tempdir().unwrap();
        let history = History::load(dir.path().to_path_buf(), 0);
        history.record(text("old"), String::new(), String::new(), 0).unwrap();
        history.record(text("kept"), String::new(), String::new(), 1).unwrap();
        history.record(text("new"), String::new(), String::new(), 8 * DAY).unwrap();
        let kept = history.list().into_iter().find(|e| e.content == Content::Text { text: "kept".into() }).unwrap();
        history.set_pinned(&kept.id, true, 5).unwrap();
        let reloaded = History::load(dir.path().to_path_buf(), 8 * DAY);
        let texts: Vec<_> = reloaded.list().into_iter().map(|e| e.content).collect();
        assert_eq!(texts, [Content::Text { text: "kept".into() }, Content::Text { text: "new".into() }]);
        reloaded.delete_all().unwrap();
        assert_eq!(reloaded.list().len(), 1);
    }

    #[test]
    fn delete_since_removes_only_recent_unpinned_entries() {
        let dir = tempfile::tempdir().unwrap();
        let history = History::load(dir.path().to_path_buf(), 0);
        history.record(text("early"), String::new(), String::new(), 10).unwrap();
        history.record(text("late"), String::new(), String::new(), 100).unwrap();
        history.delete_since(50).unwrap();
        assert_eq!(history.list()[0].content, Content::Text { text: "early".into() });
        assert_eq!(history.list().len(), 1);
    }

    #[test]
    fn image_ids_cannot_escape_the_history_folder() {
        let dir = tempfile::tempdir().unwrap();
        let history = History::load(dir.path().to_path_buf(), 0);
        assert!(history.image_path("..\\settings").is_none());
        assert!(history.image_path("abc123").is_some());
    }

    #[cfg(windows)]
    #[test]
    fn bottom_up_24_bit_dib_converts_to_png() {
        let mut dib = vec![0u8; 40];
        dib[0] = 40;
        dib[4] = 1;
        dib[8] = 2;
        dib[12] = 1;
        dib[14] = 24;
        dib.extend([0, 0, 255, 0, 255, 0, 0, 0]);
        let (png_bytes, w, h) = win::dib_to_png(&dib).unwrap();
        assert_eq!((w, h), (1, 2));
        let mut reader = png::Decoder::new(png_bytes.as_slice()).read_info().unwrap();
        let mut buffer = vec![0; reader.output_buffer_size()];
        reader.next_frame(&mut buffer).unwrap();
        assert_eq!(buffer, [0, 0, 255, 255, 255, 0, 0, 255]);
    }
}
