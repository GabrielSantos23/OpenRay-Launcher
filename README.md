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
- [Releases and auto-update](#releases-and-auto-update)
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
- Dark and light themes; follows the Windows accent color.

### Applications
- Indexes Start Menu shortcuts and Microsoft Store (MSIX) apps, with their real icons.
- Run as Administrator, Show in File Explorer, Copy Path.

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
- Volume up/down/presets, mute, microphone mute, play/pause, next/previous track.
- Pick the sound output or input device, and set each device's volume or mute it.
- Quit the active app, quit all apps, show desktop.
- Open and empty the Recycle Bin, toggle dark mode, toggle hidden files.
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

### Settings and updates
- A settings window for the hotkey, launch at login, tray icon, launcher position, Escape behavior, and the alias, hotkey and on/off switch of every command.
- Automatic updates from GitHub Releases, plus a **Check for Updates** button in Settings. When an update is ready, it appears at the top of the launcher.

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
2. Download `Openray_<version>_x64-setup.exe` and run it. The installer runs per user and does not need administrator rights.
3. Press **Alt+Space** to open Openray.

Openray then updates itself whenever a new release is published.

> If you also use Raycast, give the two apps different hotkeys (for example Raycast on Ctrl+Alt+Space). Both use a keyboard hook, so the same hotkey can't work in both at once.

**Requirements:** Windows 10 or 11 (x64) with the Microsoft Edge WebView2 Runtime, which Windows 11 already includes. The installer adds it if it is missing.

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
- [Node.js](https://nodejs.org/) 22+
- [Rust](https://rustup.rs/) (stable) with the MSVC toolchain
- The Tauri 2 prerequisites for Windows: Microsoft C++ Build Tools and WebView2. See the [Tauri prerequisites guide](https://v2.tauri.app/start/prerequisites/).

### Run in development

```bash
npm ci
npm --prefix extensions/spotify-player install
npm run tauri dev
```

The development build uses the identifier `dev.openray.app` and stores its data in `%APPDATA%\dev.openray.app`.

### Test

```bash
npm test
cd src-tauri && cargo test
```

`npm test` runs the Vitest suites for the web layer. `cargo test` covers the Rust core, including file indexing, snippet matching, window placement and the focus rules. Tests that drive real windows or audio devices are marked `#[ignore]`; run them with `cargo test -- --ignored`.

### Build an installer

```bash
npm run tauri build
```

The NSIS installer is written to `src-tauri/target/release/bundle/nsis/`.

---

## Project structure

```
openray/
├── src/                     React + TypeScript UI (one entry, one view per window)
│   ├── App.tsx              Root launcher: search, results, action panel
│   ├── ListView.tsx         Shared list/grid view with a detail pane
│   ├── ActionHost.tsx       Action panel, confirmations, footer
│   ├── extension-api.ts     The built-in extension API (defineExtension)
│   ├── extensions/          Built-in features, one file per extension
│   ├── flowkey/             Runtime for third-party (FlowKey) extensions
│   └── *.ts                 Pure logic with Vitest tests next to it
├── src-tauri/               Rust core (Tauri 2)
│   ├── src/lib.rs           App setup, Tauri commands, tray, window lifecycle
│   ├── src/launcher.rs      Launcher window: show/hide, placement, focus
│   ├── src/hotkey.rs        Low-level keyboard hook and hotkey matching
│   ├── src/apps.rs          App index, icons, ShellExecute
│   ├── src/clipboard.rs     Clipboard listener and history store
│   ├── src/file_search.rs   File index and search
│   ├── src/snippets.rs      Keyword tracking and text expansion
│   ├── src/focus.rs         Focus sessions, blocking, floating windows
│   ├── src/window_layout.rs Window placement
│   ├── src/system_actions.rs Power, audio, media and toggles
│   └── tauri.conf.json      Windows, permissions, updater
├── extensions/              Bundled third-party extensions (Spotify Player)
├── docs/                    Architecture, feature matrix, research notes
└── .github/workflows/       Release pipeline
```

---

## How it works

Openray uses **Tauri 2**: a small Rust core and a web UI rendered by **WebView2**, the same Chromium engine Raycast for Windows uses for its interface. Using the same engine makes it easier to match the reference's look, and the Rust core keeps memory use low. There is no Node.js process or .NET runtime.

- **Rust core:**
  - Owns everything that touches Windows: the keyboard hook, window styling and focus, the app and file indexes, the clipboard, audio (Core Audio), UI Automation (to read browser URLs for Focus), and simulated keystrokes.
  - Exposes these to the UI as Tauri commands and events.
- **Web layer:**
  - React 19, TypeScript and Tailwind CSS v4. Colors are CSS variables, so themes can change them.
  - Every built-in feature is an extension registered through `defineExtension`, in the same way a third-party extension would be.
- **Windows:**
  - The launcher window is created once and kept alive, so it opens instantly.
  - Settings, Notes and the Focus windows are created when needed and destroyed when closed, to save memory.
- **Data:** stored as JSON files in `%APPDATA%\<app identifier>`. Nothing is sent anywhere, except the translation and currency-rate requests those features need.

The design decisions and module boundaries are explained in [`docs/architecture.md`](docs/architecture.md).

---

## Extensions

Built-in features use the internal API in [`src/extension-api.ts`](src/extension-api.ts). An extension declares commands that either run immediately, open a view (list, grid, form or detail), or add items and inline results to the root search:

```ts
import { defineExtension } from "../extension-api";

export default defineExtension({
  id: "hello",
  title: "Hello",
  commands: [
    {
      id: "say-hello",
      title: "Say Hello",
      kind: "Command",
      primaryActionTitle: "Run Command",
      run: () => console.log("Hello from Openray"),
    },
  ],
});
```

Openray can also install third-party **FlowKey** extensions, which are sandboxed in their own frame and talk to the host through a permission-checked bridge. The bundled [Spotify Player](extensions/spotify-player) is an example.

---

## Releases and auto-update

Releases are built by GitHub Actions ([`.github/workflows/release.yml`](.github/workflows/release.yml)):

1. Bump the version in `package.json`, `src-tauri/Cargo.toml` and `src-tauri/tauri.conf.json`.
2. Push a tag named `vX.Y.Z`.
3. The workflow builds the signed NSIS installer and publishes a GitHub release with `latest.json`.

Installed copies check that file for updates. Building signed updates requires the repository secret `TAURI_SIGNING_PRIVATE_KEY`, and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` if the key has a password.

---

## Documentation

| Document | Contents |
|---|---|
| [`docs/feature-matrix.md`](docs/feature-matrix.md) | Every Raycast feature, its status in Openray, tests and remaining differences |
| [`docs/architecture.md`](docs/architecture.md) | Stack decision, module boundaries, Tauri commands and events |
| [`docs/known-limitations.md`](docs/known-limitations.md) | What doesn't work yet or works differently |
| [`docs/reverse-engineering-notes.md`](docs/reverse-engineering-notes.md) | How the reference behavior was observed, with evidence |
| [`docs/design-system.md`](docs/design-system.md) | Measured design tokens |
| [`AGENTS.md`](AGENTS.md) | Rules for contributors and coding agents |

---

## Contributing

Issues and pull requests are welcome. Before opening a pull request:

- Run `npm test` and `cargo test`, and add tests for new logic.
- Follow the conventions in [`AGENTS.md`](AGENTS.md):
  - Tailwind utilities with colors as CSS variables;
  - no inline comments;
  - only real actions, with no placeholder data.
- Update `docs/feature-matrix.md` and `docs/known-limitations.md` in the same change.
- Don't copy Raycast source code or ship its assets. Reproduce the behavior you can observe.

---

## License

Openray is released under the [MIT License](LICENSE).

Raycast is a trademark of Raycast Technologies Ltd. The MIT License covers Openray's own code only.
