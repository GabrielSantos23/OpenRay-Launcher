# Architecture

## ADR-001: Stack

- **Status:** Accepted, 2026-10-18
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

## Module boundaries

| Module | Location | Owns |
|---|---|---|
| Lifecycle | `src-tauri/src/lib.rs` | Single instance, tray, startup hidden, quit; `open_window` builds the Settings, Notes and tooltip windows from their `tauri.conf.json` entries (`"create": false`) on demand, and they are destroyed on close |
| Launcher window | `src-tauri/src/launcher.rs`, `tauri.conf.json` | Popup HWND, transparent hide/show (alpha 0, click-through) with focus restore, placement, DWM styling, hide on blur |
| Global hotkey | `src-tauri/src/hotkey.rs` | Parse hotkeys, bindings list (launcher + command hotkeys, duplicates rejected), helper process with the swallowing WH_KEYBOARD_LL hook (respawned if it exits), in-process snippet hook, suspend while recording |
| Windows integration | `src-tauri/src/apps.rs`, `launcher.rs` | Start Menu scan, AppsFolder (MSIX) scan, shell icons served on `appicon://`, ShellExecute, DWM attributes |
| Settings and persistence | `src-tauri/src/store.rs`, `src-tauri/src/registry.rs` | `settings.json` (hotkey, tray, show-on screen, Escape options, per-command alias/hotkey/disabled), `frecency.json`, `favorites.json`, and the extension data files `quicklinks.json`, `emoji.json` and `notes.json` (`data_get`/`data_set`, names allow-listed) in `%APPDATA%\dev.openray.app`; HKCU Run value for Open at Login; Windows accent from `AccentPalette` |
| Settings window | `src/SettingsWindow.tsx`, `src/settings.ts`, `src/Dropdown.tsx`, `src/HotkeyRecorder.tsx` | `settings` Tauri window: pages, hotkey recorder, commands table; `useSettings` keeps every window in sync through the `settings-changed` event |
| Clipboard | `src-tauri/src/clipboard.rs` | `WM_CLIPBOARDUPDATE` listener thread, capture (text, files, DIB→PNG), exclusion formats, history store with retention and pinning, paste sequence (offset, timeout, reset) and a short recording pause for its own writes, write-back (text, files, DIB + PNG), Ctrl+V injection, source app lookup; images served on `clipimg://` |
| Views | `src/ListView.tsx`, `src/ActionHost.tsx` | Pushed command views: back button, search with optional argument field and filter, list or grid, detail with Information, footer; action panel with submenus and confirmation dialogs, shared with the root |
| Notes | `src/NotesWindow.tsx`, `src/notes.ts`, `src/Tooltip.tsx`, `src-tauri/src/notes.rs`, `src/extensions/notes.tsx` | `notes` window (Tiptap editor with the markdown extension, format bar, action panel, browse list, history), `notes.json` through `data_get`/`data_set`, right-edge placement on first show, `tooltip` window (non-activating, click-through) positioned above the hovered button in screen coordinates; launcher commands open it through `notes_show` |
| Calculator | `src/calculator.ts`, `src/calculatorHistory.ts`, `src/extensions/calculator.ts`, `src/extensions/calculator-history.tsx` | Expression parser, currency and time-zone parsing, color parsing and formats; inline result card; history in `calculator-history.json` (upsert, retention, pins, date sections) and the Calculator History view |
| Translator | `src/translate.ts`, `src/extensions/translate.tsx`, `src-tauri/src/translate.rs` | Language list and swap/restore logic; two-pane view; HTTP call to the Google `gtx` endpoint and response parsing in Rust (no CSP change needed) |
| Search and ranking | `src/search.ts` | Matching, scoring, merging with frecency |
| Command registry | `src/registry.ts` | Collects items from registered extensions |
| Extension API | `src/extension-api.ts` | Types and `defineExtension` |
| Icons | `src/icons.tsx` | `GlyphTile` (extension tiles from an `IconSpec`), `CommandIcon` (URL, tile or fallback), `OpenrayGlyph` |
| Command arguments | `src/extension-api.ts`, `src/App.tsx` | A command's `argument` adds a field next to the root search text while it is selected; Tab moves between them and the primary action runs `argument.run(value)` (Set Volume); `optional: true` lets it run empty (run commands, script arguments) |
| Empty list actions | `src/ListView.tsx` | `emptyActions` gives an empty view its actions and Ctrl+K panel (Create Run Command, Create Quicklink, Add Script Directory) |
| Fallback views | `src/extension-api.ts`, `src/App.tsx` | A command's `fallbackView(text)` returns a view that opens from the “Use … with...” section with the typed text (Create Quicklink) |
| Hotkeys without modifiers | `src-tauri/src/hotkey.rs`, `src-tauri/src/store.rs`, `src/settings.ts`, `src/HotkeyRecorder.tsx` | Print Screen, Pause, Scroll Lock and F13–F24 may be bound alone; the recorder also reads Print Screen on keyup because Windows does not send its keydown to the page; `add_default_hotkeys` fills in Print Screen defaults for screenshot commands that have no saved preferences and whose hotkey is free |
| Performance | `src-tauri/src/cache.rs`, `src-tauri/src/launcher.rs`, `src/flowkey/FlowKeyIcon.tsx`, `vite.config.ts`, `tauri.conf.json` | Stale-while-revalidate caches for apps (20 s), Settings pages (10 min) and Steam games (1 min), warmed at startup; WebView2 memory target Low while the launcher is hidden; browser args `--disable-gpu --process-per-site` (also on runtime pin windows); extension icons by name load one Lucide module on demand from a lazy loader, with names from the `virtual:lucide-names` module, so the launcher no longer bundles all icons |
| Built-in extensions | `src/extensions/*` | `applications` (first), `calculator`, `clipboard-history`, `emoji-picker`, `notes`, `quicklinks`, `translator`, `web-searches`, `windows-run`, `urls`, `script-commands`, `screenshots`, `menu-items`, `openray-core` |
| Design system | `src/styles.css` (`@theme` tokens), `docs/design-system.md` | Measured tokens as CSS variables |
| Actions | `src/actions.ts`, `src/ActionPanel.tsx` | Action groups per command (primary, extension, favorites, ranking), shortcut matching, Ctrl+K panel |
| Toasts and HUD | `src/toast.ts`, `src/FooterToast.tsx`, `src/Hud.tsx`, `src-tauri/src/hud.rs` | `showToast` routes to the footer (launcher visible) or the `hud` window (hidden); HUD window styles, sizing and placement |
| Themes | `src/theme.ts` | Bundled theme data, `ThemeRoot` applies CSS variables in every window from settings, system appearance and accent |
| Focus | `src/focus.ts`, `src/extensions/focus.tsx`, `src/FocusWindows.tsx`, `src-tauri/src/focus.rs` | Session state and timers, app and website rules, foreground watcher with UI Automation URL reading, block overlay, notification and panel windows, native panel menus; categories and preferences in `focus.json` |
| Notch | `src/notch.ts`, `src/NotchWindow.tsx`, `src-tauri/src/notch.rs`, notch bridge in `src/extensions/flowkey.tsx` | `notch` window shape and placement, hover, volume and fullscreen watcher thread, media artwork and battery commands; the launcher renders FlowKey `notch: true` commands and forwards their trees to the notch, which sends actions back |
| Snippets | `src/snippets.ts`, `src/extensions/snippets.tsx`, `src-tauri/src/snippets.rs` | Snippets in `snippets.json`; placeholder resolution in TypeScript; keyword tracking inside the low-level keyboard hook, erasing the keyword and asking the launcher (`snippet-expand`) to resolve it, then typing it with `snippet_type` |
| System pages | `src/extensions/system-pages.ts`, `src-tauri/src/settings_pages.rs`, `apps::control_panel_items` | Settings search cache parsing, PageID → ms-settings table, All Control Panel Items enumeration |
| Games | `src/extensions/games.tsx`, `src-tauri/src/games.rs` | Steam library and manifest parsing, cached icons, Game Launcher view and game items in root search |
| Uninstall | `src-tauri/src/uninstall.rs` | Uninstall registry lookup for Start Menu apps, `Remove-AppxPackage` for Store apps, Settings fallback; actions may build their confirmation asynchronously (`Action.confirm` as a function) |
| Toggles | `src-tauri/src/toggles.rs` | Night Light (CloudStore blob), HDR (Display Config advanced color), camera consent switch |
| App volume | `src/extensions/app-volume.tsx`, `src-tauri/src/app_volume.rs` | Audio sessions of the default output device grouped by executable, per-app volume and mute, active-app commands |
| Sound devices | `src/extensions/sound-devices.tsx`, `src-tauri/src/sound_devices.rs` | Core Audio endpoint list with names and form factors, default switching through `IPolicyConfig`, endpoint volume and mute |
| Switch Windows | `src/extensions/navigation.tsx`, `src-tauri/src/window_switcher.rs` | Lists the open app windows (shared filter with Quit All Apps in `system_actions.rs`), resolving UWP frames to the hosted app, and switches, minimizes, closes, maximizes or quits them |
| File Search | `src/fileSearch.ts`, `src/extensions/file-search.tsx`, `src-tauri/src/file_search.rs` | In-memory name index of the user folder, ranked search with type filter, Recently Used from the Windows Recent folder, file metadata; thumbnails through `appicon` with `thumb=1` |
| Run | `src/run.ts`, `src/extensions/run.tsx`, `src-tauri/src/run.rs` | Windows Run catalog (PATH, App Paths, `.cpl`, `.msc`, environment variables), path completion, RunMRU history read/write/delete, ShellExecuteEx with `runas` or `cmd /k`; run commands in `run-commands.json` |
| URL | `src/urls.ts`, `src/extensions/urls.ts` | Link detection; a command's `query` (`matches`, `run`) puts it first in Results while the search text matches |
| Script Commands | `src/scriptCommands.ts`, `src/extensions/script-commands.tsx`, `src-tauri/src/scripts.rs` | `@raycast.*` metadata parsing, interpreter by extension, hidden runs with captured output; folders in `script-commands.json`; templates in TypeScript |
| Screenshots | `src/capture.ts`, `src/screenshots.ts`, `src/CaptureOverlay.tsx`, `src/PinWindow.tsx`, `src/extensions/screenshots.tsx`, `src-tauri/src/capture.rs`, `src-tauri/src/screenshot.rs` | GDI capture of the virtual screen, window and monitor rectangles, PNG/BMP encoding, OCR (Windows.Media.Ocr), QR (`rqrr`); capture session state, the `capture` overlay window (shown with alpha 0 until the frozen frame loads, because a hidden WebView2 does not load it), `pin-*` windows built at runtime, and the `capture://` protocol (`frame`, `pin/<label>`) |
| Quick Look | `src/extensions/file-search.tsx`, `src-tauri/src/explorer.rs` | File Explorer selection through IShellWindows / IFolderView, shown with the Search Files info panel |
| Menu Items | `src/extensions/menu-items.tsx`, `src-tauri/src/menu_items.rs` | Win32 menu bar walk of the previous foreground window, `WM_COMMAND` to run an item |
| System Actions | `src/extensions/system-actions.ts`, `src-tauri/src/system_actions.rs` | Power, session, app quitting, Core Audio volume and microphone mute, media keys, dark mode and hidden-files registry toggles |
| Window Management | `src/extensions/window-management.ts`, `src-tauri/src/window_layout.rs` | Layout commands; target window lookup, work-area fractions and relative frames (sizes, edge moves, grow/shrink), invisible-border compensation, display order and moves, Restore from an in-memory frame per window, minimize, always on top and app-window cycling (these update the launcher's previous window so focus returns to the right place) |
| Launcher UI | `src/App.tsx` | Search input, list, footer, loader, keyboard |
| Updater | `src/updater.ts`, `tauri-plugin-updater`, `.github/workflows/release.yml` | Periodic and manual checks against `latest.json` on the latest GitHub release (public key in `tauri.conf.json`), the `update-found` event between windows, install through the plugin; the workflow builds and signs releases with `tauri-action` |
| FlowKey host (web) | `src/flowkey/*`, `src/extensions/flowkey.tsx` | Extension list and consent (`extensions.ts`), capability gate (`gate.ts`), native dispatcher (`natives.ts`), per-extension worker runtime (`runtime.ts`, `worker.ts`, `protocol.ts`), tree renderer (`FlowKeyView.tsx`, `Markdown.tsx`, `FlowKeyIcon.tsx`), web commands (`WebCommand.tsx`), dialogs, Settings panel |
| FlowKey host (native) | `src-tauri/src/flowkey.rs` | `flowkey://` and `extimg://` protocols, package install and uninstall, per-extension data files, DPAPI secrets and OAuth tokens, HTTP with host and private-address checks, Spotify PKCE, image cache, SMTC media, scoped file operations, file pickers, rich clipboard |
| Bundled FlowKey extensions | `extensions/spotify-player` | A FlowKey CLI project; `npm run build:extensions` runs `flowkey build`, and `dist/` is bundled as `resources/extensions/spotify-player` |

The Rust core exposes Tauri commands:

- `list_apps`
- `launch_app` (optionally as administrator)
- `show_in_explorer`
- `frecency_visit`
- `frecency_all`
- `frecency_reset`
- `favorites_get`
- `favorites_set`
- `hide_launcher`
- `show_hud`, `hud_present`, `hud_hide`
- `quit_app`
- `settings_get`, `settings_set` (validates hotkeys, saves, re-registers, emits `settings-changed`)
- `notes_show` (hides the launcher, creates or shows and focuses the notes window, emits `notes-open` to an existing window or keeps the request for `notes_pending` when it creates one), `notes_pending`, `tooltip_show`, `tooltip_present`, `tooltip_hide`
- `translate` (text, source, target → translated text and detected source language)
- Focus: `focus_get`, `focus_start`, `focus_update`, `focus_pause`, `focus_resume`, `focus_restart`, `focus_complete`, `focus_cancel`, `focus_snooze`, `focus_close_tab`, `focus_block_info`, `focus_panel_menu`, `focus_panel_reset`, `focus_toast_hide`, `read_json_file`
- Snippets: `snippet_type` (text, cursor offset → types it into the foreground app); `data_set` with `snippets` also reloads the expansion keywords
- Clipboard History: `clipboard_paste_next` (→ false when nothing is left to paste), `clipboard_reset_sequence`
- System pages: `list_system_pages` (→ Settings pages and Control Panel items with name, path, keywords, kind)
- Games: `list_games` (→ Steam games with install dir, icon, launch and uninstall links, plus launchers); `list_apps` hides Steam `.url` shortcuts for detected games
- Uninstall: `app_uninstaller` (name, path → registered uninstaller name, "" for a Store package, or null), `app_uninstall` (→ removed | requested | settings)
- Toggles: `system_toggle` (toggle-night-light | toggle-hdr | toggle-camera-mute → HUD text)
- App volume: `app_volumes` (→ id, name, exe path, volume, mute, playing), `app_volume_set` (id + volume/muted), `app_volume_active` (mute | up | down → HUD text)
- Sound devices: `sound_devices` (output | input → id, name, state, form factor, default, volume, mute), `sound_device_set` (id + default/volume/muted)
- Switch Windows: `window_list` (→ id, title, app name, exe path, minimized, maximized, in Z-order), `window_action` (id, `switch` | `minimize` | `close` | `toggle-maximize` | `quit` | `force-quit`)
- File Search: `file_search` (query, type → ranked hits or Recently Used, index state), `file_reindex`, `file_info` (path → shell type name, size and dates); emits `file-index-updated` when an index build finishes
- File Search: `file_index_status` (→ files counted so far while an index build runs, else null); Quick Look: `explorer_selection` (→ selected paths of the front File Explorer window)
- Run: `run_catalog`, `run_complete` (text → path entries), `run_history`, `run_forget`, `run_split`, `run_execute` (command, arguments, admin, shell; hides the launcher)
- Script Commands: `scripts_list` (directories → scripts), `script_run` (path, arguments → stdout, stderr, exit code); `open_with` (Windows “Open with” dialog)
- Screenshots: `screenshots_set_folder` (absolute path or null → saved in `screenshots.json`, kept in `capture::CUSTOM_FOLDER`); the `capture://` origin is listed in the CSP `img-src` (release builds enforce the CSP, dev builds do not); the overlay window is created at startup and shown/hidden through Tauri so its WebView keeps painting; `capture_start` (region | color | ocr | qr | pin | ruler), `capture_pending`, `capture_ready`, `capture_pixel` (x, y → hex), `capture_finish` (rect), `capture_cancel`, `capture_direct` (fullscreen | window | monitor | last-region), `pin_file`, `pin_shown`, `pin_copy`, `pin_close`, `screenshots_list`, `screenshots_folder`, `screenshot_paste_latest`, `image_copy`, `image_paste`, `image_edit`, `image_text` (PNG path → OCR text); events `capture-start`, `screenshot-saved`
- Menu Items: `menu_items_list` (→ app name and items with id, path, shortcut, checked), `menu_items_invoke` (id)
- Data files `run-commands` and `script-commands` added to `data_get`/`data_set`
- `system_action` (action id → runs one system action)
- `window_layout` (layout id → moves, resizes, minimizes, pins or cycles the target window; `minimize-on-display` uses the launcher's monitor)
- `hotkeys_suspend`, `login_item_get`, `login_item_set`, `open_settings`, `system_accent_color`

- FlowKey host: `flowkey_list`, `flowkey_extensions_dir`, `flowkey_package_manifest`, `flowkey_install`, `flowkey_uninstall`, `flowkey_data_get`, `flowkey_data_set`, `flowkey_secret`, `flowkey_http`, `flowkey_oauth`, `flowkey_image`, `flowkey_media_current`, `flowkey_media_control`, `flowkey_fs`, `flowkey_default_app`, `flowkey_pick_path`, `flowkey_selected_text`, `flowkey_edit_clipboard_entry`, `copy_rich`, `read_clipboard_rich`

The hook dispatcher toggles the launcher for the `launcher` binding and emits `run-command` (the command key) to the launcher window for command hotkeys.

## Extension API (internal, v0)

```ts
interface Extension {
  id: string;
  title: string;
  commands?: Command[];
  items?: () => Promise<Command[]>;
}

interface Command {
  id: string;
  title: string;
  subtitle?: string;
  icon?: string;
  keywords?: string[];
  kind: string;
  primaryActionTitle: string;
  actions?: Action[][];
  view?: ComponentType<{ pop: () => void }>;
  fallback?: (text: string) => Promise<void> | void;
  run?: () => Promise<void> | void;
}

interface Extension {
  icon?: IconSpec;
  inline?: (query: string) => Promise<InlineResult | null>;
}

interface IconSpec {
  glyph: ComponentType<{ className?: string; strokeWidth?: number }>;
  tone: "blue" | "orange" | "red" | "yellow" | "purple" | "green" | "slate";
}

interface Action {
  id: string;
  title: string;
  icon?: ComponentType<{ className?: string }>;
  shortcut?: { key: string; ctrl?: boolean; alt?: boolean; shift?: boolean };
  destructive?: boolean;
  keepOpen?: boolean;
  accessory?: string;
  confirm?: { title: string; message: string; confirmTitle: string };
  submenu?: () => Action[] | Promise<Action[]>;
  run: () => Promise<void> | void;
}
```

- A command with `view` replaces the root with its component (built from `ListView`); `pop` returns to the root. Command hotkeys for view commands show the launcher first.
- `fallback` commands appear under “Use “text” with...” and receive the root text.
- `inline` results (the calculator) render as a card above the root results; their actions drive the footer.
- An action with `submenu` opens its items in the action panel; `confirm` asks first, with Cancel focused.

- `actions` are groups shown in the Ctrl+K panel. The first group follows the primary action. The launcher adds the favorites group after it and Reset Ranking at the end.
- An action hides the launcher after it runs, unless `keepOpen` is set.
- Actions report results with `showToast({ variant, title, message?, icon? })` from `src/toast.ts`. Failures thrown by `run` become failure toasts.

- Static `commands` declare fixed commands. `items()` supplies dynamic ones, such as one per installed app.
- The registry namespaces ids as `<extensionId>/<commandId>`. That key is what frecency stores.
- Built-ins are registered through `defineExtension` in `src/extensions/index.ts`, and nothing else is special-cased.
- Planned additions, when real features need them: preferences.

## FlowKey extension host

Openray runs extensions built with the user's FlowKey CLI unchanged. This does not reopen ADR-001: built-in features still use the internal API above, and each FlowKey extension is adapted into it as one Openray extension keyed `flowkey.<id>`.

- **Isolation.** Each extension runs in its own module Web Worker. Workers cannot reach Tauri IPC (the invoke key lives only in the window), and `fetch`, XHR, WebSocket and similar globals are removed before the bundle loads. The worker starts from a blob so it inherits the window CSP, which limits script loads to the app and `flowkey://`.
- **Native calls.** The worker posts `nativeCall`; the window checks the method against manifest ∩ stored consent (`gate.ts`) and runs it (`natives.ts`), mostly through `flowkey.rs`. Calls are attributed to the worker's own extension, never to an id inside the message.
- **Trees.** React extensions render through `@flowkey-cli/react-ui`'s reconciler in the worker; trees are pushed at most every 16 ms and deduplicated. The window renders them with the existing ListView, footer and action panel.
- **Web commands** run in a sandboxed iframe served from `flowkey://<source>/<id>/__host__/<entry>` with a strict CSP. A shim maps `chrome.webview` to `postMessage`, and the host recognises the frame by its window, not by message fields.
- **Data** lives in `%APPDATA%\dev.openray.app\flowkey\<id>\`: `storage`, `cache`, `consent` and `preferences` as JSON, and DPAPI-sealed `secrets.bin` and `oauth-<provider>.bin`. OAuth tokens never reach the extension; `http.fetch` with `auth` injects them only for the provider's pinned host.
