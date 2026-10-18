# Tooling status

Recorded 2026-10-18 during the kickoff task.

## Environment

| Item | Value |
|---|---|
| OS | Windows 11 Pro 10.0.26200, x64 (AMD64) |
| Display (dev machine) | 1 monitor, 1920x1080, 96 DPI (100%) |
| Shells | Windows PowerShell 5.1.26100; Git Bash |
| Node | v24.20.0 |
| npm / pnpm / bun | 11.4.2 / 12.4.1 / 1.4.2 (yarn not installed) |
| Python / uv | 3.13.5 / 0.10.2 |
| Rust | rustc 1.91.0, cargo 1.91.0, toolchain stable-x86_64-pc-windows-msvc |
| Git | 2.50.1.windows.1 |
| .NET SDK | 9.0.302 |
| MSVC | Visual Studio 2022 Community, VS 2019 Build Tools |
| WebView2 Runtime | 155.0.4283.62 |
| Package managers (system) | choco present; winget, scoop not on PATH |

## Tools

| Tool | Status | Verification step | Notes |
|---|---|---|---|
| REA (MCP) | `INSTALLED_AND_VERIFIED` | `inspect_managed_artifact` on `Raycast.dll` returned evidence `ev_8bffd5e6…`; `record_unknown` created 6 unknowns | Providers that worked: managed PE/CLI metadata, SQLite inspector (rejected encrypted DBs correctly), unknowns ledger. Not exercised: native disassembly (Ghidra/Hopper/rizin), CDP web bundle capture (needs a debuggable WebView2 target). |
| Graphify | `INSTALLED_AND_VERIFIED` | `graphify --version` → `graphify 0.9.92`; skill file at `~/.claude/skills/graphify/SKILL.md`; MCP tools `mcp__graphify__*` listed in session | Repo was empty at kickoff, so no graph was built. |
| Agent Skills validator (`skills-ref` / `agentskills`) | `PARTIALLY_CONFIGURED` | `uvx --from skills-ref agentskills --help` printed usage | Not installed; runs on demand from the uv cache. No project skills exist yet. |
| Ponytail | `INSTALLED_AND_VERIFIED` | SessionStart hook printed "PONYTAIL MODE ACTIVE — level: full"; plugin present under `~/.claude/plugins` | |
| Better Design (MCP) | `INSTALLED_AND_VERIFIED` | Tools listed in session; used for UI principles and review | |
| Tauri CLI | `INSTALLED_AND_VERIFIED` | Installed per-project as `@tauri-apps/cli` devDependency; `npx tauri --version` | Not installed globally. |

Nothing was installed system-wide.
