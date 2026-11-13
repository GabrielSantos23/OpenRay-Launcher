# Architecture

## ADR-001: Stack

- **Status:** Accepted, 2026-10-18. Superseded by ADR-002 (2026-11-04); the Tauri app was removed on 2026-11-13 (last present at commit 2925cc8).
- **Decision:** **Tauri 2 (Rust core + WebView2 UI), with React + TypeScript + Tailwind CSS v4 in the web layer.**

### Context

Raycast for Windows 2.8.2.0 is built from these parts (see `reverse-engineering-notes.md`):

- a .NET 10 WPF native host;
- a Vite-built web UI rendered in **WebView2**, with one HTML entry per window;
- a Node.js backend with Rust N-API modules for indexing and data;
- Win32 for hotkeys (WH_KEYBOARD_LL), focus, DWM and clipboard.

Every pixel the user sees in the launcher is HTML/CSS in Chromium (WebView2). The native layer only owns the frameless HWND and the OS integration.

### Options

| Criterion | Tauri 2 (Rust + WebView2) | Electron | Native (WinUI 3 / WPF, C#) |
|---|---|---|---|
| UI closeness to reference | **Same engine** (WebView2 = Chromium). CSS tokens such as `round()`, `color-mix`, relative colors and Inter render identically. | Same engine (bundled Chromium). | Different renderer. Text, blur and spacing would have to be re-tuned by hand. Parity is hardest. |
| Launcher show/hide, blur, focus | One persistent HWND, which we control through the `windows` crate (DWM corners, border, topmost, `SetForegroundWindow`). | BrowserWindow. Native DWM tweaks need native addons. | Full control. |
| Hotkey, app index, files, clipboard, window management | Direct Win32 from Rust, no FFI layer. | Needs N-API addons in C++ or Rust for hooks and Shell APIs. | Direct P/Invoke. |
| Startup / idle memory (estimate) | Small binary (~10 MB) plus a shared system WebView2. | ~150 MB+ bundled Chromium plus Node. | Small (WinUI) to medium (.NET runtime). |
| Extension runtime and isolation | TS extensions in the web layer now. Isolation later through a separate WebView/iframe or a sidecar runtime. | Node is built in. Isolation through utility processes. | Would need an embedded JS engine or .NET plugins. |
| Packaging and maintenance | `tauri build` produces MSI/NSIS. Two languages (Rust + TS). | Mature tooling, large footprint. | MSIX/WinUI tooling, slowest UI iteration. |

### Decision and rationale

Tauri 2 matches the reference's actual rendering technology, WebView2, so we can reproduce the visuals with the same CSS techniques the reference uses.

The Rust core covers the native duties: the WPF host and the Rust N-API modules in the reference. We do not need a Node process or a .NET runtime, so footprint stays low and there is no IPC hop through Node.

Electron also matches the renderer. We rejected it because of its footprint, and because it needs native addons for all the Win32 work.

A native UI was rejected because parity would have to be rebuilt by hand in a different text and layout engine.

### Consequences

- **Global hotkey.** A WH_KEYBOARD_LL hook, matching the reference. `RegisterHotKey` was used in the first skeleton and replaced in milestone 1. The hook that swallows hotkeys runs in a helper process (`openray.exe --keyboard-hook`, stdin: bindings and suspend, stdout: fired ids), like the reference's `Raycast.UIAccess.exe`, because Windows skips an in-process low-level hook for about a second after the process takes the foreground. The main process keeps a non-swallowing hook for snippets.
- **Third-party extension isolation is deferred.** Built-ins run in-process in the web layer.

## ADR-002: Native UI with egui, no WebView

- **Status:** Accepted, 2026-11-04. Supersedes ADR-001. Implemented; the Tauri app and the web frontend were removed on 2026-11-13.
- **Decision:** **Rust + egui (eframe, glow renderer), no WebView2 and no TypeScript in the main app.** Third-party extensions stay TypeScript and run in an embedded JS engine (QuickJS through `rquickjs`, implemented in `app/src/flowkey_js.rs`), rendering through the existing FlowKey `UiTree` protocol drawn natively by egui.

### Context

The owner asked for a WebView-free app. Measured on the dev machine with the launcher shown: the Tauri build uses 178 MB private memory across the app and six `msedgewebview2.exe` processes; the egui release build uses 113 MB in one process plus the 2 MB hotkey helper, with zero CPU while idle.

### Consequences

- The new app lives in `app/` (crate `openray-native`, binary `openray`). The Windows backends that the Tauri app shared through `#[path]` now live in `app/src`.
- `app/` pins Rust 1.97 (`app/rust-toolchain.toml`) because egui 0.37 needs 1.95+.
- Renderer: glow. It measured 92-121 MB against 344 MB for eframe's default wgpu backend on the dev GPU (GTX 970).
- Visual parity is now hand-tuned: tokens from the former `src/styles.css` are ported to `app/src/theme.rs`, Inter is embedded as static TTF weights converted from the OFL package, and Lucide glyphs are embedded as SVG (`scripts/lucide_svg.py`).
- Notes uses an egui block editor (`app/src/notes`) instead of Tiptap and keeps `notes.json` in the same markdown format.
- Built-ins implement the Rust `Extension` trait (`app/src/extension.rs`).
- npm remains only to rebuild FlowKey extension bundles (`npm run build:extensions`) and the QuickJS host (`npm run build:host`); both outputs are committed, so `cargo build` alone produces a complete app.
- Gaps left by removing the Tauri app (Portuguese UI, auto-update, light theme, FlowKey web commands, installer) are tracked in `known-limitations.md`.

## Module boundaries

All code is in `app/src` (crate `openray-native`, binary `openray.exe`).

| Module | Location | Owns |
|---|---|---|
| Lifecycle | `main.rs` | Single instance (named mutex plus show event), tray menu (Open, Settings, Quit), `--show` and `--keyboard-hook` arguments, module list |
| Launcher | `root.rs`, `search.rs`, `launcher.rs` | Root search, sections, favorites, frecency, pushed views (list, grid, form, page, detail), dropdowns, action panel, confirm dialog, toast and HUD; popup window show/hide (alpha 0, click-through), placement, DWM styling, hide on blur; `extension::Async` requests from other threads |
| Settings window | `settings_ui.rs` | Second viewport: General, Launcher, Shortcuts, Keyboard, Extensions, per-extension pages with the commands table, hotkey recorder and conflicts, FlowKey install, uninstall, preferences and permissions |
| Extension API | `extension.rs`, `extensions/mod.rs` | `Extension`, `Command`, `View`, `Action`, `Host`, `Request`, `Async`; built-ins listed in `built_in()`, FlowKey extensions appended on every reload |
| Built-in features | `extensions/*.rs`, `notes/`, `focus_ui.rs`, `capture_ui.rs` | One file per extension; Notes editor and window; Focus panel, toast and block windows; screenshot overlay and pins |
| Global hotkey | `hotkey.rs` | Hotkey parsing, helper process with the swallowing WH_KEYBOARD_LL hook, in-process snippet hook, suspend while recording |
| Persistence | `store.rs`, `registry.rs` | `settings.json`, `frecency.json`, `favorites.json` and extension data files in `%APPDATA%\dev.openray.app`; HKCU Run value for Open at Login |
| Windows backends | `apps.rs`, `clipboard.rs`, `file_search.rs`, `explorer.rs`, `window_layout.rs`, `window_switcher.rs`, `menu_items.rs`, `system_actions.rs`, `toggles.rs`, `app_volume.rs`, `sound_devices.rs`, `settings_pages.rs`, `games.rs`, `capture.rs`, `focus.rs`, `run.rs`, `scripts.rs`, `snippets.rs`, `contacts.rs`, `calendar.rs`, `uninstall.rs` | App index and icons, clipboard listener and history, file index, window placement and switching, menu items, power/audio/media/toggles, Settings pages, Steam games, capture and OCR, Focus engine, Run, Script Commands, snippet expansion, Contacts and Calendar (WinRT), uninstallers |
| Rendering helpers | `theme.rs`, `icons.rs`, `emoji_render.rs`, `http.rs`, `icu.rs` | Colors and fonts, Lucide glyphs, image loaders (`appicon://`, `clipimg://`, `emoji://`, extension images), WIC decoding, HTTPS through `ureq` |
| FlowKey host | `flowkey_js.rs`, `flowkey_host.rs`, `flowkey_gate.rs`, `flowkey_core.rs`, `extensions/flowkey.rs`, `app/js/*`, `app/assets/flowkey-host.js` | See below |

## FlowKey extension host

Openray runs extensions built with the user's FlowKey CLI unchanged. Each FlowKey extension becomes one Openray extension keyed `flowkey.<id>`.

- **Runtime.** Each extension runs in its own QuickJS engine (`rquickjs`) on its own thread (`flowkey_js.rs`): 512 MB heap, a 10 s limit per JS turn, timers and messages served by a Rust loop. The engine loads `app/assets/flowkey-host.js`, built by `npm run build:host` from `app/js/worker.ts` (the FlowKey worker protocol with React and `@flowkey-cli/react-ui`) and `app/js/shims.js` (timers, AbortController, URL, TextEncoder, crypto, console).
- **Isolation.** The engine has no fetch, file, process or OS access. Dynamic import resolves only the extension's own entry file. Everything else is a `nativeCall` message.
- **Native calls.** `flowkey_host.rs` checks each method against manifest ∩ stored consent (`flowkey_gate.rs`) and runs it on a small tokio runtime, mostly through `flowkey_core.rs` (HTTP with host allowlist and private-address blocking, Spotify PKCE OAuth, DPAPI secrets, scoped file operations, media, image cache, install and uninstall). Calls are attributed to the worker's own extension, never to an id inside the message.
- **Trees.** React trees arrive as `tree` messages and are drawn by `extensions/flowkey.rs` with the launcher's list, grid, page and form views.
- **Data** lives in `%APPDATA%\dev.openray.app\flowkey\<id>\`: `storage`, `cache`, `consent` and `preferences` as JSON, and DPAPI-sealed `secrets.bin` and `oauth-<provider>.bin`. OAuth tokens never reach the extension; `http.fetch` with `auth` injects them only for the provider's pinned host. Images are cached in `%LOCALAPPDATA%\dev.openray.app\flowkey-images`.
- **Packages.** Installed extensions live in `%LOCALAPPDATA%\FlowKey.Shell\extensions` (or `FLOWKEY_EXTENSIONS_DIR`); bundled ones in `extensions\` next to `openray.exe` (debug builds read `extensions/*/dist` in the repo).
