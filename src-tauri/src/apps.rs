use serde::Serialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct InstalledApp {
    pub name: String,
    pub path: String,
}

const EXTENSIONS: [&str; 3] = ["lnk", "url", "appref-ms"];
const APPS_FOLDER: &str = r"shell:AppsFolder\";

fn with_com<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> T {
    use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED};
    std::thread::spawn(move || unsafe {
        let initialized = CoInitializeEx(None, COINIT_APARTMENTTHREADED).is_ok();
        let result = work();
        if initialized {
            CoUninitialize();
        }
        result
    })
    .join()
    .expect("COM worker thread panicked")
}

pub fn packaged_apps() -> Vec<InstalledApp> {
    with_com(|| unsafe {
        use windows::Win32::System::Com::CoTaskMemFree;
        use windows::Win32::UI::Shell::{
            BHID_EnumItems, FOLDERID_AppsFolder, IEnumShellItems, IShellItem, SHGetKnownFolderItem, KF_FLAG_DEFAULT, SIGDN,
            SIGDN_NORMALDISPLAY, SIGDN_PARENTRELATIVEPARSING,
        };
        let display = |item: &IShellItem, kind: SIGDN| {
            item.GetDisplayName(kind).ok().map(|text| {
                let value = text.to_string().unwrap_or_default();
                CoTaskMemFree(Some(text.0 as _));
                value
            })
        };
        let mut apps = Vec::new();
        let Ok(folder) = SHGetKnownFolderItem::<IShellItem>(&FOLDERID_AppsFolder, KF_FLAG_DEFAULT, None) else { return apps };
        let Ok(items) = folder.BindToHandler::<_, IEnumShellItems>(None, &BHID_EnumItems) else { return apps };
        loop {
            let mut batch = [None];
            let mut fetched = 0;
            if items.Next(&mut batch, Some(&mut fetched)).is_err() || fetched == 0 {
                break;
            }
            let Some(item) = batch[0].take() else { break };
            if let (Some(name), Some(id)) = (display(&item, SIGDN_NORMALDISPLAY), display(&item, SIGDN_PARENTRELATIVEPARSING)) {
                if id.contains('!') {
                    apps.push(InstalledApp { name, path: format!("{APPS_FOLDER}{id}") });
                }
            }
        }
        apps
    })
}

pub fn icon_png(path: String, size: i32, thumbnail: bool) -> Result<Vec<u8>, String> {
    with_com(move || unsafe {
        use windows::core::HSTRING;
        use windows::Win32::Foundation::SIZE;
        use windows::Win32::Graphics::Gdi::{
            DeleteObject, GetDC, GetDIBits, GetObjectW, ReleaseDC, BITMAP, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
        };
        use windows::Win32::UI::Shell::{IShellItemImageFactory, SHCreateItemFromParsingName, SIIGBF_ICONONLY, SIIGBF_RESIZETOFIT};

        let factory: IShellItemImageFactory = SHCreateItemFromParsingName(&HSTRING::from(path.as_str()), None).map_err(|e| e.to_string())?;
        let bitmap = factory.GetImage(SIZE { cx: size, cy: size }, if thumbnail { SIIGBF_RESIZETOFIT } else { SIIGBF_ICONONLY }).map_err(|e| e.to_string())?;
        let mut header = BITMAP::default();
        GetObjectW(bitmap.into(), size_of::<BITMAP>() as i32, Some((&mut header as *mut BITMAP).cast()));
        let (width, height) = if header.bmWidth > 0 && header.bmHeight != 0 { (header.bmWidth, header.bmHeight.abs()) } else { (size, size) };
        let mut info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut pixels = vec![0u8; (width * height * 4) as usize];
        let dc = GetDC(None);
        let lines = GetDIBits(dc, bitmap, 0, height as u32, Some(pixels.as_mut_ptr().cast()), &mut info, DIB_RGB_COLORS);
        ReleaseDC(None, dc);
        let _ = DeleteObject(bitmap.into());
        if lines != height {
            return Err(format!("GetDIBits copied {lines} of {height} lines"));
        }
        to_straight_rgba(&mut pixels);
        let mut png = Vec::new();
        let mut encoder = png::Encoder::new(&mut png, width as u32, height as u32);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.write_header().and_then(|mut w| w.write_image_data(&pixels)).map_err(|e| e.to_string())?;
        Ok(png)
    })
}

fn to_straight_rgba(pixels: &mut [u8]) {
    let opaque = pixels.chunks_exact(4).all(|p| p[3] == 0);
    for p in pixels.chunks_exact_mut(4) {
        p.swap(0, 2);
        if opaque {
            p[3] = 255;
        } else if p[3] > 0 && p[3] < 255 {
            let alpha = p[3] as u32;
            for c in &mut p[..3] {
                *c = ((*c as u32 * 255 + alpha / 2) / alpha).min(255) as u8;
            }
        }
    }
}

pub fn control_panel_items() -> Vec<InstalledApp> {
    with_com(|| unsafe {
        use windows::core::w;
        use windows::Win32::System::Com::CoTaskMemFree;
        use windows::Win32::UI::Shell::{BHID_EnumItems, IEnumShellItems, IShellItem, SHCreateItemFromParsingName, SIGDN, SIGDN_DESKTOPABSOLUTEPARSING, SIGDN_NORMALDISPLAY};
        let display = |item: &IShellItem, kind: SIGDN| {
            item.GetDisplayName(kind).ok().map(|text| {
                let value = text.to_string().unwrap_or_default();
                CoTaskMemFree(Some(text.0 as _));
                value
            })
        };
        let mut items = Vec::new();
        let Ok(folder) = SHCreateItemFromParsingName::<_, _, IShellItem>(w!(r"shell:::{26EE0668-A00A-44D7-9371-BEB064C98683}\0"), None) else { return items };
        let Ok(children) = folder.BindToHandler::<_, IEnumShellItems>(None, &BHID_EnumItems) else { return items };
        loop {
            let mut batch = [None];
            let mut fetched = 0;
            if children.Next(&mut batch, Some(&mut fetched)).is_err() || fetched == 0 {
                break;
            }
            let Some(item) = batch[0].take() else { break };
            if let (Some(name), Some(id)) = (display(&item, SIGDN_NORMALDISPLAY), display(&item, SIGDN_DESKTOPABSOLUTEPARSING)) {
                items.push(InstalledApp { name, path: format!("shell:{id}") });
            }
        }
        items
    })
}

pub fn start_menu_roots() -> Vec<PathBuf> {
    ["ProgramData", "APPDATA"]
        .iter()
        .filter_map(|var| std::env::var_os(var))
        .map(|base| PathBuf::from(base).join(r"Microsoft\Windows\Start Menu\Programs"))
        .collect()
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(&path, out);
        } else if path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| EXTENSIONS.iter().any(|x| x.eq_ignore_ascii_case(e)))
        {
            out.push(path);
        }
    }
}

fn is_uninstaller(name: &str) -> bool {
    name.to_lowercase().starts_with("uninstall")
}

pub fn scan(roots: &[PathBuf], packaged: Vec<InstalledApp>) -> Vec<InstalledApp> {
    let mut by_name: BTreeMap<_, _> = packaged.into_iter().map(|app| (app.name.to_lowercase(), app)).collect();
    for root in roots {
        let mut files = Vec::new();
        walk(root, &mut files);
        for path in files {
            let Some(name) = path.file_stem().and_then(|s| s.to_str()).map(str::to_owned) else { continue };
            if is_uninstaller(&name) {
                continue;
            }
            by_name.insert(name.to_lowercase(), InstalledApp { name, path: path.to_string_lossy().into_owned() });
        }
    }
    by_name.into_values().collect()
}

pub fn launch(path: &str, as_admin: bool) -> Result<(), String> {
    use windows::core::{w, HSTRING, PCWSTR};
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::{AllowSetForegroundWindow, ASFW_ANY, SW_SHOWNORMAL};

    if !path.starts_with("shell:") && !path.starts_with("ms-settings:") && !Path::new(path).exists() {
        return Err(format!("{path} no longer exists"));
    }
    unsafe {
        let _ = AllowSetForegroundWindow(ASFW_ANY);
    }
    let result = unsafe { ShellExecuteW(None, if as_admin { w!("runas") } else { w!("open") }, &HSTRING::from(path), PCWSTR::null(), PCWSTR::null(), SW_SHOWNORMAL) };
    if result.0 as isize > 32 {
        Ok(())
    } else {
        Err(format!("ShellExecute failed with code {}", result.0 as isize))
    }
}

pub fn open_url(url: &str, app_path: Option<&str>) -> Result<(), String> {
    use windows::core::{w, HSTRING, PCWSTR};
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::{AllowSetForegroundWindow, ASFW_ANY, SW_SHOWNORMAL};

    if url.trim().is_empty() {
        return Err("the link is empty".into());
    }
    unsafe {
        let _ = AllowSetForegroundWindow(ASFW_ANY);
    }
    let result = match app_path.filter(|p| !p.is_empty()) {
        Some(app) => unsafe {
            ShellExecuteW(None, w!("open"), &HSTRING::from(app), &HSTRING::from(format!("\"{}\"", url.replace('"', "%22"))), PCWSTR::null(), SW_SHOWNORMAL)
        },
        None => unsafe { ShellExecuteW(None, w!("open"), &HSTRING::from(url), PCWSTR::null(), PCWSTR::null(), SW_SHOWNORMAL) },
    };
    if result.0 as isize > 32 {
        Ok(())
    } else {
        Err(format!("could not open {url} (ShellExecute code {})", result.0 as isize))
    }
}

pub fn show_in_explorer(path: &str) -> Result<(), String> {
    if !Path::new(path).exists() {
        return Err(format!("{path} no longer exists"));
    }
    use std::os::windows::process::CommandExt;
    std::process::Command::new("explorer.exe").raw_arg(format!("/select,\"{path}\"")).spawn().map(drop).map_err(|e| e.to_string())
}

pub fn edit(path: &str) -> Result<(), String> {
    use windows::core::{w, HSTRING, PCWSTR};
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
    if !Path::new(path).exists() {
        return Err(format!("{path} no longer exists"));
    }
    let result = unsafe { ShellExecuteW(None, w!("edit"), &HSTRING::from(path), PCWSTR::null(), PCWSTR::null(), SW_SHOWNORMAL) };
    if result.0 as isize > 32 {
        Ok(())
    } else {
        open_with(path)
    }
}

pub fn open_with(path: &str) -> Result<(), String> {
    if !Path::new(path).exists() {
        return Err(format!("{path} no longer exists"));
    }
    use std::os::windows::process::CommandExt;
    std::process::Command::new("rundll32.exe").raw_arg(format!("shell32.dll,OpenAs_RunDLL {path}")).spawn().map(drop).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn touch(path: &Path) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, b"").unwrap();
    }

    #[test]
    fn scans_shortcuts_recursively_dedupes_and_sorts() {
        let common = tempfile::tempdir().unwrap();
        let user = tempfile::tempdir().unwrap();
        touch(&common.path().join("Zed.lnk"));
        touch(&common.path().join(r"Accessories\Notepad.lnk"));
        touch(&common.path().join("Docs.url"));
        touch(&common.path().join("ClickOnce.appref-ms"));
        touch(&common.path().join("readme.txt"));
        touch(&common.path().join(r"Tool\Uninstall Tool.lnk"));
        touch(&user.path().join("notepad.LNK"));

        let apps = scan(&[common.path().to_path_buf(), user.path().to_path_buf()], Vec::new());
        let names: Vec<_> = apps.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(names, ["ClickOnce", "Docs", "notepad", "Zed"]);
        assert!(apps[2].path.starts_with(&*user.path().to_string_lossy()));
    }

    #[test]
    fn missing_roots_are_ignored() {
        assert!(scan(&[PathBuf::from(r"Z:\does\not\exist")], Vec::new()).is_empty());
    }

    #[test]
    fn shortcuts_win_over_packaged_apps_with_the_same_name() {
        let dir = tempfile::tempdir().unwrap();
        touch(&dir.path().join("Notepad.lnk"));
        let packaged = |name: &str| InstalledApp { name: name.into(), path: format!("{APPS_FOLDER}{name}!App") };
        let apps = scan(&[dir.path().to_path_buf()], vec![packaged("notepad"), packaged("Calculator")]);
        let names: Vec<_> = apps.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(names, ["Calculator", "Notepad"]);
        assert!(apps[1].path.ends_with("Notepad.lnk"));
    }

    #[test]
    fn converts_premultiplied_bgra_to_straight_rgba() {
        let mut pixels = [10, 20, 30, 255, 0, 64, 128, 128, 0, 0, 0, 0];
        to_straight_rgba(&mut pixels);
        assert_eq!(pixels, [30, 20, 10, 255, 255, 128, 0, 128, 0, 0, 0, 0]);
        let mut no_alpha = [1, 2, 3, 0];
        to_straight_rgba(&mut no_alpha);
        assert_eq!(no_alpha, [3, 2, 1, 255]);
    }
}
