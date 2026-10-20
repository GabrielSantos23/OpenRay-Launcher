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

- **Calculator.** Math, currency, time zones and colors only. No unit conversions, dates or syntax highlighting. Calculator History has no Show History or Pin Calculation action on the root card. Exchange rates come from open.er-api.com (free, no key, daily updates), cached for 6 hours; without network the last cached rates are used, and with none the currency card does not appear. The math card labels are unverified.
- **Clipboard History.** Text, files and images are kept 7 days (pinned entries forever), the reference default. There is no retention setting, OCR, disabled-apps list, Paste and Keep Window Open, Paste as…, rename or tags. Delete All keeps pinned entries. Two copies of the same content within a second count as one, because one copy can raise several clipboard notifications. Paste Sequentially uses the reference defaults (60 s timeout, newline after text) without settings, ignores copies made in the 2 s around each of its pastes, and puts the newest history item back on the clipboard afterwards.
- **Paste.** Paste actions restore focus to the previous window and send Ctrl+V. Apps that run elevated ignore the injected keys.
- **Emoji.** Names come from Unicode only, so searches match fewer words than the reference. No skin tones or pinned emoji.
- **Notes.** Deleting a note is permanent and asks first; there is no Recently Deleted. No window auto-sizing, Find in Note, Export…, Copy Deeplink, images, zoom or notes settings. Markdown typed as one block (for example pasted plain text) stays plain text until it is retyped, because the live rules run per keystroke. The window size (440×762) and the right-edge spawn rule are estimates.
- **Translator.** Translations come from Google's unofficial free endpoint (`translate.googleapis.com/translate_a/single?client=gtx`), not Raycast's Pro backend. It has no key and no published limits: it can rate-limit or change without notice, and text is sent to Google. Long text is sent as a form body, so there is no URL length limit, but no length cap is enforced. No dictation, text-to-speech, prefill from selected text, quick-translate results in root search, custom translate commands or translator settings. The text does not shrink to fit as in the reference. Source text is not kept between openings. The nav bar height (54 px) and the translation pane's left inset (38 px) are measured from one screenshot, not from the reference CSS.
- **Quicklinks.** No icon picker, tags or `{selection}`; only the first argument gets a field. Import ignores icons and tags, and Export picks a folder instead of a file name. Favicons are fetched from Google's favicon service, and offline a link glyph shows.

## Launcher

- **Position.** The default position rule is approximate: centered horizontally, with the top at 35.3% of the work area (`unk_341f…`). There is no drag-to-move or remembered position.
- **Shown ex-style.** It keeps `WS_EX_TOOLWINDOW` (so it never appears in Alt+Tab) and Tauri's `WS_EX_ACCEPTFILES`. The reference drops TOOLWINDOW while shown.
- **Show and hide.** Show latency was measured only on a debug build at 96 DPI.
- **Uninstall Application.** The uninstaller is found by matching the Start Menu shortcut against the Uninstall registry; a shortcut inside another program's folder resolves to that program, so read the confirmation before accepting.
- **Settings pages.** Only pages in Openray's PageID → ms-settings table appear; names and keywords come from Windows' Settings search cache, so a page Windows hasn't indexed yet is missing.
- **Games.** Only Steam games are detected; Epic, GOG, Riot, EA, Blizzard and Xbox are not. Game actions have no Run as Administrator or quit/restart.
- **Missing UI.** There is no compact mode. Fallbacks are a fixed list (web searches, quicklinks with an argument, and Create Quicklink) without a settings page. Arguments have a field only in Search Quicklinks, not inline in the root search.
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

## FlowKey extensions

- **Shared install folder.** Openray reads and installs into `%LOCALAPPDATA%\FlowKey.Shell\extensions`, as `flowkey dev` does, but keeps its own consent and data. Extensions installed by FlowKey ask for permission once in Openray, and uninstalling from Openray removes the folder for FlowKey too.
- **Not implemented:** `system.control` and `windows.list/focus/close` return `methodNotSupported`. Only the opt-in Ctrl+C fallback of `system.selectedText` works (no UI Automation). `image.fetch` does not downscale. The `mediaKeys` flag on detail views and `isDevelopment` are ignored.
- **Arguments** are typed in a header form after choosing the command, not inline in root search.
- **fs scopes** are checked on the path text; a symlink or junction inside a scope that points outside it is followed.
- **OAuth** supports only the `spotify` provider, like FlowKey. It needs the user's own Spotify app client ID (preference, `FLOWKEY_SPOTIFY_CLIENT_ID`, or `spotify-auth.json` in Openray's or FlowKey's data folder) with `http://127.0.0.1/callback` registered.
- **Release build.** The production CSP path (blob worker importing from `flowkey://`, iframe on `flowkey://`) was not checked in a release build; all live checks ran under `tauri dev`.
- **Bundle size.** The renderer imports Lucide's full icon map so extensions can name any icon; the main chunk grew to about 870 kB.

## Spotify Player

- Not exercised signed in: no Spotify client ID was configured for Openray on the dev machine, so every signed-in view and command follows the reference source but is unverified live.
- Start Radio calls `/recommendations`, which Spotify restricted for new apps in November 2024; it may fail with a 404 or 403 for user-created apps.
- The duplicate-check preference and quicklink creation from Add Playing Song to Playlist are left out, because FlowKey has no quicklink launch context.
- Current Track shows a HUD instead of updating the command subtitle.

## Focus

- A session lives in memory: it ends if Openray quits or the computer restarts, and nothing pauses it during sleep.
- Website blocking reads the browser's address bar with UI Automation, so it only works in the listed Chromium and Firefox browsers and was only tested in Edge.
- Blocked apps are asked to close, not force-quit; an app that shows a save prompt stays open behind the overlay.
- No Move to Taskbar, focus statistics, or typed durations.

## Snippets

- Expansion types the text with Unicode key events instead of pasting it, so very long snippets appear gradually, and apps that ignore injected Unicode input (some games, remote desktops) don't receive them.
- Only typed characters are tracked: a mouse click inside the same window doesn't reset what was typed, and keys composed with dead keys (´ + a) are not recorded, so keywords should use plain characters.
- Keywords never expand inside Openray's own windows.
- Not yet: {argument} placeholders, date `format`/`offset` options, {selection}, {browser-tab}, {calculator}, tags, the Snippets settings (expansion on/off, expand within words, disabled apps, response time), Move Up/Down for pinned snippets, Export Snippet for one snippet.

## File Search

- Searches file and folder names only. No content search, no `.gitignore`/`.rayignore` support, no Search Scopes or Ignore Patterns settings, no hidden-files toggle; the index covers only the user folder.
- The index lives in memory and is rebuilt by walking the folder (no file watcher, no NTFS scanner), so new files appear after the next refresh, at most a minute after opening the view.
- Ranking is Openray's own approximation; the reference ranking was not measured.
- Not yet: files in root search, fallback with typed text, Navigate Up/Down and folder sorting, Paste File, Open in Terminal, Show Properties, Rename, Duplicate, Copy To, Move To, Open With, Exclude From Index.
- Recently Used reads the Windows Recent folder; when Windows keeps no recent items, the list shows the newest modified files instead.

## Switch Windows

- Not yet: Minimize to Tray and Hide Application (they need a way to bring hidden windows back), and the Navigation fallback setting.
- Filtered results keep the window order instead of sorting by match score.
- Windows on other virtual desktops are not listed.
- Quit Application only asks the app's windows to close; Force Quit ends the process without saving.

## System Actions

- Not yet: Wi-Fi, Bluetooth and airplane-mode toggles and managers, Energy Saver (its Windows API is undocumented).
- The sound device pickers change the default device through `IPolicyConfig`, an undocumented Windows interface that a future Windows update could change; format, enhancements and enable/disable are not offered.
- Volume Mixer and the active-app commands only see audio sessions on the default output device.
- Command arguments: one per command, and a command hotkey for an argument command does nothing until the launcher is open with the command selected.
- Toggle Night Light edits an undocumented registry blob; on a Windows build with another layout it reports “Unknown Night Light format” and changes nothing. Toggle Camera Mute only flips the app-access privacy switch.
- Quit All Apps only asks windows to close; apps with unsaved work show their own prompts.

## Run

- Run in Shell always uses `cmd /k`; the reference lets you pick the shell.
- Running a command adds it to the Windows Run history (RunMRU), the same list Win+R shows.
- PATH executables and environment variables are read when Openray starts; changes made later appear after a restart.

## Script Commands

- Only the first `@raycast.argument` is asked for. Inline mode runs like compact (no refresh in the root list), and output appears when the script ends, not while it runs.
- Script folders are added from Search Script Commands (Add Script Directory) or when creating a script; there is no settings page to remove one.
- Interpreters (`python`, `node`, `bash`, …) must be on PATH.

## Screenshots

- Capture uses GDI, so DRM-protected video and some hardware-accelerated overlays come out black. The overlay freezes the screen first, so menus and tooltips open at that moment are included.
- With monitors at different scale factors the overlay uses one scale for the whole virtual screen; selections stay pixel-accurate but the hint and loupe may look larger or smaller on one monitor.
- Not yet: screen recording (video and GIF), scrolling capture, an annotation editor, uploads, capture delay and after-capture settings. Captures always save to `Pictures\Openray` and copy the image.
- Scan QR Code reads QR codes only. OCR needs the Windows OCR language pack for the text's language.
- Search Screenshots lists `Pictures\Openray` and `Pictures\Screenshots`; it does not search text inside images.

## Quick Look and Menu Items

- Quick Look shows the preview inside the launcher. With File Explorer tabs, the selection is read from the first tab of the front window.
- Search Menu Items reads only classic Win32 menu bars (msinfo32, Notepad++, older apps). WinUI, ribbon and browser menus are not read.

## Window Management

- Windows of apps running as administrator can't be moved while Openray runs without elevation; the command shows an error.
- Maximize fills the work area and doesn't use the native maximized state.
- The per-command settings (Almost Maximize percentage, Make Smaller/Larger step size, Move Center Window, display Wrap and Keep Aspect Ratio) are not exposed; the reference defaults are used.
- Restore remembers only the frame before the last layout command, and forgets it when Openray restarts.
- Not yet: Resize Window and Move Window (typed arguments), gaps, custom commands and layouts, Save Current Layout, virtual desktops.

## Updates

- Updates need a GitHub release built by `.github/workflows/release.yml` with the repository secret `TAURI_SIGNING_PRIVATE_KEY` (and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` when the key has one). Without a release, every check fails, the failure goes only to the console, and the launcher shows no row.
- The release workflow builds only the Windows x64 NSIS installer.
- Installing shows no download progress. The app exits while the passive NSIS installer runs.

## Windows on demand

- Settings and Notes are created each time they open, so they open slower than a window kept alive. Notes keeps its last position and size only until Openray quits.

## Platform

- Built and tested only on Windows 11 x64 at 96 DPI. Windows 10 is untested.
- Startup time and idle memory were not measured.
