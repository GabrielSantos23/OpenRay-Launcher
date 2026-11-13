# Openray

**An open-source take on Raycast, built from scratch for Windows 10 and 11.**

Openray is a keyboard-first launcher. Press **Alt+Space** and type to open apps, search files, do math, paste from your clipboard history, expand snippets, manage windows, block distractions and more.

It aims to look and behave like [Raycast for Windows](https://www.raycast.com/windows) as closely as possible. It is an independent reimplementation: it contains no Raycast code or assets, and every feature is rebuilt on Openray's own code and extension API.

> Openray is a portfolio project. It is not affiliated with, endorsed by, or connected to Raycast Technologies Ltd. "Raycast" is used only to describe what this project is modeled on.

---

## Contents

- [Features](#features)
- [What's not included](#whats-not-included)
- [Install](#install)
- [Usage](#usage)
- [Build from source](#build-from-source)
- [Project structure](#project-structure)
- [How it works](#how-it-works)
- [Extensions](#extensions)
- [Releases](#releases)
- [Documentation](#documentation)
- [Contributing](#contributing)
- [License](#license)

---

## Features

Each feature below works today. The full per-feature status, with evidence and remaining differences, is in [`docs/feature-matrix.md`](docs/feature-matrix.md).

### Launcher
- Global hotkey (Alt+Space by default, configurable) using a low-level keyboard hook, so it works the way Raycast's does.
- Fuzzy search across apps, commands and extensions, ranked by how often and how recently you use them.
- Favorites, aliases and per-command hotkeys.
- An action panel (**Ctrl+K**) with keyboard shortcuts for every action, as in Raycast.
- Inline results for the calculator and colors, right in the root search.
- Opens on the screen with the mouse, the active screen or the primary screen; hides when it loses focus.
- A dark theme matching the reference.

### Applications
- Indexes Start Menu shortcuts and Microsoft Store (MSIX) apps, with their real icons.
- Finds Windows Settings pages and Control Panel items in your Windows language (try “sound” or “power”).
- Run as Administrator, Show in File Explorer, Copy Path, and Uninstall Application (with a confirmation naming the uninstaller).

### Games
- Your installed Steam games show up in search with their icons, plus a Game Launcher view; open, show in File Explorer or uninstall through Steam.

### Run
- An improved Win+R: Open Run lists your Run history, programs on PATH, Control Panel applets, consoles, environment variables and path completions; run as administrator or in a shell.
- Execute Run Command straight from the search bar, and save run commands with aliases that show up in search.

### Script Commands
- Put scripts with `@raycast.*` metadata (PowerShell, Batch, Python, Node and more) in a folder and they become commands, with arguments and silent, compact or full output.
- Create Script Command writes a ready-to-edit template.

### Screenshots
- Built-in capture tools inspired by ShareX, no other app needed: capture a region (or click a window), the full screen, the active window or monitor, or repeat the last region. Each capture is saved to `Pictures\Openray` and copied to the clipboard.
- Pick a color from the screen with a magnifier, copy text from any part of the screen (OCR), scan QR codes, measure with a ruler, and pin a region on top of everything.
- Search Screenshots shows your captures and Windows screenshots in a grid: copy, paste, edit, pin, copy their text. Paste Latest Screenshot drops the newest one into the active app.

### More
- Typing a link offers Open in Browser as the first result.
- Quick Look previews the file selected in File Explorer; Search Menu Items runs menu commands of classic Windows apps.

### File Search
- Searches file and folder names in your user folder, ranked by match quality and how recently a file changed.
- Filter by type: Directory, File, Archive, Document, Image, Video, Audio, Text, Other.
- **Recently Used** files when the search is empty.
- Info panel with a thumbnail, location, type, size and dates.
- Actions: Open, Show in File Explorer, Copy File, Copy Name, Copy Path, Delete (to the Recycle Bin).

### Clipboard History
- Keeps text, images, files, links, emails and colors. Respects apps that ask not to be recorded, such as password managers.
- Pin entries, filter by type, delete recent entries, preview images and colors.
- Paste straight into the app you were using.
- **Paste Sequentially** pastes your recent copies one after another, newest first.

### Snippets
- Save text and give it a **keyword**: type the keyword in any app and Openray replaces it with the snippet.
- Dynamic placeholders: `{clipboard}`, `{date}`, `{time}`, `{datetime}`, `{day}`, `{uuid}`, `{cursor}` and `{snippet name="…"}`.
- Search, pin, edit, duplicate, import and export snippets as JSON.

### Focus
- Focus sessions with a goal and an optional duration.
- **Block** or **allow** lists of apps, websites and categories. Websites are detected in Chromium-based browsers and Firefox.
- A floating session panel with a progress bar, pause, complete and a drag handle.
- A blocking overlay with snooze, and a notification when a session ends.
- Built-in categories (Social, Messaging, Gaming, Streaming, News, Shopping), custom categories and JSON import.

### Window Management
- Halves, thirds, two-thirds, fourths, three-fourths, quarters and sixths.
- Maximize, almost maximize, reasonable size, maximize height or width, center, restore and minimize.
- Move to an edge, make larger or smaller, toggle always on top, and cycle through an app's windows.
- Move a window between monitors.
- Accounts for invisible window borders, so windows line up exactly.

### Switch Windows
- Search every open window by title or app name and jump to it. Tab cycles through the list, like Alt+Tab.
- Minimize, close, maximize or restore a window, or quit and force quit its app, from the action panel.

### System Actions
- Lock, sleep, hibernate, restart, shut down, log out, screen saver, sleep displays.
- Volume up/down/presets or a typed value, mute, microphone mute, play/pause, next/previous track.
- Pick the sound output or input device, and set each device's volume or mute it.
- Volume Mixer for each app's volume, plus commands to mute or change the volume of the active app.
- Quit the active app, quit all apps, show desktop.
- Open and empty the Recycle Bin, toggle dark mode, toggle hidden files.
- Toggle Night Light, HDR and camera access.
- Dangerous actions ask for confirmation first.

### Notes
- A floating notes window with a rich-text (Markdown) editor, a format bar, a notes browser and history.

### Calculator and Translator
- Math, units, currencies (live rates), time zones and color conversions.
- **Calculator History** keeps the calculations you copy or paste for three months, with pins and search.
- Translate between 133 languages, with language detection.

### Quicklinks and Web Searches
- Save links with `{argument}`, `{clipboard}` and date placeholders, and open them in a chosen browser.
- Create a quicklink straight from a typed URL, and import or export quicklinks as JSON.
- Google, DuckDuckGo and Bing searches.

### Emoji & Symbols
- Searchable emoji grid by category, with frequently used emoji first.

### Settings
- A settings window for the hotkey, launch at login, tray icon, launcher position, Escape behavior, and the alias, hotkey and on/off switch of every command.
- Install, configure and uninstall FlowKey extensions, and review the permissions each one has.

---

## What's not included

By design, Openray leaves out:

- **Raycast AI**: chat, AI commands, AI extensions and anything that calls a model.
- **Anything that needs Raycast servers or a Raycast account**: Cloud Sync, the Store, Teams, Pro features.
- **Compatibility with Raycast Store extensions.** They rely on macOS-only APIs and Raycast's own runtime. Openray has its own extension API instead (see [Extensions](#extensions)).

Known gaps and differences in the features that do exist are listed in [`docs/known-limitations.md`](docs/known-limitations.md).

---

## Install

1. Open the [latest release](https://github.com/GabrielSantos23/OpenRay-Launcher/releases/latest).
2. Download `openray-windows-x64.zip` and unzip it to a folder of your choice, for example `%LOCALAPPDATA%\Programs\Openray`. Keep the `extensions` folder next to `openray.exe`.
3. Run `openray.exe` and press **Alt+Space**. Turn on **Open at Login** in Settings to start it with Windows.

To update, download the newer zip and replace the files. Your data stays in `%APPDATA%\dev.openray.app`.

> If you also use Raycast, give the two apps different hotkeys (for example Raycast on Ctrl+Alt+Space). Both use a keyboard hook, so the same hotkey can't work in both at once.

**Requirements:** Windows 10 or 11 (x64). Nothing else: no WebView2, .NET or Node.js runtime.

---

## Usage

| Shortcut | What it does |
|---|---|
| **Alt+Space** | Show or hide Openray |
| Type | Search apps, commands, files and extensions |
| **Enter** | Run the primary action |
| **Ctrl+K** | Open the action panel for the selected item |
| **Ctrl+Enter** | Run the secondary action (for example Copy instead of Paste) |
| **Esc** | Clear the search, go back, or close |
| **Backspace** on an empty search | Go back to the root list |

Some commands to try: `clipboard history`, `search files`, `create snippet`, `start focus session`, `left half`, `translate`, `2+2*3`, `100 usd to eur`, `#ff6363`.

---

## Build from source

### Prerequisites

- Windows 10 or 11
- [Rust](https://rustup.rs/) with the MSVC toolchain (`app/rust-toolchain.toml` pins the version)
- [Node.js](https://nodejs.org/) 22+, only to rebuild a FlowKey extension or the extension host bundle

### Run

```bash
cd app
cargo run
```

The app stores its data in `%APPDATA%\dev.openray.app`. Debug builds load the bundled extensions from `extensions/*/dist`.

### Test

```bash
cd app
cargo test
```

Tests that drive real windows or audio devices are marked `#[ignore]`; run them with `cargo test -- --ignored`.

### Release build

```bash
cd app
cargo build --release
```

Put `app/target/release/openray.exe` next to an `extensions` folder that holds each bundled extension's `dist` contents (`extensions/spotify-player/`).

### Extension bundles (npm)

The built bundles are committed, so Cargo alone builds a complete app. Node.js is needed only to rebuild them:

```bash
npm ci
npm run build:extensions
npm run build:host
```

`build:extensions` runs the FlowKey CLI for `extensions/spotify-player`. `build:host` bundles the QuickJS extension host (`app/js`) into `app/assets/flowkey-host.js`.

---

## Project structure

```
openray/
â”œâ”€â”€ app/                     The app (Rust, egui)
â”‚   â”œâ”€â”€ src/main.rs          Startup, tray, single instance, module list
â”‚   â”œâ”€â”€ src/root.rs          Launcher: search, results, views, action panel
â”‚   â”œâ”€â”€ src/settings_ui.rs   Settings window
â”‚   â”œâ”€â”€ src/extension.rs     The built-in extension API (Extension, Command, View)
â”‚   â”œâ”€â”€ src/extensions/      Built-in features, one file per extension
â”‚   â”œâ”€â”€ src/flowkey_*.rs     QuickJS runtime, gate and host for FlowKey extensions
â”‚   â”œâ”€â”€ src/*.rs             Windows backends: hotkey hook, apps, clipboard, files, focus, capture...
â”‚   â”œâ”€â”€ js/                  FlowKey worker host and browser shims (bundled for QuickJS)
â”‚   â””â”€â”€ assets/              Fonts, icons, the built extension host
â”œâ”€â”€ extensions/              Bundled FlowKey extensions (Spotify Player), source and built dist
â”œâ”€â”€ docs/                    Architecture, feature matrix, research notes
â””â”€â”€ .github/workflows/       Release pipeline
```

---

## How it works

Openray is a single native Rust executable. The UI is drawn with **egui** (eframe, OpenGL through glow); there is no WebView, browser engine, Node.js process or .NET runtime.

- **Windows integration:** the keyboard hook, window styling and focus, the app and file indexes, the clipboard, audio (Core Audio), UI Automation (to read browser URLs for Focus), OCR, and simulated keystrokes all go through the Windows APIs directly.
- **Built-in features:** each one implements the `Extension` trait in `app/src/extension.rs` and returns commands that run immediately, open a view (list, grid, form, page or detail), or add inline results to the root search.
- **Windows:** the launcher window is created once and kept hidden, so it opens instantly. Settings, Notes, Focus and screenshot overlays are extra egui viewports created when needed.
- **Data:** stored as JSON files in `%APPDATA%\dev.openray.app`. Nothing is sent anywhere, except the translation, dictionary and currency-rate requests those features need.

The design decisions and module boundaries are explained in [`docs/architecture.md`](docs/architecture.md).

---

## Extensions

Built-in features implement the `Extension` trait in [`app/src/extension.rs`](app/src/extension.rs):

```rust
pub struct Hello;

impl Extension for Hello {
    fn id(&self) -> &'static str {
        "hello"
    }

    fn commands(&self) -> Vec<Command> {
        vec![command(self.id(), "say-hello", "Say Hello", Icon::Glyph("rocket"), "Command", "Run Command", |host| {
            host.hud(Ok("Hello from Openray".into()));
            Ok(())
        })]
    }
}
```

Third-party **FlowKey** extensions are written in TypeScript with the FlowKey CLI. Openray runs each one in its own QuickJS engine with no file, network or OS access of its own; every native call goes through a permission gate based on the extension's manifest and the consent you gave. The bundled [Spotify Player](extensions/spotify-player) is an example.

---

## Releases

Releases are built by GitHub Actions ([`.github/workflows/release.yml`](.github/workflows/release.yml)):

1. Bump the version in `app/Cargo.toml`.
2. Push a tag named `vX.Y.Z`.
3. The workflow runs the tests, builds the release executable, and publishes `openray-windows-x64.zip` (the executable, bundled extensions and license) as a GitHub release.

There is no auto-update yet; see [`docs/known-limitations.md`](docs/known-limitations.md).

---

## Documentation

| Document | Contents |
|---|---|
| [`docs/feature-matrix.md`](docs/feature-matrix.md) | Every Raycast feature, its status in Openray, tests and remaining differences |
| [`docs/architecture.md`](docs/architecture.md) | Stack decision and module boundaries |
| [`docs/known-limitations.md`](docs/known-limitations.md) | What doesn't work yet or works differently |
| [`docs/reverse-engineering-notes.md`](docs/reverse-engineering-notes.md) | How the reference behavior was observed, with evidence |
| [`docs/design-system.md`](docs/design-system.md) | Measured design tokens |
| [`AGENTS.md`](AGENTS.md) | Rules for contributors and coding agents |

---

## Contributing

Issues and pull requests are welcome. Before opening a pull request:

- Run `cargo test` in `app/`, and add tests for new logic.
- Follow the conventions in [`AGENTS.md`](AGENTS.md):
  - colors from `app/src/theme.rs`;
  - no inline comments;
  - only real actions, with no placeholder data.
- Update `docs/feature-matrix.md` and `docs/known-limitations.md` in the same change.
- Don't copy Raycast source code or ship its assets. Reproduce the behavior you can observe.

---

## License

Openray is released under the [MIT License](LICENSE).

Raycast is a trademark of Raycast Technologies Ltd. The MIT License covers Openray's own code only.
