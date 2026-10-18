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

### Show and hide sequence (measured)

Measured on 2026-10-18 for an injected Alt+Space.

- **Method:** a probe polled every 1 ms for the foreground window, the launcher's ex-style, and one screen pixel inside the launcher. The script is `showprobe.ps1` in the session scratchpad. It ran outside REA, so it has no REA evidence ID.

| t after key | Show | Hide |
|---|---|---|
| ~6 ms | ex-style `0x080800A0` → `0x00080000` (TRANSPARENT, NOACTIVATE and TOOLWINDOW removed) | — |
| ~21 ms | Launcher painted and foreground | — |
| ~30 ms | — | ex-style back to `0x080800A0`; foreground returns to the app that was active before |
| ~47 ms | `WS_EX_TOPMOST` added | — |

- **No DWM animations.** The window is never shown or hidden with `ShowWindow`, so DWM plays no fade. A `ShowWindow`-based hide in Openray's first skeleton faded for about 150 ms.
- **Fresh content on reopen.** The window stays mapped, so the WebView keeps rendering while invisible and can reset before the next show. This matches the `HideWindowTransparentlyForOcclusionBypass` symbol.
- **Open question.** Which API grants foreground (`AttachThreadInput` or the UIAccess helper) is still open in `unk_ce5e…`.

### Global hotkey

- `Raycast.dll` imports `SetWindowsHookEx` and has the type `LowLevelKeyboardEvent`. `RegisterHotKey` was **not found** in its strings.
- The hotkey is therefore a WH_KEYBOARD_LL hook, which is how it can take Alt+Space and Win-key combinations.
- **Injected input.** Raycast reacts to injected `keybd_event` input; the LLKHF_INJECTED events in the 2026-10-18 probe toggled it.
- **Competing hooks.** When two apps hook the same chord, the most recently installed hook sees it first. With Openray's hook installed after Raycast's, the first Alt+Space went to Openray. Later presses reached Raycast, which suggests Raycast re-registers its hook. That is not confirmed.
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

### Settings window

Sources are in `Raycast\frontend\`.

- **Routes (`settings-routes-DC7HQqEu.js`).** `/general`, `/launcher`, `/shortcuts`, `/keyboard`, `/advanced`, `/about`, plus `/extensions/<id>` per built-in extension.
- **General defaults (`register-query-client-k6wPQZTD.js`, `gn`).** `openAtLogin: true`, `showInMenuBar: true`, `windowMode: "expanded"`, `windowPresentationMode: "mouse-screen"`, `popToRootTimeout: 90`, `escapeKeyBehavior: "pop-back-or-close"`, `escapeKeyClosesWindow: true`, `navigationBindings: "emacs"`, `pageNavigationKeys: "square-brackets"`, `rootSearchSensitivity: "high"`, `uiZoom: 1`. `openAtLogin` changes go to the host through `settings.openAtLogin`.
- **Option lists.**
  - `keyboard-settings-view-Bzez7uVc.js`: Escape `pop-back-or-close` (“Navigate back or close window”) and `close-and-pop-to-root` (“Close window and pop to root”).
  - `launcher-settings-view-DyZc2mbk.js`: Show on `mouse-screen`, `active-screen`, `primary-screen`; pop-to-root 0, 5, 15, 30, 60, 90, 120 and 180 s; sensitivity low, medium and high.
- **Row copy.** Titles and descriptions are in `keyboard-settings-page-definition-FqDoobrW.js` and `launcher-settings-page-definition-6EFT4SqQ.js`. Escape Key Closes Window reads “Dismiss windows such as AI Chat, Notes, and Settings by pressing the Escape key”.
- **Layout CSS.**
  - `settings-ui-OsI-JiRS.css`: row min-height 46·spx with padding 12, card `rgb(fg / 2.5%)`, section gap 4, section title min-height 32.
  - `settings-window-Bw5-XMVM.css`: nav link 32 px with padding 6 and gap 8, icon 22, selection `--selection-5`; on Windows the title bar uses `--fg-5` with a 1 px separator and the sidebar has a right border.
- **Hotkey recorder (`keyboard-shortcut-recorder-DpQf5nsV.js`).**
  - The popover is 320×170·spx, `side: top`, `align: center`, with an arrow.
  - Messages: “e.g.” with cycling Single Step / Single Key / Double Tap hints; “Press Backspace to delete” + keys + “or record a new hotkey”; “Already used by **name**” + “Overwrite it or record a new hotkey” (command conflict) or “Discard or record a new hotkey”; “This key combination isn't supported”.
  - Keys: Escape discards, Backspace clears, Enter overwrites on a command conflict.
  - On save it shows the `saved` green tint for 200 ms (`xe=200`). The trigger text is “Recording…” while open.
- **Select (`select-kjjMNEfX.js`).** A popover with a menu whose search field is on by default and max height 280. Arrow keys on the closed trigger change the value; Space and Enter open it; typing a letter opens it with that search term.
- **Scroll area (`scroll-area-D6piR2l3.css`).** A custom overlay scrollbar, 6 px wide (`--thumb-size`) with an `--fg-40` thumb, widening to 11 px with an `--fg-5` track on hover after a 2 s delay. Native scrollbars are hidden.
- **Themes (`contrast-improvement-CQ0EgiTy.js`).** Eight bundled themes (`bn`). `Dn()` writes `--bg`, `--bg-secondary`, `--fg`, `--theme-accent`, `--selection`, `--loader` and the status colors. `css-C58pR_1e.css` sets `--color-window-bg: var(--bg-40)` over a Windows backdrop of `#333c` (`#333` with reduced transparency), which explains the measured `#1F1F1F`. `--accent` is `var(--color-system-accent, var(--theme-accent))`.
- **Accent.** On the dev machine `AccentPalette` entry 3 is `E2241A`, the switch color in the user's screenshot.
- **Unknowns.** Window size and position (`unk_d92d0ead…`); alias ranking (`unk_26be2170…`).

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

### Toasts, HUD and loader

Sources are under `Raycast\frontend\`. Class names are CSS-module hashes.

- **Routing (`toast-store-yMR5z_cG.js`).** `toast.open(t)` keeps the toast in the window when `window.identifier !== "main"`. In the main window it renders in the footer when `windowStateStore.isVisible && !isCompact`, and otherwise passes the same object to the HUD store (`hud-store-D8l7mVGu.js`, `open/close/get`).
- **Footer toast (`ActionBarToast` in `main-window-BN8deh-D.js`, CSS `action-bar-toast__*` in `main-window-38KZXJcp.css`).**
  - It replaces the footer's left element while a toast is set (`ActionBarHost.Root`).
  - It auto-closes 3000 ms after a non-pending toast appears, with a 200 ms opacity fade when the window is active. Hovering pauses the timer, and a pending toast stays until replaced.
  - It is a button with a 16 px fake-icon slot, the title and an optional subtitle. An activity indicator sits at `top/left: --space-6`.
  - Background: an absolutely positioned element at `top: 50·spx; left: 80·spx`, 10000 × 400 px, `translate(-50%,-50%) rotate(20deg)`, `opacity: .2`, `radial-gradient(farthest-side, <variant color>, transparent)`, `z-index: -1`. Variant colors: success `--green`, warning `--yellow`, failure `--red`, pending transparent.
- **Activity indicator (`activity-indicator-CxsLT95H.css`).** 16 px grid. It shows a 5·spx dot with `box-shadow: 0 0 12px 8px <color>-20` for success/warning/failure, and a rotating 25 % arc (0.8 s) for pending.
- **HUD (`Hud.*` and `Huds` in `window-shell-hosts-CeP7zO_K.js`, CSS `hud-primitive__*` in `window-shell-hosts-DX8dudFS.css`).**
  - It renders inside a native "portal" window (`yt` with `feature: "hud"`). `Raycast.dll` has `HUDWindowParameters` next to `PortalWindowParameters`, `TooltipWindowParameters`, `PopoverWindowParameters` and `InfoCardWindowParameters`, and the UTF-16 portal kinds `hud`, `alert`, `dialog`, `popover`, `info-card`.
  - It auto-closes after 1200 ms, or 3000 ms for failure. Pending stays.
  - Root: `inline-flex`, height 46·spx (56 with a message), padding 12, gap 12, start padding 16 with an icon (24 without), end padding 24, max-width 500, background `--bg-40`.
  - Variant background: `radial-gradient(45% 30% at 50% -10%, c, transparent), radial-gradient(100% 90% at 50% -15%, c, transparent)`, where c is the variant color at 26 %.
  - Icon 22 px. The activity indicator sits at the bottom-right (−6), and the icon is masked with a 6 px radial hole at (20, 20).
- **Loader (`NavigationBarUI.Root` in `command-list-item-accessory-xaxrXNsQ.js`, CSS in `command-list-item-accessory-FLHqb7-O.css`).**
  - Windows renders only `loaderClassic`; the glow loader is `null`.
  - It is a 1 px line at the bottom of the navigation bar: `linear-gradient(to right, loader-0 25%, loader, loader-0 75%)`, animated `translate(-35%) ↔ translate(35%)` over 1.5 s ease-in-out, infinite.
  - It fades in over 0.3 s, and only after `isLoading` has been true for `loadingDelayMs = 300`.
- **Theme (`contrast-improvement-CQ0EgiTy.js`, `bundled-raycast-dark`).** background `#000000`, foreground `#FFFFFF`, loader `#FFFFFF`, red `#FF6363`, orange `#FF9217`, yellow `#FFC531`, green `#59D499`, blue `#56C2FF`, purple `#A485FF`, accent `#4FA3F8`.
- **Measured from user screenshots (100 % scale).**
  - Footer toast: dot centered at (22, 454); title at x=37, 12 px regular (width and ink compared); glow tints the footer separator.
  - HUD: 232×46 with a 1 px border `#414141`, a base of `#1F1F1F`, the icon at x=16, the text at x=51 (13 px).
- **HUD placement is unknown.** One probe found no new top-level `Raycast.exe` window during a HUD (`unk_3a8f3266…`).

## Built-in features (milestone 4)

- **Clipboard History (`main-window-BN8deh-D.js`).**
  - Actions come from `qoe`: primary group Paste (`lN`, “Paste to <app>”), Copy to Clipboard (`uN`), Paste and Keep Window Open, Paste as…; secondary group Copy Color as… (`Coe`), Edit, Rename, Copy Formatted JSON, Pin/Unpin (`Aoe`/`Moe`), move pinned; export group (OCR, QR); delete group Delete Entry (`Foe`), Delete Recent Entries… (`Ioe`: Last 5/15/30 Minutes, Last Hour, Last 24 Hours), Delete All Entries (`Loe`).
  - The type filter (`vT` options) is All Types, Text Only, Images Only, Files Only, Links Only, Emails Only, Colors Only.
  - The Information panel shows Source, Type, Characters, Words, then Copied, or Times copied / Last copied / First copied.
  - The retention default is `P1W` (`historyDuration` in `internal-extensions-DZRANhqM.js` and `backend/index.mjs`); options 1 Day to 3 Months, longer ones Pro.
- **Shortcuts on Windows (`shortcuts-C1kqjDmf.js`).** primary Enter, secondary Ctrl+Enter, tertiary Ctrl+Shift+Enter, quaternary Ctrl+Alt+Enter; clear Ctrl+D, clearSecondary Ctrl+Shift+D, clearTertiary Ctrl+Alt+D; copy Ctrl+Shift+C, copySecondary Ctrl+Alt+C; pin Ctrl+.; edit Ctrl+E; new Ctrl+N; open Ctrl+O, openSecondary Ctrl+Shift+O.
- **Calculator.** Copy Answer (`vk`, primary + copy) and Paste Answer (`yk`, secondary). Color formats come from the backend `color.getFormatted` (`J1r` ids, `q1r` labels, `SX` = 3 significant digits). Settings: syntax highlighting, color conversion, flight tracker, weather, rem-to-px rate.
- **Emoji picker.** Categories `mI`: Pinned, Frequently Used, Smileys & People, Animals & Nature, Food & Drink, Activity, Travel & Places, Objects, Symbols, Flags, Other Symbols, AI Results. Settings: column count, primary action (paste/copy), skin tone.
- **Quicklinks.** Form title Create/Update/Duplicate Quicklink, primary “Save Quicklink” on secondary (Ctrl+Enter); search placeholder “Search for Quicklinks…”. Settings: link previews, prefer existing tabs, pass selected text as argument, delete warning.
- **Live check (2026-10-18).** In the debug build, PowerShell `Set-Clipboard` produced entries with source “Windows PowerShell”. Reading the clipboard immediately on `WM_CLIPBOARDUPDATE` made `Set-Clipboard` fail and one copy raised several notifications, so capture waits 100 ms and repeats within 1 s are one copy.

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
| `unk_3a8f3266…` | Screen placement of the HUD portal window | low |
| `unk_d92d0ead…` | Settings window default size and position (switch color resolved: system accent) | low |
| `unk_26be2170…` | Alias matching and rank in root search | low |
| `unk_f8b3baf9…` | Calculator card labels for plain math | low |
| `unk_b6c2e6e4…` | Default fallback commands and order | low |
| `unk_66397645…` | Selected emoji cell tint | low |

Not yet investigated:

- Exact row text metrics.
- Show and hide animation.
- Compact mode height.
