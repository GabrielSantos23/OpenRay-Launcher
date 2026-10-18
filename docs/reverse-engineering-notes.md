# Reverse-engineering notes

Reference inspected on 2026-10-18.

## Version discrepancy

`AGENTS.md` names Raycast for Windows **2.4.0.0**. The machine has **2.8.2.0** installed, and no 2.4.0.0 copy was found.

- Evidence: `Get-AppxPackage` → `Raycast.Raycast 2.8.2.0`, InstallLocation `C:\Program Files\WindowsApps\Raycast.Raycast_2.8.2.0_x64__qypenmj9wpt2a`.
- Evidence: `Raycast.dll` assembly version 2.8.2.0 (REA `ev_8bffd5e629469c3657b93a79947beef0b3552f11987966e56ef0ec6fd7dbc504`).

All findings below are for 2.8.2.0.

The machine also runs **RaycastMini** (`%LOCALAPPDATA%\RaycastMini`, window title "Openray"). It is the owner's previous clone attempt, not the reference. Both processes listen for Alt+Space.

## How the reference is built

### Runtime and UI technology

A hybrid of three runtimes:

1. **Native host: .NET 10 WPF** (`Raycast.exe` + `Raycast.dll`).
   - `Raycast.runtimeconfig.json`: `tfm net10.0`, frameworks `Microsoft.NETCore.App 10.0.12` and `Microsoft.WindowsDesktop.App 10.0.12`.
   - REA: `Raycast.dll`, IL-only, target framework `.NETCoreApp,Version=v10.0`, 4,764 TypeDefs, 38,017 MethodDefs.
   - Self-contained: the app folder ships `coreclr.dll`, `PresentationFramework.dll`, `wpfgfx_cor3.dll` and the WinForms assemblies.
   - UI libraries: `Wpf.Ui.dll`, `H.NotifyIcon.Wpf.dll` (tray), `CommunityToolkit.Mvvm.dll`.
   - Window classes are `HwndWrapper[Raycast;Main;<guid>]`, which is the WPF HWND class.
2. **UI: web front-end in WebView2.**
   - Files: `Microsoft.Web.WebView2.Core/Wpf/WinForms.dll`, `WebView2Loader.dll`.
   - `Raycast\frontend\` holds 1,363 files from a Vite build with hashed chunk names: one HTML entry per window (`main-window.html`, `settings-window.html`, `notes-window.html`, `ai-chat-window.html`, `theme-studio-window.html`, `welcome-window.html`, …) plus JS and CSS chunks. The main chunk `main-window-BN8deh-D.js` is 1.6 MB.
   - Bundled fonts: `InterVariable.woff2`, `InterVariable-Italic.woff2`, `JetBrainsMono-Regular.woff2`.
3. **Backend: Node.js.**
   - `Raycast\backend\node.exe` (87 MB) runs `index.mjs` (7.4 MB bundle).
   - Native Rust N-API modules: `indexer.win32-x64-msvc.node`, `data.win32-x64-msvc.node` (46 MB), `fs-utils…node`, `process-utils…node`, `calculator-rs…node`.
   - Calculator: SoulverCore on the Swift runtime for Windows (`SoulverCoreDynamic.dll`, `swiftCore.dll`, `Foundation.dll`).
   - Other bundled tools: `rg.exe` (ripgrep) and `Raycast Content Indexer.exe`.
4. **Extension API runtime.**
   - `Raycast\api\node_modules\@raycast\api`, `react`, `react-devtools-raycast`.
   - Third-party extensions are React components rendered by Node, the same model as on macOS.

### Executables and processes

From the files in `Raycast\` and the AppxManifest:

| Executable | Role (evidence) |
|---|---|
| `Raycast.exe` | Main WPF host. Manifest `Application Executable="Raycast\Raycast.exe"`. It is also the COM server for toast activation, CLSID `FA056661-…`. |
| `Raycast.UIAccess.exe` | Manifest `windows.fullTrustProcess` with `/uiaccess`. The `uiaccess` restricted capability is declared. Used for input and focus over elevated windows. |
| `Raycast.SystemService.exe` | Helper service. The `InstallSystemService` type exists in `Raycast.dll`. |
| `Raycast.StartupLauncher.exe` | Start-at-login launcher (`BuildStartupLauncherCommandLine`). |
| `Raycast.CrashHelper.exe`, `Raycast.TraceRecorder.exe`, `createdump.exe` | Diagnostics. |
| `backend\node.exe` | Backend. Running instances were visible as `node` processes. |

The package also declares the `raycast://` deep-link protocol and the `com.raycast` OAuth protocol.

### Launcher window

Measured with `EnumWindows`, `GetWindowRect`, `GetWindowLongPtr` and `DwmGetWindowAttribute` on the live process (PID 18708), plus a screenshot taken after a physical Alt+Space (`scratchpad/ref2-full.png`).

- **One persistent HWND.** It is `HwndWrapper[Raycast;Main;…]`, title "Raycast", at 750×475 px, 96 DPI. It stays mapped while hidden.
- **Hidden ("parked") state.**
  - Style `0x16000000`: WS_VISIBLE | WS_CLIPSIBLINGS | WS_CLIPCHILDREN, with no WS_CAPTION. The window is frameless.
  - Ex-style `0x080800A0`: WS_EX_NOACTIVATE | WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_TRANSPARENT.
  - So the window is hidden by transparency and click-through, not by `ShowWindow(SW_HIDE)`. `Raycast.dll` contains `HideWindowTransparently` and `HideWindowTransparentlyForOcclusionBypass`.
- **Corners and border.**
  - `DWMWA_WINDOW_CORNER_PREFERENCE` = 2 (DWMWCP_ROUND).
  - Observed 1 px border with color `#3B3B3B` (59,59,59) on top and `#3A3A3A` on the sides.
- **Always on top and transparency.** The WPF `Topmost` and `AllowsTransparency` symbols are present in `Raycast.dll`.
- **Backdrop.** `Raycast.dll` references `SetWindowCompositionAttribute`, `DwmSetWindowAttribute`, `AcrylicBlur`, `MicaBlur` and `AcrylicBackdrop`. Whether the default launcher uses a backdrop is unknown, see `unk_86e9…`.
- **Focus.**
  - `Raycast.dll` contains `ForceActivateWindow`, `AttachThreadInput`, `AllowSetForegroundWindow`, `EnableMainWindowHideOnFocusLoss` and `ForegroundWindowScope`.
  - The launcher hides on focus loss: setting `EnableMainWindowHideOnFocusLoss`.
- **Position.** On the 1920×1080 display the window appeared at (627,381), which is not centered. See `unk_341f…`.
  - Command `resetPosition`, "Reset Raycast Window Position", exists, so the position is user-movable and remembered.
  - Setting `windowPresentationMode`, "Window Position", picks the monitor.

### Global hotkey

- `Raycast.dll` imports `SetWindowsHookEx` and has the type `LowLevelKeyboardEvent`. `RegisterHotKey` was **not found** in its strings.
- The hotkey is therefore a WH_KEYBOARD_LL hook, which is how it can take Alt+Space and Win-key combinations.
- An injected `keybd_event` Alt+Space was received by RaycastMini. The real Raycast responded to a later physical press. The live state was observed but the cause was not proven.
- The setting `globalHotkey` is titled "Raycast Hotkey", in `backend/index.mjs` settings metadata. It is serialized for Windows as `XRayHotkeyWindows`.
- The default value is not readable, because settings are encrypted (see below). The owner's configured value is Alt+Space.

### App indexing

`Raycast.AppDiscovery.*` contains:

- Collectors: `ShortcutApplicationWatcher` (Start Menu `.lnk`), `MsixApplicationsCollector` / `MsixApplicationWatcher` (packaged apps), `FolderApplicationsWatcher`, `WingetLinksApplicationWatcher`, `ControlPanelItemsDiscovery`, `SystemPreferencesDiscovery`.
- Game launchers: `RiotLauncherBuilder`, `EpicLauncherBuilder`, `GOGLauncherBuilder`, `BlizzardLauncherBuilder`, `EALauncherBuilder`, `BattlestateLauncherBuilder`.
- Launch and support:
  - Launch goes through `ShellExecuteEx` and `IApplicationActivationManager` (MSIX).
  - Shortcuts are parsed with `IShellLinkW_Extensions`.
  - Icons come from `PngIconDataProvider`.
- Results are stored in `%LOCALAPPDATA%\Raycast\app_index.db`.

### Clipboard

- `AddClipboardFormatListener` is used.
- A hidden window of class `RaycastClipboardListener` exists in the live process.
- Code lives in `Raycast.Clipboard.ClipboardEntryProducer`.
- Storage is `clipboard.db`.

### Window management

- `Raycast.Handlers.WindowManagement` and `Raycast.Handlers.VirtualDesktops`.
- Win32 calls: `SetWindowPos`, `EnumWindows`, `MonitorFromWindow`, `GetDpiForWindow` and `DwmGetWindowAttribute` (extended frame bounds).
- FFI types `FfiConverterTypeSetWindowPosOutcome` and `FfiConverterTypeToggleWindowTopmostOutcome` suggest a native helper library. That helper is unknown.

### Settings and persistence

- Location: `%LOCALAPPDATA%\Raycast\`.
- Databases: `settings.db`, `settings_v2.db`, `main.db`, `frecency.db`, `app_index.db`, `clipboard.db`, `user_activity.db`, `node_extensions.db`, `emoji.db`, `screenshots.db`, `ai.db`. All are SQLite in WAL mode (`-wal`/`-shm` files).
- **Every `.db` is encrypted.** The first 16 bytes are not `SQLite format 3\0`, and REA `inspect_sqlite_database` reports "file is not a database". A 64-byte `last_key` file sits beside them. We did not try to decrypt.
- The settings schema is declarative in `backend/index.mjs`:
  - Each preference is `bg({default, storage:"synced"|"local"})`.
  - General settings keys include `globalHotkey`, `windowMode` (`compact`/`expanded`), `windowPresentationMode`, `popToRootTimeout`, `escapeKeyBehavior`, `escapeKeyClosesWindow`, `showFavoritesInCompactMode`, `faviconProvider`, `uiZoom`, `themeDarkId`/`themeLightId`, `openAtLogin` and `showInMenuBar`.

### Frecency

From `backend/index.mjs`, a class with `prefetch`, `getScore` and `decayedScore`:

- **Decayed score:** `max(1, score · 2^(−Δt / 864e6 ms))`. That is exponential decay with a **10-day half-life** and a floor of 1.
- **Visit:** `G.frecency.visit(key, visitDate, searchTerm)` runs in the native `data` module. The increment is unknown (`unk_f0ba…`). The search term is stored with the visit.
- **Keys:** `commandId` + `instanceId`, for example the application command id plus `raycastAppId`.

### Extensions

**Built-in extensions.** These are declarative objects in `backend/index.mjs`:

- An extension is `{key, title, description, icon, settings, commands:{<name>: {title, subtitle, description, icon, type, keywords, suggestionPriority, fallbackStatus, rootSearch}}}`.
- Each command also carries an `.actions([...])` list.
- Command `type` values seen: `view-command`, `no-view-command`, `ai-extension-command`.
- Some commands have inline argument kinds: `text`, `dropdown`, `checkbox`.

**Third-party extensions.**

- They run in Node workers, which are prewarmed in a pool (`getFromPool`, `prewarmed`).
- They use `@raycast/api` + React from `Raycast\api\node_modules`.
- Installed ones live in `%LOCALAPPDATA%\Raycast\extensions` and `node_extensions.db`.

**IPC.**

- `Raycast.IPC.*` defines `Inbound`, `Outbound` and `FrontendResponseEnvelope` for host, backend and frontend.
- `Raycast.InterRaycastPipe` uses a named pipe for single-instance and deep links (`BringRaycastToFront`, `RaycastDeepLinkData`).

### Visual tokens (CSS)

From `frontend/css-C58pR_1e.css` and `main-window-38KZXJcp.css`:

- **Scaling:** `--spx: calc(1px * var(--zoom,1))`.
- **Spacing:** `--space-2…56` = n × spx.
- **Fonts:**
  - Sizes: `--font-size-8/11/13/16/18/24`.
  - UI font: `InterVariable`. A Windows fallback stack `Segoe UI Variable, Segoe UI, InterVariable` also appears.
- **Icons:** `--icon-12/16/22/32`.
- **Window dimensions:**
  - `--main-window-width: 750·spx`
  - `--main-window-header-height: 64·spx` (66 in one variant)
  - `--main-window-footer-height: 42·spx` (one rule uses `--default-window-footer-height`)
  - `--root-padding: --space-8`
- **Radii:** `--radius-window: 8px` (macOS-26 and macOS-27 variants also exist).
- **Colors:**
  - Derived from theme variables `--bg`, `--fg`, `--selection` and `--accent`.
  - Text: `--color-text-secondary: fg 60%`, `--color-text-placeholder: fg 40%`.
  - Lists: `--color-surface-list-selection: fg 10%`, `--color-surface-list-hover: fg 5%`.
  - Borders and window: `--color-border-separator: fg 10%`, `--color-window-bg: bg 40%`.
- The theme values for `--bg` and `--fg` are not in the CSS. They come from theme JS, which was not located.

## Feature inventory source

- **Built-in commands.** The list in `docs/feature-matrix.md` was produced by parsing `backend/index.mjs` for `key:` / `title:` / `commands:{…H({title…})}`. The raw dump is in the kickoff scratchpad `inventory.txt`.
- **Window Management commands.** Titles were taken from the `window-management` extension segment (offset 1020541).
- **Feedback categories** in the bundle list the product areas: AI, Applications, Auto Quit, Calculator, Calendar/My Schedule, Clipboard History, Cloud Sync, Custom Themes, Deeplinks, Dictation, Extensions, Flight Tracker, Focus, Hyper Key, Notes, Quicklinks, Script Commands, Search Files, Settings, Snippets, Store, Window Management.

## Open unknowns (REA ledger)

| ID | Question | Severity |
|---|---|---|
| `unk_341f93db…` | Default launcher position rule (observed (627,381) on 1920×1080, not centered) | medium |
| `unk_f0bab293…` | Frecency visit increment (native module) | low |
| `unk_93ecb166…` | Root-search matching and ranking algorithm | medium |
| `unk_86e9f470…` | Launcher backdrop: opaque vs acrylic or mica | medium |
| `unk_31815e75…` | Which bundle commands are visible on Windows | medium |
| `unk_ce5e0942…` | Focus acquisition sequence on show (NOACTIVATE while parked, UIAccess helper) | high |

Not yet investigated:

- Theme color values (`--bg`, `--fg`, `--selection`).
- Exact row text metrics.
- Show and hide animation.
- Compact mode height.
