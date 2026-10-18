# Feature matrix

Statuses: `NOT_INVESTIGATED`, `INVESTIGATING`, `CONFIRMED`, `IMPLEMENTING`, `IMPLEMENTED`, `TESTED`, `BLOCKED`, `NOT_APPLICABLE`.

Reference: Raycast for Windows 2.8.2.0 (see version note in `reverse-engineering-notes.md`). Offsets are character offsets in `Raycast/backend/index.mjs`.

## Platform and launcher

| Feature | Reference evidence | Windows behavior | Status | Tests | Remaining differences |
|---|---|---|---|---|---|
| Background app, tray icon | `H.NotifyIcon.Wpf.dll`; live window class `H.NotifyIcon_…` (PID 18708) | Tauri tray icon (left click opens, menu Open/Quit); starts hidden without taking focus | TESTED | Manual E2E: started hidden, `foreground=False` | Tray menu clicked by hand only |
| Single instance | `Raycast.InterRaycastPipe.Server/Client`, `BringRaycastToFront` (Raycast.dll) | tauri-plugin-single-instance; 2nd launch shows the launcher | TESTED | Manual E2E: 2nd process exited, launcher shown, 1 process | Reference also routes deep links over the pipe |
| Global hotkey toggles launcher | `SetWindowsHookEx`, `LowLevelKeyboardEvent` (Raycast.dll); setting `globalHotkey` | WH_KEYBOARD_LL hook, default Alt+Space, `hotkey` in settings.json; swallows the chord and suppresses Alt/Win menu activation | TESTED | `hotkey.rs` tests (4: parse, reject, fire/swallow, exact modifiers); 6-cycle toggle probe; Alt+Space taken over from a running Raycast | Shared chords alternate with other hooking apps; no settings UI |
| Launcher window: frameless, topmost, rounded, 750×475 | Live HWND measurement; CSS `--main-window-width` | Popup HWND 750×475, DWM round corners + border #3B3B3B; hidden = layered alpha 0, click-through, no-activate, not topmost; DWM transitions disabled | TESTED | `npm run smoke` (asserts opaque, non-click-through 750×475); pixel diff vs reference = 0 px; show probe ~21–24 ms vs reference ~21 ms | Position rule ≈ (unk_341f…); TOOLWINDOW kept while shown |
| Hide on Escape / on focus loss | `EnableMainWindowHideOnFocusLoss`, setting `escapeKeyBehavior` (index.mjs) | Esc and hotkey hide and return focus to the previous app (~20–33 ms; reference ~30 ms); losing foreground hides without stealing focus back; query resets while hidden | TESTED | Probe; manual E2E; no stale query in first frames after reopen | `escapeKeyBehavior` options not implemented |
| Root search, incremental filtering | Live launcher; `rootSearch` metadata (index.mjs) | Client-side tiered matcher over registry items, updates per keystroke | TESTED | `src/search.test.ts` (13 tests); manual E2E | Matching algorithm unconfirmed (unk_93ec…) |
| Keyboard navigation (↑ ↓ Enter Esc) | Live launcher | ↑/↓ move selection (clamped), Enter runs, Esc hides; mouse hover selects, click runs | TESTED | Manual E2E | Wrap-around behaviour of the reference unknown |
| Frecency ranking, persisted | `decayedScore` 10-day half-life (index.mjs); `frecency.db` | `%APPDATA%\dev.openray.app\frecency.json`, 10-day half-life, +1 per visit; empty query shows top 8 as Suggestions | TESTED | `store.rs` tests (4); manual E2E: survived restart | Visit increment ≈ (unk_f0ba…); suggestion cap ≈ |
| Applications: Start Menu shortcuts | `ShortcutApplicationWatcher`, `ShellExecuteEx` (Raycast.dll) | Scan both Start Menu roots for .lnk/.url/.appref-ms, dedupe by name, launch via ShellExecuteW | TESTED | `apps.rs` tests (2); manual E2E launched Character Map | Letter tiles instead of icons; rescans on every show |
| Applications: MSIX/Store apps | `MsixApplicationsCollector`, `IApplicationActivationManager` | — | CONFIRMED |  |  |
| Applications: app icons | `PngIconDataProvider`, `icons_cache.bin` | — | CONFIRMED |  |  |
| Applications: game launchers (Steam, Epic, GOG, Riot, EA, Blizzard) | `*LauncherBuilder` types, ext `games` | — | CONFIRMED |  |  |
| Control Panel / Settings pages as apps | `ControlPanelItemsDiscovery`, `SystemPreferencesDiscovery` | — | CONFIRMED |  |  |
| Footer: primary action + Actions (Ctrl+K) | Live launcher footer "Open Draft ↵ | Actions Ctrl K" | — | CONFIRMED |  |  |
| Favorites / Suggestions sections in empty root | Live launcher sections "Drafts", "Favorites", "Suggestions" | — | CONFIRMED |  |  |
| Settings window | `settings-window.html`, `Raycast.Handlers.Settings` | — | CONFIRMED |  |  |
| Deep links (raycast://) | AppxManifest protocol `raycast`; `RaycastDeepLinkData` | — | CONFIRMED |  |  |
| Themes (dark/light, custom) | Settings `themeDarkId`/`themeLightId`, `theme-studio-window.html` | — | CONFIRMED |  |  |
| Hyper Key | Feedback category list (index.mjs @6484341) | — | CONFIRMED |  |  |
| Auto Quit apps | `AutoQuitApplicationService`; cmd `manageAutoQuits` | — | CONFIRMED |  |  |
| Cloud Sync | Settings storage `synced`; cmd `openCloudSyncSettings` | — | NOT_APPLICABLE |  | Raycast servers (excluded) |
| Raycast Store / third-party Raycast extensions | `api/node_modules/@raycast/api`; ext `developer` | — | NOT_APPLICABLE |  | Excluded by AGENTS.md |

| Extension API (`defineExtension`, registry) | Built-ins declared as `{key,title,commands}` in `backend/index.mjs` | `src/extension-api.ts`, `src/registry.ts`; `applications` and `openray-core` register through it | TESTED | `src/registry.test.ts` (2) | No views, actions, arguments or preferences yet |

## Built-in commands

| Extension | Command | Reference evidence | Windows behavior | Status | Tests | Remaining differences |
|---|---|---|---|---|---|---|
| AI | AI Chat | `backend/index.mjs` @840113, ext `ai`, cmd `openAiChat` | — | NOT_APPLICABLE |  | AI (excluded) |
| AI | Copy Remote Diagnostics | `backend/index.mjs` @840113, ext `ai`, cmd `copyRemoteAiChatDiagnostics` | — | NOT_APPLICABLE |  | AI (excluded) |
| AI | Quick AI | `backend/index.mjs` @840113, ext `ai`, cmd `openQuickAi` | — | NOT_APPLICABLE |  | AI (excluded) |
| AI | Show AI Credit Usage | `backend/index.mjs` @840113, ext `ai`, cmd `showAiCreditUsage` | — | NOT_APPLICABLE |  | AI (excluded) |
| AI | Show Claude Usage | `backend/index.mjs` @840113, ext `ai`, cmd `showClaudeUsage` | — | NOT_APPLICABLE |  | AI (excluded) |
| AI | Show ChatGPT Usage | `backend/index.mjs` @840113, ext `ai`, cmd `showChatGptUsage` | — | NOT_APPLICABLE |  | AI (excluded) |
| AI | Show GitHub Copilot Usage | `backend/index.mjs` @840113, ext `ai`, cmd `showCopilotUsage` | — | NOT_APPLICABLE |  | AI (excluded) |
| AI | Manage Models | `backend/index.mjs` @840113, ext `ai`, cmd `manageModels` | — | NOT_APPLICABLE |  | AI (excluded) |
| AI | Profile | `backend/index.mjs` @840113, ext `ai`, cmd `openAiProfile` | — | NOT_APPLICABLE |  | AI (excluded) |
| AI | Show Memory | `backend/index.mjs` @840113, ext `ai`, cmd `openAiMemory` | — | NOT_APPLICABLE |  | AI (excluded) |
| AI | Import Memory | `backend/index.mjs` @840113, ext `ai`, cmd `openAiMemoryImport` | — | NOT_APPLICABLE |  | AI (excluded) |
| AI | Create AI Command | `backend/index.mjs` @840113, ext `ai`, cmd `createAiCommand` | — | NOT_APPLICABLE |  | AI (excluded) |
| AI | Search AI Commands | `backend/index.mjs` @840113, ext `ai`, cmd `searchAiCommands` | — | NOT_APPLICABLE |  | AI (excluded) |
| AI | Import AI Commands | `backend/index.mjs` @840113, ext `ai`, cmd `importAiCommands` | — | NOT_APPLICABLE |  | AI (excluded) |
| AI | Create Automation | `backend/index.mjs` @840113, ext `ai`, cmd `createAutomation` | — | NOT_APPLICABLE |  | AI (excluded) |
| AI | Manage Automations | `backend/index.mjs` @840113, ext `ai`, cmd `manageAutomations` | — | NOT_APPLICABLE |  | AI (excluded) |
| AI | Create Agent | `backend/index.mjs` @840113, ext `ai`, cmd `createAiMode` | — | NOT_APPLICABLE |  | AI (excluded) |
| AI | Create Team Skill | `backend/index.mjs` @840113, ext `ai`, cmd `createSkill` | — | NOT_APPLICABLE |  | AI (excluded) |
| AI | Search Agents | `backend/index.mjs` @840113, ext `ai`, cmd `searchAiModes` | — | NOT_APPLICABLE |  | AI (excluded) |
| AI | Import Agents | `backend/index.mjs` @840113, ext `ai`, cmd `importAiModes` | — | NOT_APPLICABLE |  | AI (excluded) |
| AI | Send Selected Text to AI | `backend/index.mjs` @840113, ext `ai`, cmd `sendSelectedText` | — | NOT_APPLICABLE |  | AI (excluded) |
| AI | Send Focused Window to AI | `backend/index.mjs` @840113, ext `ai`, cmd `sendFocusedWindow` | — | NOT_APPLICABLE |  | AI (excluded) |
| AI | Send Screen to AI | `backend/index.mjs` @840113, ext `ai`, cmd `sendScreen` | — | NOT_APPLICABLE |  | AI (excluded) |
| AI | Send Screen Area to AI | `backend/index.mjs` @840113, ext `ai`, cmd `sendScreenArea` | — | NOT_APPLICABLE |  | AI (excluded) |
| AI | Quick Fix | `backend/index.mjs` @840113, ext `ai`, cmd `quickFixSpellingGrammar` | — | NOT_APPLICABLE |  | AI (excluded) |
| AI | Ask AI Chat | `backend/index.mjs` @840113, ext `ai`, cmd `chatAiExtension` | — | NOT_APPLICABLE |  | AI (excluded) |
| AI | Ask Memory | `backend/index.mjs` @840113, ext `ai`, cmd `memoryAiExtension` | — | NOT_APPLICABLE |  | AI (excluded) |
| AI | Ask Terminal | `backend/index.mjs` @840113, ext `ai`, cmd `terminalAiExtension` | — | NOT_APPLICABLE |  | AI (excluded) |
| AI | Ask Location | `backend/index.mjs` @840113, ext `ai`, cmd `locationAiExtension` | — | NOT_APPLICABLE |  | AI (excluded) |
| AI | Ask Selected Text | `backend/index.mjs` @840113, ext `ai`, cmd `selectedTextAiExtension` | — | NOT_APPLICABLE |  | AI (excluded) |
| AI | Ask Weather | `backend/index.mjs` @840113, ext `ai`, cmd `weatherAiExtension` | — | NOT_APPLICABLE |  | AI (excluded) |
| AI | Ask Agents | `backend/index.mjs` @840113, ext `ai`, cmd `agentsAiExtension` | — | NOT_APPLICABLE |  | AI (excluded) |
| AI | Ask Automations | `backend/index.mjs` @840113, ext `ai`, cmd `automationsAiExtension` | — | NOT_APPLICABLE |  | AI (excluded) |
| Applications | Uninstall Applications | `backend/index.mjs` @862881, ext `applications`, cmd `uninstall` | — | CONFIRMED |  |  |
| Applications | Manage Auto Quits | `backend/index.mjs` @862881, ext `applications`, cmd `manageAutoQuits` | — | CONFIRMED |  |  |
| Browser | Install Browser Extension | `backend/index.mjs` @867935, ext `browser`, cmd `installBrowserExtension` | — | NOT_APPLICABLE |  | Raycast browser extension (store) |
| Browser | Ask Browser | `backend/index.mjs` @867935, ext `browser`, cmd `aiExtension` | — | NOT_APPLICABLE |  | AI (excluded) |
| Calculator | Color | `backend/index.mjs` @871785, ext `calculator`, cmd `color` | — | CONFIRMED |  |  |
| Calculator | Flight Tracker | `backend/index.mjs` @871785, ext `calculator`, cmd `flight` | — | NOT_APPLICABLE |  | Raycast servers |
| Calculator | Calculator | `backend/index.mjs` @871785, ext `calculator`, cmd `calculate` | — | CONFIRMED |  |  |
| Calculator | Weather | `backend/index.mjs` @871785, ext `calculator`, cmd `weather` | — | NOT_APPLICABLE |  | Raycast servers |
| Calculator | Calculator History | `backend/index.mjs` @871785, ext `calculator`, cmd `history` | — | CONFIRMED |  |  |
| Calculator | Ask Calculator | `backend/index.mjs` @871785, ext `calculator`, cmd `aiExtension` | — | NOT_APPLICABLE |  | AI (excluded) |
| Calendar | My Schedule | `backend/index.mjs` @881514, ext `calendar`, cmd `mySchedule` | — | CONFIRMED |  |  |
| Calendar | Create Event | `backend/index.mjs` @881514, ext `calendar`, cmd `createEvent` | — | CONFIRMED |  |  |
| Calendar | Ask Calendar | `backend/index.mjs` @881514, ext `calendar`, cmd `aiExtension` | — | NOT_APPLICABLE |  | AI (excluded) |
| Clipboard History | Clipboard History | `backend/index.mjs` @885550, ext `clipboard-history`, cmd `history` | — | CONFIRMED |  |  |
| Clipboard History | Paste Sequentially | `backend/index.mjs` @885550, ext `clipboard-history`, cmd `pasteSequential` | — | CONFIRMED |  |  |
| Clipboard History | Reset Paste Sequence | `backend/index.mjs` @885550, ext `clipboard-history`, cmd `resetPasteSequence` | — | CONFIRMED |  |  |
| Clipboard History | Ask Clipboard | `backend/index.mjs` @885550, ext `clipboard-history`, cmd `aiExtension` | — | NOT_APPLICABLE |  | AI (excluded) |
| Contacts | Search Contacts | `backend/index.mjs` @888849, ext `contacts`, cmd `search` | — | CONFIRMED |  |  |
| Developer | Import Extension | `backend/index.mjs` @892092, ext `developer`, cmd `importExtension` | — | NOT_APPLICABLE |  | Raycast extension compatibility (excluded) |
| Developer | Create Extension | `backend/index.mjs` @892092, ext `developer`, cmd `createExtension` | — | NOT_APPLICABLE |  | Raycast extension compatibility (excluded) |
| Developer | Manage Extensions | `backend/index.mjs` @892092, ext `developer`, cmd `manageExtensions` | — | NOT_APPLICABLE |  | Raycast extension compatibility (excluded) |
| Developer | Extension Diagnostics | `backend/index.mjs` @892092, ext `developer`, cmd `extensionDiagnostics` | — | NOT_APPLICABLE |  | Raycast extension compatibility (excluded) |
| Developer | Ask Developer | `backend/index.mjs` @892092, ext `developer`, cmd `aiExtension` | — | NOT_APPLICABLE |  | AI (excluded) |
| Dictation | Dictate | `backend/index.mjs` @894776, ext `dictation`, cmd `dictateText` | — | NOT_APPLICABLE |  | Server transcription (excluded) |
| Dictation | Dictate to AI | `backend/index.mjs` @894776, ext `dictation`, cmd `dictateToAi` | — | NOT_APPLICABLE |  | Server transcription (excluded) |
| Dictation | Dictate to Note | `backend/index.mjs` @894776, ext `dictation`, cmd `dictateToNote` | — | NOT_APPLICABLE |  | Server transcription (excluded) |
| Dictation | Get Last Transcription | `backend/index.mjs` @894776, ext `dictation`, cmd `copyLastTranscription` | — | NOT_APPLICABLE |  | Server transcription (excluded) |
| Dictation | Add Word to Vocabulary | `backend/index.mjs` @894776, ext `dictation`, cmd `addWordToVocabulary` | — | NOT_APPLICABLE |  | Server transcription (excluded) |
| Dictation | Finish Dictation | `backend/index.mjs` @894776, ext `dictation`, cmd `finishDictation` | — | NOT_APPLICABLE |  | Server transcription (excluded) |
| Dictation | Cancel Dictation | `backend/index.mjs` @894776, ext `dictation`, cmd `cancelDictation` | — | NOT_APPLICABLE |  | Server transcription (excluded) |
| Dictation | Dictation History | `backend/index.mjs` @894776, ext `dictation`, cmd `searchTranscriptions` | — | NOT_APPLICABLE |  | Server transcription (excluded) |
| Dictation | Ask Dictation | `backend/index.mjs` @894776, ext `dictation`, cmd `aiExtension` | — | NOT_APPLICABLE |  | Server transcription (excluded) |
| Dictation | Custom Instructions | `backend/index.mjs` @894776, ext `dictation`, cmd `openDictationCustomInstructions` | — | NOT_APPLICABLE |  | Server transcription (excluded) |
| Dictation | Vocabulary | `backend/index.mjs` @894776, ext `dictation`, cmd `openDictationVocabulary` | — | NOT_APPLICABLE |  | Server transcription (excluded) |
| Dictation | Create Style | `backend/index.mjs` @894776, ext `dictation`, cmd `createTranscriptionStyle` | — | NOT_APPLICABLE |  | Server transcription (excluded) |
| Dictation | Search Styles | `backend/index.mjs` @894776, ext `dictation`, cmd `searchTranscriptionStyles` | — | NOT_APPLICABLE |  | Server transcription (excluded) |
| Dictation | Get Started with Dictation | `backend/index.mjs` @894776, ext `dictation`, cmd `setUpDictation` | — | NOT_APPLICABLE |  | Server transcription (excluded) |
| Dictionary | Define Word | `backend/index.mjs` @902832, ext `dictionary`, cmd `defineWord` | — | CONFIRMED |  |  |
| Emoji & Symbols | Search Emoji & Symbols | `backend/index.mjs` @904013, ext `emoji-picker`, cmd `searchEmoji` | — | CONFIRMED |  |  |
| File Search | Search Files | `backend/index.mjs` @906004, ext `file-search`, cmd `searchFiles` | — | CONFIRMED |  |  |
| File Search | Quick Look | `backend/index.mjs` @906004, ext `file-search`, cmd `quickLookSelection` | — | CONFIRMED |  |  |
| File Search | File Search is indexing\u2026 | `backend/index.mjs` @906004, ext `file-search`, cmd `indexing` | — | CONFIRMED |  |  |
| File Search | Ask Finder | `backend/index.mjs` @906004, ext `file-search`, cmd `aiExtension` | — | NOT_APPLICABLE |  | AI (excluded) |
| File Search | Ask File Explorer | `backend/index.mjs` @906004, ext `file-search`, cmd `fileExplorerAiExtension` | — | NOT_APPLICABLE |  | AI (excluded) |
| File Search | Ask File Search | `backend/index.mjs` @906004, ext `file-search`, cmd `fileSearchAiExtension` | — | NOT_APPLICABLE |  | AI (excluded) |
| Raycast Focus | Start Focus Session | `backend/index.mjs` @914373, ext `raycast-focus`, cmd `start` | — | CONFIRMED |  |  |
| Raycast Focus | Toggle Focus Session | `backend/index.mjs` @914373, ext `raycast-focus`, cmd `toggle` | — | CONFIRMED |  |  |
| Raycast Focus | Pause Focus Session | `backend/index.mjs` @914373, ext `raycast-focus`, cmd `pause` | — | CONFIRMED |  |  |
| Raycast Focus | Resume Focus Session | `backend/index.mjs` @914373, ext `raycast-focus`, cmd `resume` | — | CONFIRMED |  |  |
| Raycast Focus | Complete Focus Session | `backend/index.mjs` @914373, ext `raycast-focus`, cmd `complete` | — | CONFIRMED |  |  |
| Raycast Focus | Edit Focus Session | `backend/index.mjs` @914373, ext `raycast-focus`, cmd `edit` | — | CONFIRMED |  |  |
| Raycast Focus | Search Focus Categories | `backend/index.mjs` @914373, ext `raycast-focus`, cmd `searchCategories` | — | CONFIRMED |  |  |
| Raycast Focus | Create Focus Category | `backend/index.mjs` @914373, ext `raycast-focus`, cmd `createCategory` | — | CONFIRMED |  |  |
| Raycast Focus | Import Focus Categories | `backend/index.mjs` @914373, ext `raycast-focus`, cmd `importCategories` | — | CONFIRMED |  |  |
| Raycast Focus | Ask Raycast Focus | `backend/index.mjs` @914373, ext `raycast-focus`, cmd `aiExtension` | — | NOT_APPLICABLE |  | AI (excluded) |
| Games | Game Launcher | `backend/index.mjs` @919098, ext `games`, cmd `searchGames` | — | CONFIRMED |  |  |
| MCP | Install MCP Server | `backend/index.mjs` @922294, ext `mcp`, cmd `installServer` | — | NOT_APPLICABLE |  | AI/MCP (excluded) |
| MCP | Manage MCP Servers | `backend/index.mjs` @922294, ext `mcp`, cmd `manageServers` | — | NOT_APPLICABLE |  | AI/MCP (excluded) |
| MCP | Ask MCP | `backend/index.mjs` @922294, ext `mcp`, cmd `aiExtension` | — | NOT_APPLICABLE |  | AI/MCP (excluded) |
| MCP | Switch Windows | `backend/index.mjs` @922294, ext `mcp`, cmd `switchWindows` | — | CONFIRMED |  |  |
| MCP | Search Menu Bar Items | `backend/index.mjs` @922294, ext `mcp`, cmd `searchMenuItems` | — | CONFIRMED |  |  |
| Raycast Notes | Raycast Notes | `backend/index.mjs` @925510, ext `notes`, cmd `open` | — | CONFIRMED |  |  |
| Raycast Notes | Search Notes | `backend/index.mjs` @925510, ext `notes`, cmd `search` | — | CONFIRMED |  |  |
| Raycast Notes | Create Note | `backend/index.mjs` @925510, ext `notes`, cmd `create` | — | CONFIRMED |  |  |
| Raycast Notes | Ask Raycast Notes | `backend/index.mjs` @925510, ext `notes`, cmd `aiExtension` | — | NOT_APPLICABLE |  | AI (excluded) |
| Raycast for Teams | Manage Organizations | `backend/index.mjs` @926971, ext `organizations`, cmd `manageOrganizations` | — | NOT_APPLICABLE |  | Raycast Teams, server (excluded) |
| Raycast for Teams | Create Organization | `backend/index.mjs` @926971, ext `organizations`, cmd `createOrganization` | — | NOT_APPLICABLE |  | Raycast Teams, server (excluded) |
| Raycast for Teams | Copy Organization Invite Link | `backend/index.mjs` @926971, ext `organizations`, cmd `copyOrganizationInvite` | — | NOT_APPLICABLE |  | Raycast Teams, server (excluded) |
| Raycast for Teams | Manage Webhooks | `backend/index.mjs` @926971, ext `organizations`, cmd `manageWebhooks` | — | NOT_APPLICABLE |  | Raycast Teams, server (excluded) |
| Raycast for Teams | Create Webhook | `backend/index.mjs` @926971, ext `organizations`, cmd `createWebhook` | — | NOT_APPLICABLE |  | Raycast Teams, server (excluded) |
| Quicklinks | Create Quicklink | `backend/index.mjs` @939378, ext `quicklinks`, cmd `create` | — | CONFIRMED |  |  |
| Quicklinks | Create Quicklink | `backend/index.mjs` @939378, ext `quicklinks`, cmd `createWithURL` | — | CONFIRMED |  |  |
| Quicklinks | Search Quicklinks | `backend/index.mjs` @939378, ext `quicklinks`, cmd `search` | — | CONFIRMED |  |  |
| Quicklinks | Import Quicklinks | `backend/index.mjs` @939378, ext `quicklinks`, cmd `import` | — | CONFIRMED |  |  |
| Quicklinks | Export Quicklinks | `backend/index.mjs` @939378, ext `quicklinks`, cmd `export` | — | CONFIRMED |  |  |
| Quicklinks | Ask Quicklinks | `backend/index.mjs` @939378, ext `quicklinks`, cmd `aiExtension` | — | NOT_APPLICABLE |  | AI (excluded) |
| Account | Log Out | `backend/index.mjs` @943709, ext `raycast-account`, cmd `logout` | — | NOT_APPLICABLE |  | Raycast account (excluded) |
| Raycast | Install Update | `backend/index.mjs` @944232, ext `raycast-core`, cmd `installUpdate` | — | NOT_APPLICABLE |  | Raycast update server |
| Raycast | Upgrade to Raycast 2.0 | `backend/index.mjs` @944232, ext `raycast-core`, cmd `upgradeToV2` | — | NOT_APPLICABLE |  | Raycast v1 migration |
| Raycast | Release Notes | `backend/index.mjs` @944232, ext `raycast-core`, cmd `whatsNew` | — | NOT_APPLICABLE |  | Raycast servers |
| Raycast | Announcement | `backend/index.mjs` @944232, ext `raycast-core`, cmd `announcement` | — | NOT_APPLICABLE |  | Raycast servers |
| Raycast | Get Started with Raycast | `backend/index.mjs` @944232, ext `raycast-core`, cmd `getStarted` | — | CONFIRMED |  |  |
| Raycast | Sign In Again | `backend/index.mjs` @944232, ext `raycast-core`, cmd `sessionExpired` | — | NOT_APPLICABLE |  | Account |
| Raycast | Check for Updates | `backend/index.mjs` @944232, ext `raycast-core`, cmd `checkForUpdates` | — | NOT_APPLICABLE |  | Raycast update server |
| Raycast | Check for Extension Updates | `backend/index.mjs` @944232, ext `raycast-core`, cmd `checkForExtensionUpdates` | — | NOT_APPLICABLE |  | Store (excluded) |
| Raycast | Force Update All Extensions | `backend/index.mjs` @944232, ext `raycast-core`, cmd `forceUpdateAllExtensions` | — | NOT_APPLICABLE |  | Store (excluded) |
| Raycast | Quit Raycast | `backend/index.mjs` @944232, ext `raycast-core`, cmd `quitApp` | "Quit Openray" in `openray-core` (app.exit) | IMPLEMENTED | Not run end-to-end | |
| Raycast | Settings | `backend/index.mjs` @944232, ext `raycast-core`, cmd `openSettings` | — | CONFIRMED |  |  |
| Raycast | Show Onboarding | `backend/index.mjs` @944232, ext `raycast-core`, cmd `showOnboarding` | — | CONFIRMED |  |  |
| Raycast | Open Pro Walkthrough | `backend/index.mjs` @944232, ext `raycast-core`, cmd `proWalkthrough` | — | NOT_APPLICABLE |  | Pro/account |
| Raycast | Reset Raycast Window Position | `backend/index.mjs` @944232, ext `raycast-core`, cmd `resetPosition` | — | CONFIRMED |  |  |
| Raycast | Confetti | `backend/index.mjs` @944232, ext `raycast-core`, cmd `confetti` | — | CONFIRMED |  |  |
| Raycast | Toggle Bounce Animation | `backend/index.mjs` @944232, ext `raycast-core`, cmd `toggleBounceAnimation` | — | NOT_APPLICABLE |  | Internal |
| Raycast | Inspect WebView | `backend/index.mjs` @944232, ext `raycast-core`, cmd `inspectWebView` | — | NOT_APPLICABLE |  | Internal |
| Raycast | Fix Login Items | `backend/index.mjs` @944232, ext `raycast-core`, cmd `fixLoginItems` | — | NOT_APPLICABLE |  | macOS-only |
| Raycast | Toggle Performance Logs | `backend/index.mjs` @944232, ext `raycast-core`, cmd `togglePerformanceLogs` | — | NOT_APPLICABLE |  | Internal |
| Raycast | Show Theme Studio | `backend/index.mjs` @944232, ext `raycast-core`, cmd `showThemeStudio` | — | CONFIRMED |  |  |
| Raycast | Switch Theme | `backend/index.mjs` @944232, ext `raycast-core`, cmd `switchTheme` | — | CONFIRMED |  |  |
| Raycast | Open Manual | `backend/index.mjs` @944232, ext `raycast-core`, cmd `openManual` | — | CONFIRMED |  |  |
| Raycast | Ask Manual | `backend/index.mjs` @944232, ext `raycast-core`, cmd `askManual` | — | NOT_APPLICABLE |  | AI (excluded) |
| Raycast | Send Feedback | `backend/index.mjs` @944232, ext `raycast-core`, cmd `sendFeedback` | — | NOT_APPLICABLE |  | Raycast servers |
| Raycast | Ask Feedback | `backend/index.mjs` @944232, ext `raycast-core`, cmd `askFeedback` | — | NOT_APPLICABLE |  | AI (excluded) |
| Raycast | Changelog | `backend/index.mjs` @944232, ext `raycast-core`, cmd `changelog` | — | NOT_APPLICABLE |  | Raycast servers |
| Raycast | Store | `backend/index.mjs` @944232, ext `raycast-core`, cmd `openSearchStore` | — | NOT_APPLICABLE |  | Store (excluded) |
| Raycast | Ask Store | `backend/index.mjs` @944232, ext `raycast-core`, cmd `askStore` | — | NOT_APPLICABLE |  | AI (excluded) |
| Raycast | Share Raycast | `backend/index.mjs` @944232, ext `raycast-core`, cmd `shareRaycast` | — | CONFIRMED |  |  |
| Raycast | Copy Raycast Version | `backend/index.mjs` @944232, ext `raycast-core`, cmd `copyRaycastVersion` | — | CONFIRMED |  |  |
| Raycast | Copy Raycast Logs | `backend/index.mjs` @944232, ext `raycast-core`, cmd `copyLogFiles` | — | CONFIRMED |  |  |
| Raycast | Investigate Raycast Logs | `backend/index.mjs` @944232, ext `raycast-core`, cmd `investigateLogFiles` | — | CONFIRMED |  |  |
| Raycast | Reveal Raycast Logs | `backend/index.mjs` @944232, ext `raycast-core`, cmd `revealLogFiles` | — | CONFIRMED |  |  |
| Raycast | Create Raycast Memory Dump | `backend/index.mjs` @944232, ext `raycast-core`, cmd `createMemoryDump` | — | NOT_APPLICABLE |  | Internal |
| Raycast | Capture Window | `backend/index.mjs` @944232, ext `raycast-core`, cmd `windowCapture` | — | CONFIRMED |  |  |
| Raycast | Open Camera | `backend/index.mjs` @944232, ext `raycast-core`, cmd `openCamera` | — | CONFIRMED |  |  |
| Raycast | Export Settings & Data | `backend/index.mjs` @944232, ext `raycast-core`, cmd `exportSettingsAndData` | — | CONFIRMED |  |  |
| Raycast | Import Settings & Data | `backend/index.mjs` @944232, ext `raycast-core`, cmd `importSettingsAndData` | — | CONFIRMED |  |  |
| Raycast | Migrate from Raycast v1 | `backend/index.mjs` @944232, ext `raycast-core`, cmd `migrateClassicSettingsData` | — | NOT_APPLICABLE |  | Raycast v1 migration |
| Raycast | Become an Affiliate | `backend/index.mjs` @944232, ext `raycast-core`, cmd `becomeAnAffiliate` | — | NOT_APPLICABLE |  | Account |
| Raycast | Open Affiliate Dashboard | `backend/index.mjs` @944232, ext `raycast-core`, cmd `openAffiliateDashboard` | — | NOT_APPLICABLE |  | Account |
| Debug | Memory History | `backend/index.mjs` @960609, ext `raycast-debug`, cmd `memoryHistory` | — | NOT_APPLICABLE |  | Internal debug tooling |
| Debug | Activate Raycast Palette | `backend/index.mjs` @960609, ext `raycast-debug`, cmd `activateRaycastPalette` | — | NOT_APPLICABLE |  | Internal debug tooling |
| Debug | Create Debug Clipboard History Items | `backend/index.mjs` @960609, ext `raycast-debug`, cmd `createDebugClipboardHistory` | — | NOT_APPLICABLE |  | Internal debug tooling |
| Debug | Show HUD (Fade Out Test) | `backend/index.mjs` @960609, ext `raycast-debug`, cmd `showHudFadeTest` | — | NOT_APPLICABLE |  | Internal debug tooling |
| Raycast Settings | Ask Raycast Settings | `backend/index.mjs` @968743, ext `raycast-settings`, cmd `aiExtension` | — | NOT_APPLICABLE |  | AI (excluded) |
| Raycast Settings | Account | `backend/index.mjs` @968743, ext `raycast-settings`, cmd `openAccountSettings` | — | NOT_APPLICABLE |  | Account |
| Raycast Settings | General | `backend/index.mjs` @968743, ext `raycast-settings`, cmd `openGeneralSettings` | — | CONFIRMED |  |  |
| Raycast Settings | Launcher | `backend/index.mjs` @968743, ext `raycast-settings`, cmd `openLauncherSettings` | — | CONFIRMED |  |  |
| Raycast Settings | Shortcuts | `backend/index.mjs` @968743, ext `raycast-settings`, cmd `openShortcutsSettings` | — | CONFIRMED |  |  |
| Raycast Settings | Keyboard | `backend/index.mjs` @968743, ext `raycast-settings`, cmd `openKeyboardSettings` | — | CONFIRMED |  |  |
| Raycast Settings | Cloud Sync | `backend/index.mjs` @968743, ext `raycast-settings`, cmd `openCloudSyncSettings` | — | NOT_APPLICABLE |  | Cloud Sync (account) |
| Raycast Settings | Advanced | `backend/index.mjs` @968743, ext `raycast-settings`, cmd `openAdvancedSettings` | — | CONFIRMED |  |  |
| Raycast Settings | Organizations | `backend/index.mjs` @968743, ext `raycast-settings`, cmd `openOrganizationsSettings` | — | NOT_APPLICABLE |  | Teams (account) |
| Raycast Settings | Internal | `backend/index.mjs` @968743, ext `raycast-settings`, cmd `openInternalSettings` | — | NOT_APPLICABLE |  | Internal |
| Raycast Settings | About | `backend/index.mjs` @968743, ext `raycast-settings`, cmd `openAboutSettings` | — | CONFIRMED |  |  |
| Raycast Wrapped | Raycast Wrapped | `backend/index.mjs` @973184, ext `raycast-wrapped`, cmd `showRaycastWrapped` | — | NOT_APPLICABLE |  | Raycast servers (excluded) |
| Screen Awareness | Send Selected Text to AI | `backend/index.mjs` @974697, ext `screen-awareness`, cmd `sendSelectedText` | — | NOT_APPLICABLE |  | AI (excluded) |
| Screen Awareness | Send Screen to AI | `backend/index.mjs` @974697, ext `screen-awareness`, cmd `sendScreen` | — | NOT_APPLICABLE |  | AI (excluded) |
| Screen Awareness | Send Screen Area to AI | `backend/index.mjs` @974697, ext `screen-awareness`, cmd `sendScreenArea` | — | NOT_APPLICABLE |  | AI (excluded) |
| Screen Awareness | Send Window to AI | `backend/index.mjs` @974697, ext `screen-awareness`, cmd `sendWindow` | — | NOT_APPLICABLE |  | AI (excluded) |
| Screen Awareness | Send Element to AI | `backend/index.mjs` @974697, ext `screen-awareness`, cmd `sendElement` | — | NOT_APPLICABLE |  | AI (excluded) |
| Screen Awareness | Send Focused Window to AI | `backend/index.mjs` @974697, ext `screen-awareness`, cmd `sendFocusedWindow` | — | NOT_APPLICABLE |  | AI (excluded) |
| Screen Awareness | Ask Screen Awareness | `backend/index.mjs` @974697, ext `screen-awareness`, cmd `aiExtension` | — | NOT_APPLICABLE |  | AI (excluded) |
| Screenshots | Search Screenshots | `backend/index.mjs` @978249, ext `screenshots`, cmd `searchScreenshots` | — | CONFIRMED |  |  |
| Screenshots | Paste Latest Screenshot | `backend/index.mjs` @978249, ext `screenshots`, cmd `pasteLatestScreenshot` | — | CONFIRMED |  |  |
| Script Commands | Search Script Commands | `backend/index.mjs` @980340, ext `script-commands`, cmd `searchScriptCommands` | — | CONFIRMED |  |  |
| Script Commands | Create Script Command | `backend/index.mjs` @980340, ext `script-commands`, cmd `createScriptCommand` | — | CONFIRMED |  |  |
| Script Commands | Reload Script Commands | `backend/index.mjs` @980340, ext `script-commands`, cmd `reloadScriptCommands` | — | CONFIRMED |  |  |
| Snippets | Create Snippet | `backend/index.mjs` @983121, ext `snippets`, cmd `createSnippet` | — | CONFIRMED |  |  |
| Snippets | Search Snippets | `backend/index.mjs` @983121, ext `snippets`, cmd `searchSnippets` | — | CONFIRMED |  |  |
| Snippets | Auto Snippet | `backend/index.mjs` @983121, ext `snippets`, cmd `autoSnippet` | — | CONFIRMED |  |  |
| Snippets | Import Snippets | `backend/index.mjs` @983121, ext `snippets`, cmd `importSnippets` | — | CONFIRMED |  |  |
| Snippets | Export Snippets | `backend/index.mjs` @983121, ext `snippets`, cmd `exportSnippets` | — | CONFIRMED |  |  |
| Snippets | Ask Snippets | `backend/index.mjs` @983121, ext `snippets`, cmd `aiExtension` | — | NOT_APPLICABLE |  | AI (excluded) |
| System Actions | Dismiss Notifications | `backend/index.mjs` @987025, ext `system`, cmd `dismissNotifications` | — | CONFIRMED |  |  |
| System Actions | Eject All Disks | `backend/index.mjs` @987025, ext `system`, cmd `ejectAllDisks` | — | NOT_APPLICABLE |  | macOS-only |
| System Actions | Hide All Apps Except Frontmost | `backend/index.mjs` @987025, ext `system`, cmd `hideAllAppsExceptFrontmost` | — | NOT_APPLICABLE |  | macOS-only |
| System Actions | Lock Screen | `backend/index.mjs` @987025, ext `system`, cmd `lockScreen` | — | CONFIRMED |  |  |
| System Actions | Log Out | `backend/index.mjs` @987025, ext `system`, cmd `logOut` | — | CONFIRMED |  |  |
| System Actions | Quit All Apps | `backend/index.mjs` @987025, ext `system`, cmd `quitAllApplications` | — | CONFIRMED |  |  |
| System Actions | Quit All Apps Except Frontmost | `backend/index.mjs` @987025, ext `system`, cmd `quitAllApplicationsExceptFrontmost` | — | CONFIRMED |  |  |
| System Actions | Quit the Active App | `backend/index.mjs` @987025, ext `system`, cmd `quitActiveApplication` | — | CONFIRMED |  |  |
| System Actions | Restart | `backend/index.mjs` @987025, ext `system`, cmd `restart` | — | CONFIRMED |  |  |
| System Actions | Manage Wi-Fi Connections | `backend/index.mjs` @987025, ext `system`, cmd `wifiNetworks` | — | CONFIRMED |  |  |
| System Actions | Manage Bluetooth Devices | `backend/index.mjs` @987025, ext `system`, cmd `bluetoothDevices` | — | CONFIRMED |  |  |
| System Actions | Volume Mixer | `backend/index.mjs` @987025, ext `system`, cmd `appVolume` | — | CONFIRMED |  |  |
| System Actions | Set Sound Output Device | `backend/index.mjs` @987025, ext `system`, cmd `setSoundOutputDevice` | — | CONFIRMED |  |  |
| System Actions | Set Sound Input Device | `backend/index.mjs` @987025, ext `system`, cmd `setSoundInputDevice` | — | CONFIRMED |  |  |
| System Actions | Toggle Active App Mute | `backend/index.mjs` @987025, ext `system`, cmd `toggleMuteCurrentApp` | — | CONFIRMED |  |  |
| System Actions | Turn Active App Volume Up | `backend/index.mjs` @987025, ext `system`, cmd `turnCurrentAppVolumeUp` | — | CONFIRMED |  |  |
| System Actions | Turn Active App Volume Down | `backend/index.mjs` @987025, ext `system`, cmd `turnCurrentAppVolumeDown` | — | CONFIRMED |  |  |
| System Actions | Set Volume (+ presets 0/25/50/75/100%) | `backend/index.mjs` @987025, ext `system`, cmd `setVolume` | — | CONFIRMED |  |  |
| System Actions | Play / Pause | `backend/index.mjs` @987025, ext `system`, cmd `playPause` | — | CONFIRMED |  |  |
| System Actions | Next Track | `backend/index.mjs` @987025, ext `system`, cmd `nextTrack` | — | CONFIRMED |  |  |
| System Actions | Previous Track | `backend/index.mjs` @987025, ext `system`, cmd `previousTrack` | — | CONFIRMED |  |  |
| System Actions | Show Desktop | `backend/index.mjs` @987025, ext `system`, cmd `showDesktop` | — | CONFIRMED |  |  |
| System Actions | Show Screen Saver | `backend/index.mjs` @987025, ext `system`, cmd `showScreenSaver` | — | CONFIRMED |  |  |
| System Actions | Shut Down | `backend/index.mjs` @987025, ext `system`, cmd `shutDown` | — | CONFIRMED |  |  |
| System Actions | Sleep | `backend/index.mjs` @987025, ext `system`, cmd `sleep` | — | CONFIRMED |  |  |
| System Actions | Hibernate | `backend/index.mjs` @987025, ext `system`, cmd `hibernate` | — | CONFIRMED |  |  |
| System Actions | Sleep Displays | `backend/index.mjs` @987025, ext `system`, cmd `sleepDisplays` | — | CONFIRMED |  |  |
| System Actions | Toggle WiFi | `backend/index.mjs` @987025, ext `system`, cmd `toggleWifi` | — | CONFIRMED |  |  |
| System Actions | Toggle Airplane Mode | `backend/index.mjs` @987025, ext `system`, cmd `toggleAirplaneMode` | — | CONFIRMED |  |  |
| System Actions | Toggle Bluetooth | `backend/index.mjs` @987025, ext `system`, cmd `toggleBluetooth` | — | CONFIRMED |  |  |
| System Actions | Toggle Camera Mute | `backend/index.mjs` @987025, ext `system`, cmd `toggleCameraMute` | — | CONFIRMED |  |  |
| System Actions | Toggle Energy Saver | `backend/index.mjs` @987025, ext `system`, cmd `toggleEnergySaver` | — | CONFIRMED |  |  |
| System Actions | Toggle Hidden Files | `backend/index.mjs` @987025, ext `system`, cmd `toggleHiddenFiles` | — | CONFIRMED |  |  |
| System Actions | Toggle Mute | `backend/index.mjs` @987025, ext `system`, cmd `toggleMute` | — | CONFIRMED |  |  |
| System Actions | Toggle Stage Manager | `backend/index.mjs` @987025, ext `system`, cmd `toggleStageManager` | — | NOT_APPLICABLE |  | macOS-only |
| System Actions | Toggle HDR | `backend/index.mjs` @987025, ext `system`, cmd `toggleHdr` | — | CONFIRMED |  |  |
| System Actions | Toggle Night Light | `backend/index.mjs` @987025, ext `system`, cmd `toggleNightLight` | — | CONFIRMED |  |  |
| System Actions | Toggle Microphone Mute | `backend/index.mjs` @987025, ext `system`, cmd `toggleMicrophoneMute` | — | CONFIRMED |  |  |
| System Actions | Turn Volume Down | `backend/index.mjs` @987025, ext `system`, cmd `turnVolumeDown` | — | CONFIRMED |  |  |
| System Actions | Turn Volume Up | `backend/index.mjs` @987025, ext `system`, cmd `turnVolumeUp` | — | CONFIRMED |  |  |
| System Actions | Unhide All Hidden Apps | `backend/index.mjs` @987025, ext `system`, cmd `unhideAllHiddenApps` | — | NOT_APPLICABLE |  | macOS-only |
| Translator | Translate | `backend/index.mjs` @1006741, ext `translator`, cmd `translate` | — | NOT_APPLICABLE |  | Raycast servers (excluded) |
| Trial Nudges | Welcome to Pro | `backend/index.mjs` @1011134, ext `trial-nudges`, cmd `welcomePro` | — | NOT_APPLICABLE |  | Pro trial, account (excluded) |
| Trial Nudges | Level up, sign in and start a 7-day free trial | `backend/index.mjs` @1011134, ext `trial-nudges`, cmd `startTrial` | — | NOT_APPLICABLE |  | Pro trial, account (excluded) |
| Trial Nudges | Continue Pro after your trial | `backend/index.mjs` @1011134, ext `trial-nudges`, cmd `chargeReminder` | — | NOT_APPLICABLE |  | Pro trial, account (excluded) |
| Typing Practice | Start Typing Practice | `backend/index.mjs` @1014458, ext `typing-practice`, cmd `startTypingPractice` | — | CONFIRMED |  |  |
| URL | Open in Browser | `backend/index.mjs` @1014972, ext `urls`, cmd `quickOpen` | — | CONFIRMED |  |  |
| Web Searches | Google Search | `backend/index.mjs` @1015437, ext `web-searches`, cmd `google` | — | CONFIRMED |  |  |
| Web Searches | DuckDuckGo Search | `backend/index.mjs` @1015437, ext `web-searches`, cmd `duckDuckGo` | — | CONFIRMED |  |  |
| Web Searches | Bing | `backend/index.mjs` @1015437, ext `web-searches`, cmd `bing` | — | CONFIRMED |  |  |
| Run | Open Run | `backend/index.mjs` @1043114, ext `windows-run`, cmd `windowsRun` | — | CONFIRMED |  |  |
| Run | Execute Run Command | `backend/index.mjs` @1043114, ext `windows-run`, cmd `windowsRunWithArgument` | — | CONFIRMED |  |  |
| Run | Search Run Commands | `backend/index.mjs` @1043114, ext `windows-run`, cmd `searchRunCommands` | — | CONFIRMED |  |  |
| Run | Create Run Command | `backend/index.mjs` @1043114, ext `windows-run`, cmd `createRunCommand` | — | CONFIRMED |  |  |
| Taskbar | Toggle Taskbar Window | `backend/index.mjs` @1046352, ext `windows-taskbar`, cmd `toggleTaskbar` | — | CONFIRMED |  |  |
| Window Management | Maximize, Restore, Minimize, Minimize on This Display, Maximize Height/Width, Almost Maximize, Reasonable Size, Center, Toggle Fullscreen | `backend/index.mjs` @1020541 ext `window-management`; `Raycast.Handlers.WindowManagement` / `VirtualDesktops` (Raycast.dll) | — | CONFIRMED |  |  |
| Window Management | Halves, thirds, two-thirds, fourths, three-fourths, quarters, sixths (~50 layout commands) | `backend/index.mjs` @1020541 ext `window-management`; `Raycast.Handlers.WindowManagement` / `VirtualDesktops` (Raycast.dll) | — | CONFIRMED |  |  |
| Window Management | 60/70/80/90%, Percentage, Resize Window, Make Smaller/Larger (16/32/64/128 px) | `backend/index.mjs` @1020541 ext `window-management`; `Raycast.Handlers.WindowManagement` / `VirtualDesktops` (Raycast.dll) | — | CONFIRMED |  |  |
| Window Management | Move Left/Right/Top/Bottom, Move Window, Move to Previous/Next Display | `backend/index.mjs` @1020541 ext `window-management`; `Raycast.Handlers.WindowManagement` / `VirtualDesktops` (Raycast.dll) | — | CONFIRMED |  |  |
| Window Management | Toggle Always on Top, Focus Next/Previous App Window, Toggle Grid Overlay | `backend/index.mjs` @1020541 ext `window-management`; `Raycast.Handlers.WindowManagement` / `VirtualDesktops` (Raycast.dll) | — | CONFIRMED |  |  |
| Window Management | Create Command, Create Layout, Save Current Layout | `backend/index.mjs` @1020541 ext `window-management`; `Raycast.Handlers.WindowManagement` / `VirtualDesktops` (Raycast.dll) | — | CONFIRMED |  |  |
| Window Management | Switch Desktops, Open/Move to Desktop 1-10, Close Desktop, Rename Desktop | `backend/index.mjs` @1020541 ext `window-management`; `Raycast.Handlers.WindowManagement` / `VirtualDesktops` (Raycast.dll) | — | CONFIRMED |  |  |
| Window Management | Switch to Previous/Next Space | `backend/index.mjs` @1020541 ext `window-management`; `Raycast.Handlers.WindowManagement` / `VirtualDesktops` (Raycast.dll) | — | NOT_APPLICABLE |  | macOS Spaces |
