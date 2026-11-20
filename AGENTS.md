\# Openray — Agent Guide



\## What this is



Openray is a Windows clone of Raycast, built as a portfolio project and started from zero. It is an independent implementation that targets Windows 10 and 11.



The reference is \*\*Raycast for Windows 2.8.2.0\*\*, which is installed on the dev machine. Earlier versions of this guide named 2.4.0.0; see the version note in `docs/reverse-engineering-notes.md`.



\## Scope



\- \*\*Goal:\*\* maximum observable visual and functional parity with the reference.

\- \*\*Excluded:\*\*

&nbsp; - All Raycast AI features (chat, agents, AI commands, model integrations).

&nbsp; - Features that depend on Raycast servers or accounts.

\- \*\*Raycast Store and compatibility with original Raycast extensions are out of scope.\*\* A previous attempt broke on AI and macOS-only APIs. Built-in features ship as internal extensions on the project's own extension API.



\## Fixed decisions



\- \*\*Stack:\*\* Rust + egui (eframe, wgpu renderer since 2026-11-19) with no WebView and no TypeScript in the main app; third-party extensions in TypeScript on an embedded JS engine. See ADR-002 in `docs/architecture.md`. The Tauri 2 app (ADR-001) was removed in Phase 5 (2026-11-13; last present at commit 2925cc8). What it had and the egui app lacks is listed in `docs/known-limitations.md`.

\- \*\*Extension API:\*\* built-in features implement the `Extension` trait in `app/src/extension.rs`; third-party FlowKey extensions run on QuickJS through `app/src/flowkey_host.rs` (ADR-002).



Once a decision is listed here, do not reopen it. If a task seems to require changing one, stop and ask.



\## Reference and evidence



\- The installed Raycast for Windows 2.4.0.0 is the source of truth for behavior and visuals. Screenshots attached to a task are authoritative for layout.

\- Use REA to inspect the installed app:

&nbsp; - Start by checking which analysis providers are actually available.

&nbsp; - Inspect binaries, resources and bundled UI code. Look for stack and runtime, settings keys, defaults, option lists, component structure and assets.

\- Record findings in `docs/reverse-engineering-notes.md`. Each finding needs evidence: a file, a symbol or address, or a screenshot.

\- Record open questions with REA's unknowns tools. Do not guess.

\- Never invent measurements. Anything not measured is marked as an approximation.

\- Reproduce observable behavior and appearance. Do not copy proprietary source code, and do not ship assets extracted from Raycast.



\## Tools



\- \*\*REA\*\* (MCP): use it for reference investigation, as described above.

\- \*\*Graphify\*\* (optional, if installed):

&nbsp; - Query it before adding a module, to find existing implementations and affected tests.

&nbsp; - Regenerate the graph after structural changes.

&nbsp; - Treat its answers as hints and confirm them in source.

\- \*\*Simplicity (Ponytail principles)\*\*:

&nbsp; - Prefer the standard library, platform APIs and existing dependencies.

&nbsp; - No speculative abstractions.

&nbsp; - Never cut required behavior to reduce code.

\- \*\*Project skills\*\* (`.claude/skills/`): at most `raycast-reference-investigation` and `visual-parity-check`. Create one only when the task actually repeats.



Do not spend a session on tool setup. If a tool is unavailable, note it in `docs/tooling-status.md` and continue.



\## Docs to keep current



\- `docs/feature-matrix.md` has one row per feature, with these columns: reference evidence, Windows behavior, status, tests, remaining differences.

&nbsp; - Statuses: `NOT\_INVESTIGATED`, `INVESTIGATING`, `CONFIRMED`, `IMPLEMENTING`, `IMPLEMENTED`, `TESTED`, `BLOCKED`, `NOT\_APPLICABLE`.

\- `docs/reverse-engineering-notes.md`

\- `docs/architecture.md`: stack ADR, module boundaries, extension API.

\- `docs/design-system.md` holds tokens measured from the reference, with approximations marked as such.

\- `docs/known-limitations.md`

\- `docs/tooling-status.md`



Update the matrix in the same change as the code.



\## Working alongside other agents



\- Until the kickoff task is done (stack chosen, skeleton running), only one agent works on the repo.

\- After that, other agents may edit the repo at the same time:

&nbsp; - Run `git status` before starting and before finishing. Never revert or overwrite changes you did not make.

&nbsp; - Touch only the files your task needs. Do not reformat, rename or move anything outside your scope.

&nbsp; - In shared files (feature matrix, command registry, design tokens), edit only your own entries.

&nbsp; - Do not rewrite git history.

&nbsp; - End with a list of the files you touched.



\## Code conventions



\- No inline comments.

\- If the UI is web-based: use Tailwind utilities, not inline styles, and define colors as CSS variables.

\- Dark theme first. Add light theme where the reference supports it.

\- Only real actions. No static demo lists and no fake results. Anything unfinished is marked in the feature matrix.



\## Definition of done



\- The app builds and runs on Windows.

\- Relevant tests pass, and new logic has tests.

\- UI work has been compared side by side with the reference, at the same DPI and in the same state.

\- The feature matrix and known limitations are updated.

\- The final report covers:

&nbsp; - what changed

&nbsp; - what is verified vs. estimated

&nbsp; - open unknowns

&nbsp; - files touched

