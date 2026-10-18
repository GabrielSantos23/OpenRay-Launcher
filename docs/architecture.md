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

- **Global hotkey.** A WH_KEYBOARD_LL hook, matching the reference. `RegisterHotKey` was used in the first skeleton and replaced in milestone 1.
- **Third-party extension isolation is deferred.** Built-ins run in-process in the web layer.

## Module boundaries

| Module | Location | Owns |
|---|---|---|
| Lifecycle | `src-tauri/src/lib.rs` | Single instance, tray, startup hidden, quit |
| Launcher window | `src-tauri/src/launcher.rs`, `tauri.conf.json` | Popup HWND, transparent hide/show (alpha 0, click-through) with focus restore, placement, DWM styling, hide on blur |
| Global hotkey | `src-tauri/src/hotkey.rs` | Parse hotkeys, bindings list (launcher + command hotkeys, duplicates rejected), WH_KEYBOARD_LL hook thread, dispatcher thread, suspend while recording |
| Windows integration | `src-tauri/src/apps.rs`, `launcher.rs` | Start Menu scan, AppsFolder (MSIX) scan, shell icons served on `appicon://`, ShellExecute, DWM attributes |
| Settings and persistence | `src-tauri/src/store.rs`, `src-tauri/src/registry.rs` | `settings.json` (hotkey, tray, show-on screen, Escape options, per-command alias/hotkey/disabled), `frecency.json`, `favorites.json` in `%APPDATA%\dev.openray.app`; HKCU Run value for Open at Login; Windows accent from `AccentPalette` |
| Settings window | `src/SettingsWindow.tsx`, `src/settings.ts`, `src/Dropdown.tsx`, `src/HotkeyRecorder.tsx` | `settings` Tauri window: pages, hotkey recorder, commands table; `useSettings` keeps every window in sync through the `settings-changed` event |
| Search and ranking | `src/search.ts` | Matching, scoring, merging with frecency |
| Command registry | `src/registry.ts` | Collects items from registered extensions |
| Extension API | `src/extension-api.ts` | Types and `defineExtension` |
| Built-in extensions | `src/extensions/*` | `applications` (first), `openray-core` |
| Design system | `src/styles.css` (`@theme` tokens), `docs/design-system.md` | Measured tokens as CSS variables |
| Actions | `src/actions.ts`, `src/ActionPanel.tsx` | Action groups per command (primary, extension, favorites, ranking), shortcut matching, Ctrl+K panel |
| Toasts and HUD | `src/toast.ts`, `src/FooterToast.tsx`, `src/Hud.tsx`, `src-tauri/src/hud.rs` | `showToast` routes to the footer (launcher visible) or the `hud` window (hidden); HUD window styles, sizing and placement |
| Themes | `src/theme.ts` | Bundled theme data, `ThemeRoot` applies CSS variables in every window from settings, system appearance and accent |
| Launcher UI | `src/App.tsx` | Search input, list, footer, loader, keyboard |

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
- `hotkeys_suspend`, `login_item_get`, `login_item_set`, `open_settings`, `system_accent_color`

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
  run: () => Promise<void> | void;
}

interface Action {
  id: string;
  title: string;
  icon?: ComponentType<{ className?: string }>;
  shortcut?: { key: string; ctrl?: boolean; alt?: boolean; shift?: boolean };
  destructive?: boolean;
  keepOpen?: boolean;
  run: () => Promise<void> | void;
}
```

- `actions` are groups shown in the Ctrl+K panel. The first group follows the primary action. The launcher adds the favorites group after it and Reset Ranking at the end.
- An action hides the launcher after it runs, unless `keepOpen` is set.
- Actions report results with `showToast({ variant, title, message?, icon? })` from `src/toast.ts`. Failures thrown by `run` become failure toasts.

- Static `commands` declare fixed commands. `items()` supplies dynamic ones, such as one per installed app.
- The registry namespaces ids as `<extensionId>/<commandId>`. That key is what frecency stores.
- Built-ins are registered through `defineExtension` in `src/extensions/index.ts`, and nothing else is special-cased.
- Planned additions, when real features need them:
  - views (list/detail/form);
  - arguments;
  - preferences.
