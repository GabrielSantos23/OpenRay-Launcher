# Known limitations

## Reference version

- The task brief named Raycast 2.4.0.0, but the machine has **2.8.2.0**. Every finding comes from 2.8.2.0, and `AGENTS.md` now says so.

## Global hotkey

- **Mechanism.** Openray uses a WH_KEYBOARD_LL hook (`src-tauri/src/hotkey.rs`), like the reference.
  - The hook only decides and returns; a dispatcher thread does the work. Doing work inside the hook made Windows drop it after the first press.
  - For Alt and Win chords, an unassigned key (0xE8) is injected so that releasing the modifier does not open the menu bar or Start menu.
- **Shared hotkey.** If another running app hooks the same chord (Raycast, RaycastMini on Alt+Space), the most recently installed hook wins. Raycast appears to re-hook, so the two alternate. Use a different hotkey or quit the other launcher. On the dev machine, Raycast was moved to Ctrl+Alt+Space so that Openray keeps Alt+Space.
- **Settings.** The hotkey is set by `hotkey` in `%APPDATA%\dev.openray.app\settings.json`. An invalid value shows in the tray tooltip. There is no settings UI, and changes need a restart.

## Applications

- **Sources.** Start Menu shortcuts (`.lnk`, `.url`, `.appref-ms` under `%ProgramData%` and `%APPDATA%` Start Menu\Programs) and packaged apps from the shell AppsFolder are indexed.
  - Game launchers and Control Panel items are missing.
  - Packaged apps launch through `shell:AppsFolder`, not `IApplicationActivationManager` as in the reference.
- **Uninstaller filter.** Shortcuts whose name starts with "Uninstall" are skipped. The reference's own filter rule is unconfirmed.
- **Icons.** Icons come from the shell at 22 px × device pixel ratio. They are not cached on disk, so each one is extracted again after a restart. A letter tile shows when extraction fails.
- **Rescans.** The index is re-scanned every time the launcher hides (about 0.4 s in a debug build, off the UI thread). There is no file watcher and no cache.

## Launcher

- **Position.** The default position rule is approximate: centered horizontally, with the top at 35.3% of the work area (`unk_341f…`). There is no drag-to-move or remembered position.
- **Shown ex-style.** It keeps `WS_EX_TOOLWINDOW` (so it never appears in Alt+Tab) and Tauri's `WS_EX_ACCEPTFILES`. The reference drops TOOLWINDOW while shown.
- **Show and hide.** Show latency was measured only on a debug build at 96 DPI.
- **Missing UI.** There is no action panel (Ctrl+K), no Favorites section, no fallback commands and no compact mode.
- **Theme.** Only the dark theme exists, and only at the measured 100% DPI.
- **Escape.** Escape always hides the launcher. The reference has an `escapeKeyBehavior` setting whose default is unknown.
- **Search input focus.** The search input has no focus ring. This matches the reference, and the caret stays visible.

## Search and ranking

- **Matching.** The tiers are prefix > word-start > acronym > substring > subsequence. This is our own heuristic, because the reference algorithm is unknown (`unk_93ec…`).
- **Frecency.** It uses the reference's decay (10-day half-life, floor 1). The +1-per-visit increment is an approximation (`unk_f0ba…`).
- **Suggestions.** Up to 8 entries are shown. The cap is ≈, not measured.

## Platform

- Built and tested only on Windows 11 x64 at 96 DPI. Windows 10 is untested.
- Startup time and idle memory were not measured.
