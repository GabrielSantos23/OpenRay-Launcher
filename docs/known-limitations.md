# Known limitations

## Reference version

- The task brief named Raycast 2.4.0.0, but the machine has **2.8.2.0**. Every finding comes from 2.8.2.0, and `AGENTS.md` now says so.

## Global hotkey

- **Mechanism.** Openray uses a WH_KEYBOARD_LL hook (`src-tauri/src/hotkey.rs`), like the reference.
  - The hook only decides and returns; a dispatcher thread does the work. Doing work inside the hook made Windows drop it after the first press.
  - For Alt and Win chords, an unassigned key (0xE8) is injected so that releasing the modifier does not open the menu bar or Start menu.
- **Shared hotkey.** If another running app hooks the same chord (Raycast, RaycastMini on Alt+Space), the most recently installed hook wins. Raycast appears to re-hook, so the two alternate. Use a different hotkey or quit the other launcher. On the dev machine, Raycast was moved to Ctrl+Alt+Space so that Openray keeps Alt+Space.
- **Settings.** The hotkey is recorded in Settings › General and applies at once. The recorder pauses the hook while it listens. Chords that Windows keeps for itself (Win+L, Ctrl+Alt+Del, some Win+letter shortcuts) never reach the page and cannot be recorded.
- **Startup.** If any saved hotkey in `settings.json` is invalid at startup, none are registered and the tray tooltip says so. The Settings window refuses to save an invalid or duplicate hotkey.

## Settings window

- **Scope.** Only settings that change real behavior are shown: General (Open at Login, Show in System Tray, hotkey), Launcher (Show Openray on), Shortcuts (alias, hotkey, enable per command and extension) and Keyboard (Escape behavior, Escape closes window). Appearance, window mode, interface size, pop-to-root, sensitivity, search history, file search, fallbacks, navigation bindings, hyper key, input source, Advanced and About wait for the features behind them.
- **Open at Login.** It is off until the user turns it on (the reference defaults to on). It points at the running exe, so a debug build registers the debug path.
- **Controls.** Dropdowns and the hotkey recorder are rebuilt from the reference CSS; widths, alignment, blur and open animations are estimates. Sidebar icons are Lucide glyphs on tiles, not Raycast assets. Back and forward only move between pages visited in this session.
- **Size.** The window opens at 1370×836 ≈ (taken from one screenshot) and its size is not remembered.
- **Themes.** The eight bundled themes are available. Theme Studio, custom themes and the reference's contrast-adjusted status colors are not. The native window background is fixed at `#1F1F1F`, so a light theme can flash dark for a frame when a window first paints.
- **Accent.** The accent is read once at startup from `AccentPalette`. Changing it in Windows needs a restart of Openray.
- **Scrollbars.** They take a 10 px gutter instead of overlaying the content, do not widen on hover, and show while hovered instead of while scrolling.
- **Hotkey recorder.** Only modifier + key chords. Single keys, double taps and per-side modifiers are not supported.
- **Command hotkeys.** A hotkey runs the command's primary action through the launcher webview. That webview must have loaded its commands, which happens shortly after startup.

## Applications

- **Sources.** Start Menu shortcuts (`.lnk`, `.url`, `.appref-ms` under `%ProgramData%` and `%APPDATA%` Start Menu\Programs) and packaged apps from the shell AppsFolder are indexed.
  - Game launchers and Control Panel items are missing.
  - Packaged apps launch through `shell:AppsFolder`, not `IApplicationActivationManager` as in the reference.
- **Uninstaller filter.** Shortcuts whose name starts with "Uninstall" are skipped. The reference's own filter rule is unconfirmed.
- **Icons.** Icons come from the shell at 22 px × device pixel ratio. They are not cached on disk, so each one is extracted again after a restart. A letter tile shows when extraction fails.
- **Rescans.** The index is re-scanned every time the launcher hides (about 0.4 s in a debug build, off the UI thread). There is no file watcher and no cache.

## Built-in features

- **Calculator.** Math, currency, time zones and colors only. No unit conversions, dates, history or syntax highlighting. Exchange rates come from open.er-api.com (free, no key, daily updates), cached for 6 hours; without network the last cached rates are used, and with none the currency card does not appear. The math card labels are unverified.
- **Clipboard History.** Text, files and images are kept 7 days (pinned entries forever), the reference default. There is no retention setting, OCR, disabled-apps list, Paste and Keep Window Open, Paste as…, rename or tags. Delete All keeps pinned entries. Two copies of the same content within a second count as one, because one copy can raise several clipboard notifications.
- **Paste.** Paste actions restore focus to the previous window and send Ctrl+V. Apps that run elevated ignore the injected keys.
- **Emoji.** Names come from Unicode only, so searches match fewer words than the reference. No skin tones or pinned emoji.
- **Quicklinks.** No icon picker, tags or `{selection}`; only the first argument gets a field. Favicons are fetched from Google's favicon service, and offline a link glyph shows.

## Launcher

- **Position.** The default position rule is approximate: centered horizontally, with the top at 35.3% of the work area (`unk_341f…`). There is no drag-to-move or remembered position.
- **Shown ex-style.** It keeps `WS_EX_TOOLWINDOW` (so it never appears in Alt+Tab) and Tauri's `WS_EX_ACCEPTFILES`. The reference drops TOOLWINDOW while shown.
- **Show and hide.** Show latency was measured only on a debug build at 96 DPI.
- **Missing UI.** There is no compact mode. Fallbacks are a fixed list (web searches and quicklinks with an argument) without a settings page. Arguments have a field only in Search Quicklinks, not inline in the root search.
- **Leaving a command.** Escape or Backspace on an empty search returns to the root search, which is cleared.
- **Action panel.** Configure Command, Configure Extension, Treat as Application and Hide/Quit/Force Quit are not offered. The panel has no noise texture, and its radius and shadow are estimates.
- **App actions.** Show in File Explorer selects the Start Menu shortcut, not its target; the reference behavior is unverified. Packaged apps have no Run as Administrator, Show in File Explorer or Copy Path.
- **Theme.** Only the dark theme exists, and only at the measured 100% DPI.
- **Escape.** Escape always hides the launcher. The reference has an `escapeKeyBehavior` setting whose default is unknown.
- **Search input focus.** The search input has no focus ring. This matches the reference, and the caret stays visible.

## Toasts and HUD

- **HUD position.** The HUD position is an approximation (bottom-center, 12 % above the work-area bottom), because the reference rule is unknown (`unk_3a8f3266…`).
- **No HUD trigger yet.** Every current action that toasts keeps the launcher open, so only the footer toast shows. The HUD appears for any toast raised while the launcher is hidden.
- **Footer toast.** There is no toast action menu (Ctrl+T) and no dismiss button.
- **Loader.** The loader only covers command refreshes and running actions. Most refreshes finish before the launcher is shown.

## Search and ranking

- **Matching.** The tiers are prefix > word-start > acronym > substring > subsequence. This is our own heuristic, because the reference algorithm is unknown (`unk_93ec…`).
- **Frecency.** It uses the reference's decay (10-day half-life, floor 1). The +1-per-visit increment is an approximation (`unk_f0ba…`).
- **Suggestions.** Up to 8 entries are shown. The cap is ≈, not measured.

## Platform

- Built and tested only on Windows 11 x64 at 96 DPI. Windows 10 is untested.
- Startup time and idle memory were not measured.
