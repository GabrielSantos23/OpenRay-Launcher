use windows::core::Interface;
use windows::Win32::System::Variant::VARIANT;
use windows::Win32::Foundation::HWND;
use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, IServiceProvider, CLSCTX_ALL, COINIT_APARTMENTTHREADED};
use windows::Win32::UI::Shell::{IFolderView, IShellBrowser, IShellItemArray, IShellWindows, IWebBrowserApp, ShellWindows, SID_STopLevelBrowser, SIGDN_FILESYSPATH, SVGIO_SELECTION};
use windows::Win32::UI::WindowsAndMessaging::GetAncestor;
use windows::Win32::UI::WindowsAndMessaging::GA_ROOT;

fn selection_of(window: &IWebBrowserApp) -> windows::core::Result<Vec<String>> {
    unsafe {
        let provider: IServiceProvider = window.cast()?;
        let browser: IShellBrowser = provider.QueryService(&SID_STopLevelBrowser)?;
        let view: IFolderView = browser.QueryActiveShellView()?.cast()?;
        let items: IShellItemArray = view.Items(SVGIO_SELECTION)?;
        let mut paths = Vec::new();
        for i in 0..items.GetCount()? {
            let name = items.GetItemAt(i)?.GetDisplayName(SIGDN_FILESYSPATH)?;
            paths.push(name.to_string().unwrap_or_default());
            windows::Win32::System::Com::CoTaskMemFree(Some(name.0 as _));
        }
        Ok(paths)
    }
}

pub fn selected_files(foreground: HWND) -> Result<Vec<String>, String> {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let windows: IShellWindows = CoCreateInstance(&ShellWindows, None, CLSCTX_ALL).map_err(|e| e.to_string())?;
        let root = GetAncestor(foreground, GA_ROOT);
        for i in 0..windows.Count().map_err(|e| e.to_string())? {
            let Ok(item) = windows.Item(&VARIANT::from(i)) else { continue };
            let Ok(app) = item.cast::<IWebBrowserApp>() else { continue };
            if app.HWND().map(|h| h.0 as isize) != Ok(root.0 as isize) {
                continue;
            }
            if let Ok(paths) = selection_of(&app) {
                if !paths.is_empty() {
                    return Ok(paths);
                }
            }
        }
    }
    Err("Select a file in File Explorer first".into())
}
