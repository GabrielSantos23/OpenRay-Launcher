# Known limitations

## Reference version

- `AGENTS.md` names Raycast 2.4.0.0, but the machine has **2.8.2.0**, and every finding comes from 2.8.2.0.

## Global hotkey

- **Mechanism.** Openray uses `RegisterHotKey` through `tauri-plugin-global-shortcut`. The reference uses a WH_KEYBOARD_LL hook.
  - While Raycast or RaycastMini runs, their hooks take **Alt+Space** first, so Openray's default hotkey never fires.
  - To test next to them, change `hotkey` in `%APPDATA%\dev.openray.app\settings.json`.
- **Unavailable hotkey.** If registration fails, the app still runs and shows the failure in the tray tooltip. There is no settings UI yet.
- **Changes need a restart.** Hotkey changes apply after restarting the app.

## Applications

- **Sources.** Only Start Menu shortcuts are indexed (`.lnk`, `.url`, `.appref-ms` under `%ProgramData%` and `%APPDATA%` Start Menu\Programs).
  - Packaged MSIX/Store apps are missing, including Windows 11 Notepad, Calculator and Settings.
  - Game launchers and Control Panel items are also missing.
- **Uninstaller filter.** Shortcuts whose name starts with "Uninstall" are skipped. The reference's own filter rule is unconfirmed.
- **Icons.** There are no app icons yet. Rows show a letter tile.
- **Rescans.** The index is re-scanned every time the launcher opens. There is no file watcher and no cache.

## Launcher

- **Position.** The default position rule is approximate: centered horizontally, with the top at 35.3% of the work area (`unk_341f…`). There is no drag-to-move or remembered position.
- **Hiding.** Hiding uses `ShowWindow(SW_HIDE)`. The reference keeps the window mapped and makes it transparent and click-through.
- **Show and hide.** There is no show or hide animation, and show latency was not measured.
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
