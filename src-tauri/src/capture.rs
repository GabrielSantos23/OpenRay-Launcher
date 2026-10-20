use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use windows::core::BOOL;
use windows::Win32::Foundation::{HWND, LPARAM, RECT};
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED, DWMWA_EXTENDED_FRAME_BOUNDS};
use windows::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, EnumDisplayMonitors, GetDC, GetDIBits, GetMonitorInfoW, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, CAPTUREBLT,
    DIB_RGB_COLORS, HDC, HMONITOR, MONITORINFO, SRCCOPY,
};
use windows::Win32::UI::Shell::{FOLDERID_Pictures, SHGetKnownFolderPath, KF_FLAG_DEFAULT};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetCursorPos, GetSystemMetrics, GetWindowTextLengthW, GetWindowThreadProcessId, IsIconic, IsWindowVisible, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Default)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    fn from_win(r: RECT) -> Rect {
        Rect { x: r.left, y: r.top, w: r.right - r.left, h: r.bottom - r.top }
    }

    pub fn intersect(&self, other: &Rect) -> Option<Rect> {
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let right = (self.x + self.w).min(other.x + other.w);
        let bottom = (self.y + self.h).min(other.y + other.h);
        (right > x && bottom > y).then_some(Rect { x, y, w: right - x, h: bottom - y })
    }

    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.w && y < self.y + self.h
    }
}

#[derive(Clone)]
pub struct Frame {
    pub rect: Rect,
    pub bgra: Vec<u8>,
}

impl Frame {
    pub fn crop(&self, rect: Rect) -> Option<Frame> {
        let area = rect.intersect(&self.rect)?;
        let mut bgra = Vec::with_capacity((area.w * area.h * 4) as usize);
        for row in 0..area.h {
            let start = (((area.y - self.rect.y + row) * self.rect.w + (area.x - self.rect.x)) * 4) as usize;
            bgra.extend_from_slice(&self.bgra[start..start + (area.w * 4) as usize]);
        }
        Some(Frame { rect: area, bgra })
    }

    pub fn rgba(&self) -> Vec<u8> {
        self.bgra.chunks_exact(4).flat_map(|p| [p[2], p[1], p[0], 255]).collect()
    }

    pub fn png(&self) -> Result<Vec<u8>, String> {
        let mut out = Vec::new();
        let mut encoder = png::Encoder::new(&mut out, self.rect.w as u32, self.rect.h as u32);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_compression(png::Compression::Fast);
        encoder.write_header().and_then(|mut w| w.write_image_data(&self.rgba())).map_err(|e| e.to_string())?;
        Ok(out)
    }

    pub fn bmp(&self) -> Vec<u8> {
        let size = self.bgra.len() as u32;
        let mut out = Vec::with_capacity(54 + self.bgra.len());
        out.extend_from_slice(b"BM");
        out.extend_from_slice(&(54 + size).to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&54u32.to_le_bytes());
        out.extend_from_slice(&40u32.to_le_bytes());
        out.extend_from_slice(&self.rect.w.to_le_bytes());
        out.extend_from_slice(&(-self.rect.h).to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&32u16.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&size.to_le_bytes());
        out.extend_from_slice(&[0u8; 16]);
        out.extend_from_slice(&self.bgra);
        out
    }
}

pub fn virtual_screen() -> Rect {
    unsafe { Rect { x: GetSystemMetrics(SM_XVIRTUALSCREEN), y: GetSystemMetrics(SM_YVIRTUALSCREEN), w: GetSystemMetrics(SM_CXVIRTUALSCREEN), h: GetSystemMetrics(SM_CYVIRTUALSCREEN) } }
}

pub fn grab(rect: Rect) -> Result<Frame, String> {
    if rect.w <= 0 || rect.h <= 0 {
        return Err("The capture area is empty".into());
    }
    unsafe {
        let screen = GetDC(None);
        let memory = CreateCompatibleDC(Some(screen));
        let bitmap = CreateCompatibleBitmap(screen, rect.w, rect.h);
        let old = SelectObject(memory, bitmap.into());
        let copied = BitBlt(memory, 0, 0, rect.w, rect.h, Some(screen), rect.x, rect.y, SRCCOPY | CAPTUREBLT);
        let mut info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER { biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32, biWidth: rect.w, biHeight: -rect.h, biPlanes: 1, biBitCount: 32, biCompression: BI_RGB.0, ..Default::default() },
            ..Default::default()
        };
        let mut bgra = vec![0u8; (rect.w * rect.h * 4) as usize];
        let lines = GetDIBits(memory, bitmap, 0, rect.h as u32, Some(bgra.as_mut_ptr() as _), &mut info, DIB_RGB_COLORS);
        SelectObject(memory, old);
        let _ = DeleteObject(bitmap.into());
        let _ = DeleteDC(memory);
        ReleaseDC(None, screen);
        copied.map_err(|e| e.to_string())?;
        if lines == 0 {
            return Err("Could not read the screen".into());
        }
        Ok(Frame { rect, bgra })
    }
}

pub fn cursor() -> (i32, i32) {
    let mut point = Default::default();
    unsafe {
        let _ = GetCursorPos(&mut point);
    }
    (point.x, point.y)
}

pub fn monitors() -> Vec<Rect> {
    unsafe extern "system" fn each(monitor: HMONITOR, _: HDC, _: *mut RECT, data: LPARAM) -> BOOL {
        let list = &mut *(data.0 as *mut Vec<Rect>);
        let mut info = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
        if GetMonitorInfoW(monitor, &mut info).as_bool() {
            list.push(Rect::from_win(info.rcMonitor));
        }
        true.into()
    }
    let mut list: Vec<Rect> = Vec::new();
    unsafe {
        let _ = EnumDisplayMonitors(None, None, Some(each), LPARAM(&mut list as *mut _ as isize));
    }
    list
}

pub fn window_frame(hwnd: HWND) -> Option<Rect> {
    let mut rect = RECT::default();
    unsafe { DwmGetWindowAttribute(hwnd, DWMWA_EXTENDED_FRAME_BOUNDS, &mut rect as *mut _ as _, std::mem::size_of::<RECT>() as u32) }.ok()?;
    let rect = Rect::from_win(rect);
    (rect.w > 0 && rect.h > 0).then_some(rect)
}

pub fn windows() -> Vec<Rect> {
    unsafe extern "system" fn each(hwnd: HWND, data: LPARAM) -> BOOL {
        let list = &mut *(data.0 as *mut Vec<Rect>);
        let mut cloaked = 0u32;
        let _ = DwmGetWindowAttribute(hwnd, DWMWA_CLOAKED, &mut cloaked as *mut _ as _, 4);
        let mut pid = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if IsWindowVisible(hwnd).as_bool() && !IsIconic(hwnd).as_bool() && cloaked == 0 && GetWindowTextLengthW(hwnd) > 0 && pid != std::process::id() {
            if let Some(rect) = window_frame(hwnd) {
                list.push(rect);
            }
        }
        true.into()
    }
    let mut list: Vec<Rect> = Vec::new();
    unsafe {
        let _ = EnumWindows(Some(each), LPARAM(&mut list as *mut _ as isize));
    }
    list
}

fn pictures() -> PathBuf {
    unsafe { SHGetKnownFolderPath(&FOLDERID_Pictures, KF_FLAG_DEFAULT, None) }
        .ok()
        .and_then(|p| unsafe { p.to_string() }.ok())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(std::env::var("USERPROFILE").unwrap_or_default()).join("Pictures"))
}

pub static CUSTOM_FOLDER: Mutex<Option<PathBuf>> = Mutex::new(None);

pub fn default_folder() -> PathBuf {
    pictures().join("Openray")
}

pub fn folder() -> PathBuf {
    CUSTOM_FOLDER.lock().unwrap_or_else(|e| e.into_inner()).clone().unwrap_or_else(default_folder)
}

pub fn system_folder() -> PathBuf {
    pictures().join("Screenshots")
}

pub fn custom_folder(value: &serde_json::Value) -> Option<PathBuf> {
    value["folder"].as_str().map(str::trim).filter(|p| !p.is_empty()).map(PathBuf::from).filter(|p| p.is_absolute())
}

pub fn file_name(app: &str, random: &str) -> String {
    let app: String = app.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '.').collect();
    format!("{}_{random}.png", if app.is_empty() { "Openray" } else { &app })
}

fn random_id() -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut bytes = [0u8; 10];
    let _ = getrandom::fill(&mut bytes);
    bytes.iter().map(|b| CHARS[*b as usize % CHARS.len()] as char).collect()
}

pub fn save(frame: &Frame, app: &str, now: (i32, u32)) -> Result<(PathBuf, Vec<u8>), String> {
    let dir = folder().join(format!("{:04}-{:02}", now.0, now.1));
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let png = frame.png()?;
    let path = dir.join(file_name(app, &random_id()));
    std::fs::write(&path, &png).map_err(|e| e.to_string())?;
    Ok((path, png))
}

pub fn decode_qr(frame: &Frame) -> Vec<String> {
    let (w, h) = (frame.rect.w as usize, frame.rect.h as usize);
    let mut image = rqrr::PreparedImage::prepare_from_greyscale(w, h, |x, y| {
        let p = &frame.bgra[(y * w + x) * 4..];
        ((p[2] as u32 * 299 + p[1] as u32 * 587 + p[0] as u32 * 114) / 1000) as u8
    });
    image.detect_grids().into_iter().filter_map(|grid| grid.decode().ok().map(|(_, text)| text)).collect()
}

pub fn recognize_text(frame: &Frame) -> Result<String, String> {
    use windows::Graphics::Imaging::{BitmapPixelFormat, SoftwareBitmap};
    use windows::Media::Ocr::OcrEngine;
    use windows::Security::Cryptography::CryptographicBuffer;
    let run = || -> windows::core::Result<String> {
        let engine = OcrEngine::TryCreateFromUserProfileLanguages()?;
        let mut source = frame.clone();
        let max = OcrEngine::MaxImageDimension()? as i32;
        if source.rect.w > max || source.rect.h > max {
            source = source.crop(Rect { w: source.rect.w.min(max), h: source.rect.h.min(max), ..source.rect }).unwrap_or(source);
        }
        let buffer = CryptographicBuffer::CreateFromByteArray(&source.bgra)?;
        let bitmap = SoftwareBitmap::CreateCopyFromBuffer(&buffer, BitmapPixelFormat::Bgra8, source.rect.w, source.rect.h)?;
        let result = engine.RecognizeAsync(&bitmap)?.get()?;
        let mut lines = Vec::new();
        for line in result.Lines()? {
            lines.push(line.Text()?.to_string());
        }
        Ok(lines.join("\n"))
    };
    run().map_err(|e| format!("Text recognition failed: {}", e.message()))
}

pub fn screenshots(dirs: &[PathBuf]) -> Vec<(PathBuf, u64, u64)> {
    let mut list = Vec::new();
    for root in dirs {
        let mut stack = vec![(root.clone(), 0)];
        while let Some((dir, depth)) = stack.pop() {
            for entry in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
                let Ok(meta) = entry.metadata() else { continue };
                let path = entry.path();
                if meta.is_dir() {
                    if depth < 2 {
                        stack.push((path, depth + 1));
                    }
                    continue;
                }
                if is_image(&path) {
                    let modified = meta.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map_or(0, |d| d.as_millis() as u64);
                    list.push((path, modified, meta.len()));
                }
            }
        }
    }
    list.sort_by(|a, b| b.1.cmp(&a.1));
    list.truncate(3000);
    list
}

pub fn is_image(path: &Path) -> bool {
    path.extension().map(|e| e.to_string_lossy().to_lowercase()).is_some_and(|e| ["png", "jpg", "jpeg", "gif", "bmp", "webp"].contains(&e.as_str()))
}

pub static FROZEN: Mutex<Option<Frame>> = Mutex::new(None);
pub static LAST_REGION: Mutex<Option<Rect>> = Mutex::new(None);

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(w: i32, h: i32) -> Frame {
        Frame { rect: Rect { x: 10, y: 20, w, h }, bgra: (0..w * h).flat_map(|i| [i as u8, 0, 0, 255]).collect() }
    }

    #[test]
    fn crops_inside_the_frame() {
        let f = frame(4, 3);
        let c = f.crop(Rect { x: 11, y: 21, w: 2, h: 5 }).unwrap();
        assert_eq!(c.rect, Rect { x: 11, y: 21, w: 2, h: 2 });
        assert_eq!(c.bgra.iter().step_by(4).copied().collect::<Vec<_>>(), vec![5, 6, 9, 10]);
        assert!(f.crop(Rect { x: 0, y: 0, w: 5, h: 5 }).is_none());
    }

    #[test]
    fn encodes_png_and_bmp() {
        let f = frame(3, 2);
        assert_eq!(&f.png().unwrap()[1..4], b"PNG");
        let bmp = f.bmp();
        assert_eq!(&bmp[..2], b"BM");
        assert_eq!(bmp.len(), 54 + 3 * 2 * 4);
        assert_eq!(i32::from_le_bytes(bmp[22..26].try_into().unwrap()), -2);
    }

    #[test]
    fn names_files_like_sharex() {
        assert_eq!(file_name("chrome", "AbC123xyZ0"), "chrome_AbC123xyZ0.png");
        assert_eq!(file_name("my app?", "x"), "myapp_x.png");
        assert_eq!(file_name("", "x"), "Openray_x.png");
        assert_eq!(random_id().len(), 10);
    }

    #[test]
    fn reads_the_saved_folder() {
        assert_eq!(custom_folder(&serde_json::json!({ "folder": r"D:\Shots" })), Some(PathBuf::from(r"D:\Shots")));
        assert_eq!(custom_folder(&serde_json::json!({ "folder": "relative" })), None);
        assert_eq!(custom_folder(&serde_json::json!({ "folder": "  " })), None);
        assert_eq!(custom_folder(&serde_json::Value::Null), None);
    }

    #[test]
    fn decodes_nothing_from_a_blank_image() {
        assert!(decode_qr(&Frame { rect: Rect { x: 0, y: 0, w: 40, h: 40 }, bgra: vec![255; 40 * 40 * 4] }).is_empty());
    }

    #[test]
    #[ignore = "reads the real screen"]
    fn grabs_the_primary_screen_and_reads_text() {
        let frame = grab(virtual_screen()).unwrap();
        println!("{:?} monitors {:?} windows {}", frame.rect, monitors(), windows().len());
        println!("{}", recognize_text(&frame.crop(Rect { x: 0, y: 0, w: 800, h: 300 }).unwrap()).unwrap());
    }
}
