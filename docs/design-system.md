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
| Outer size (incl. 1 px border) | 750 × 475 | H: `GetWindowRect` on `HwndWrapper[Raycast;Main;…]`; C: `--main-window-width: 750·spx` | Popup window (no caption or frame), client 750×475; the DWM 1 px border overlays the outermost pixel, as in the reference (smoke test) |
| Border | 1 px `#3B3B3B` | M | `DWMWA_BORDER_COLOR` 0x3B3B3B |
| Corner | DWMWCP_ROUND | H: `DWMWA_WINDOW_CORNER_PREFERENCE`=2 | Same |
| Background | `#1F1F1F` opaque composite | M | `--color-bg` |
| Position | (627,381) observed; rule unknown | H, `unk_341f…` | ≈ horizontally centered; top at 35.3% of work-area height (gives y=381 on 1080 px) |
| Scale at >100% DPI | unknown | — | ≈ 750×475 × DPI scale (`launcher::init`) |
| Hidden state | Mapped, layered alpha 0, click-through, no-activate, not topmost | H (show/hide probe) | Same, via `SetLayeredWindowAttributes` + ex-style; `DWMWA_TRANSITIONS_FORCEDISABLED` |

## Layout (y offsets from the outer top edge)

| Element | Reference | Source | Openray |
|---|---|---|---|
| Search header | 64 px content, plus a 1 px separator at y=65 | M; C: `--main-window-header-height: 64·spx` | `--spacing-header: 66px` (hidden border row + 64 + separator, border-box) |
| Section header | 32 px (y 66–97) | M | `h-8`; text bottom padding 5 px ≈ |
| Row | 38 px, 8 px from the outer edge on each side, radius ≈ 8 px | M | `--spacing-row: 38px`, `--spacing-inset: 8px`, `--radius-row: 8px` ≈ |
| Row icon | 22 px, 8 px from the row's left edge; title 12 px after the icon | M; C: `--icon-22` | Same |
| Footer | 40 px content, separator above at y=433 | M (C says 42·spx; measured 40) | `--spacing-footer: 42px` (separator + 40 + hidden border row) |
| Search glyph | x=16 from the outer edge | M | `--spacing-search-inset: 16px` |
| Footer Actions button | x 634–740, y 441–466 (26 px), radius ≈ 6, text 8 px from its left, keycaps 3 px from its right; fg 10 % fill while the panel is open; label fg-60 | M (user screenshots) | `h-[26px] pl-2 pr-[3px] rounded-[6px] text-fg-60`, `pb-px` on the footer |
| Footer divider | 1 px × 11 px at x=627, fg ≈ 20 %; hidden while the panel is open | M | `--color-footer-divider` |
| Keycap | 18 px tall, 1 px border fg ≈ 18 %, transparent fill, 11 px fg-60 label, 2 px between caps | M | `--color-key-border`, `h-[18px] min-w-[18px] rounded-[4px]` (radius ≈) |

## Action panel (Ctrl+K)

Measured from user-supplied screenshots of Raycast 2.8.2.0 at 100 % scale (750×475 window).

| Element | Reference | Openray |
|---|---|---|
| Frame | 350 px wide incl. 1 px border `#5E5E5E`; right edge 8 px from the window edge; bottom border at y=423 (9 px above the footer separator); radius ≈ 6 | `w-[350px] right-2 bottom-[51px]`, `--color-panel-border`, `--radius-panel: 6px` ≈ |
| Fill | `#363636` with a faint noise texture | `--color-panel` (no noise) |
| List | 7 px padding inside the border at top and sides, 8 px at the bottom; max 239 px tall, then scrolls with no visible scrollbar | `px-[7px] pt-[7px] pb-2 max-h-[239px]` |
| Title | 24 px row, 11 px ≈ semibold fg-60, text at x=408 | `h-6 px-2 text-11` |
| Row | 32 px, inset 8 px from the border, selection `#4A4A4A`; 16 px icon at +8, title at +32, keycaps 8 px from the right | `h-8 px-2 gap-2`, `--color-panel-selection` |
| Group separator | 1 px `#4A4A4A`, full panel width, 8 px above and 7 px below | `-mx-[7px] mt-2 mb-[7px]`, `--color-panel-separator` |
| Search | separator at y=385, 37 px input, placeholder at x=408, 13 px fg-40 | `h-[38px] border-t px-[15px]` |
| Destructive item | text `#FF6363` | `--color-destructive` |
| Shadow | faint, ≈ | `--shadow-panel` ≈ |

## Toasts, HUD and loader

| Element | Reference | Openray |
|---|---|---|
| Footer toast dot | 5 px, center (22, 454), glow `0 0 12px 8px` color-20 % | `ActivityIndicator`, `size-[5px]` with shadow |
| Footer toast title | x=37, 12 px regular, white | `text-12`, `gap-[7px]` after the 16 px indicator slot |
| Footer toast glow | 10000×400 px radial band, rotated 20°, opacity 0.2, center 80/50 px into the toast container, clipped to the footer including its separator | `toast-glow` utility at `left-[88px] top-[48px]` ≈; separator drawn as a `-z-20` layer so the glow tints it |
| HUD frame | 46 px tall (56 with message), padding 12, gap 12, start 16 with an icon (24 without), end 24, max width 500; border `#414141`; base `#1F1F1F` | `hud` window, `DWMWCP_ROUNDSMALL` ≈, DWM border `0x414141` |
| HUD glow | two radial gradients from the top center, variant color at 26 % | `hud-glow` utility |
| HUD position | unknown (`unk_3a8f3266…`) | ≈ centered on the cursor's monitor, bottom edge 12 % of the work-area height above its bottom |
| Loader | 1 px `linear-gradient(to right, transparent 25%, #FFF, transparent 75%)`, ±35 % over 1.5 s ease-in-out, fade 0.3 s, 300 ms delay | `loader-gradient animate-loader`, overlays the search separator (`-bottom-px`) ≈ |
| Status colors | green `#59D499`, yellow `#FFC531`, red `#FF6363` (theme `bundled-raycast-dark`) | `--color-green`, `--color-yellow`, `--color-red` |

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
- the hover color on the live app;
- text baselines to the pixel.
