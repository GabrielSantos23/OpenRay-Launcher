# Design system

Tokens are measured from Raycast for Windows 2.8.2.0 on a 1920×1080 display at 96 DPI (100% scale), dark theme.

- **Sources:**
  - **M**: screenshot pixel measurement. Files: `ref2-full.png` (reference) and `e2e-v2.png` (Openray), from the kickoff scratchpad.
  - **H**: Win32 window measurement (`GetWindowRect`, `DwmGetWindowAttribute`).
  - **C**: the reference CSS (`frontend/css-C58pR_1e.css`, `main-window-38KZXJcp.css`).
  - **≈**: an approximation, not measured.
- **Code:** tokens live in `src/styles.css` (`@theme`) as CSS variables.

## Launcher window

| Token | Value | Source | Openray |
|---|---|---|---|
| Outer size (incl. 1 px border) | 750 × 475 | H: `GetWindowRect` on `HwndWrapper[Raycast;Main;…]`; C: `--main-window-width: 750·spx` | Client 748×473 plus a DWM 1 px border = 750×475 frame (smoke test) |
| Border | 1 px `#3B3B3B` | M | `DWMWA_BORDER_COLOR` 0x3B3B3B |
| Corner | DWMWCP_ROUND | H: `DWMWA_WINDOW_CORNER_PREFERENCE`=2 | Same |
| Background | `#1F1F1F` opaque composite | M | `--color-bg` |
| Position | (627,381) observed; rule unknown | H, `unk_341f…` | ≈ horizontally centered; top at 35.3% of work-area height (gives y=381 on 1080 px) |
| Scale at >100% DPI | unknown | — | ≈ logical pixels × DPI scale (Tauri default) |

## Layout (y offsets from the outer top edge)

| Element | Reference | Source | Openray |
|---|---|---|---|
| Search header | 64 px content, plus a 1 px separator at y=65 | M; C: `--main-window-header-height: 64·spx` | `--spacing-header: 65px` (64 + separator, border-box) |
| Section header | 32 px (y 66–97) | M | `h-8`; text bottom padding 5 px ≈ |
| Row | 38 px, 8 px from the outer edge on each side, radius ≈ 8 px | M | `--spacing-row: 38px`, `--spacing-inset: 7px` (client starts inside the border), `--radius-row: 8px` ≈ |
| Row icon | 22 px, 8 px from the row's left edge; title 12 px after the icon | M; C: `--icon-22` | Same |
| Footer | 40 px content, separator above at y=433 | M (C says 42·spx; measured 40) | `--spacing-footer: 41px` (40 + separator) |
| Search glyph | x=16 from the outer edge | M | `--spacing-search-inset: 15px` |

## Color

| Token | Value | Source |
|---|---|---|
| `--color-fg` | `#FFFFFF` ≈ | Inferred: the measured 10 % overlay `#363636` equals white at 10 % over `#1F1F1F` |
| `--color-fg-60` | fg 60 % | C: `--color-text-secondary` |
| `--color-fg-40` | fg 40 % | C: `--color-text-placeholder` |
| `--color-separator` | fg 10 % → `#363636` | C: `--color-border-separator`; M |
| `--color-list-selection` | fg 10 % → `#363636` | C: `--color-surface-list-selection`; M |
| `--color-list-hover` | fg 5 % | C: `--color-surface-list-hover` |

## Type

| Token | Value | Source |
|---|---|---|
| Family | Inter Variable, then Segoe UI Variable | C: `@font-face InterVariable` (we ship `@fontsource-variable/inter`, OFL, not Raycast's file) |
| Search input | 18 px ≈ | C has `--font-size-18`; size chosen by visual match, not measured |
| Row title / subtitle / accessory | 13 px ≈, title weight 500 | C: `--font-size-13`; visual match |
| Section header | 11 px ≈, weight 600, fg 60 % | C: `--font-size-11`; visual match |

Not yet measured:

- light theme;
- compact mode;
- show and hide animation;
- the action panel;
- the hover color on the live app;
- text baselines to the pixel.
