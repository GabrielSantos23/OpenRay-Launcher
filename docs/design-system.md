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

## Settings window

Measured from the user's General screenshot at 100 % (x and y include the 1 px window border).

| Element | Reference | Openray |
|---|---|---|
| Title bar | 40 px, `#2A2A2A` (fg 5 %), line `#3F3F3F` 2 px above its bottom edge; “Settings” at x=16 | `h-10 bg-titlebar`, `after:bottom-px` line |
| Sidebar | 230 px including a 1 px `#363636` border; padding ≈ 9–10 | `w-[230px] border-sidebar-border px-[9px]` |
| Sidebar search | y 57–88 (32 px), `#363636`, radius ≈ 6 | `h-8 bg-field rounded-md` |
| Nav item | 32 px, 22 px icon, text at x≈46, selection fg 5 % | `h-8 gap-2 px-1.5 bg-nav-selection` |
| Content inset | 16 px from sidebar and title bar (`--content-padding-x`) | `p-4` |
| Card | fg 2.5 % (`#242424` on `#1F1F1F`), radius ≈ 8 | `bg-card rounded-lg` |
| Row | 46 px + 1 px separator `#303030`; padding 12 | `min-h-[47px] px-3 border-card-separator` |
| Switch | 28×16; on = system accent (`#E2241A` here, `AccentPalette[3]`), white knob ≈ 10 px; off ring fg ≈ 25 % | `w-7 h-4 bg-switch-on` |
| Button | 24 px, `#3A3A3A` (fg 10 % on the card) | `h-6 bg-control` |
| Commands table | Alias column at x=1073, Hotkey at x=1181, checkbox right edge 12 px from the card | columns `108px` / `133px` / `40px` |

## Theme-derived colors

From `css-C58pR_1e.css` and the theme CSS builder in `contrast-improvement-CQ0EgiTy.js`. The window background is `--bg-40` over the Windows backdrop (`#333` dark, `#CCC` light, opaque variant). For Raycast Dark that gives `#1F1F1F`, the value measured earlier.

| Token | Rule | Openray |
|---|---|---|
| Window | `linear-gradient(bg-40, bg-secondary-40)` over the backdrop | `--color-bg` → `--color-bg-bottom` on `html` |
| Popover / menu | `#FFFFFF1A` overlay over `bg-secondary-40` over `#333` / `#E6E6E6` | `--color-panel`; border, separator and selection mix fg 20/10/10 % |
| Surfaces | `--fg-5`, `--fg-10`, `--fg-2.5` etc. relative to the theme foreground | `rgb(from var(--color-fg) …)` tokens |
| Accent | system accent, else theme accent `#4FA3F8` | `--color-accent` |
| Recorder tint | conflict red 15 % → 8 % → 0, saved green likewise | `recorder-conflict`, `recorder-saved` utilities |
| Recorder key | 24 px, min-width 24, padding 0 6, fg 10 %, radius 4, inset top/bottom half-pixel highlights | `KeyCap` |
| Table hotkey | button max 145 × 24, padding 6, keycaps 20 px high, min 16 wide, 1 px apart; clear icon 12 px | `HotkeyRecorder variant="cell"` |
| Dropdown trigger | ghost: 32 px, padding 12/8, gap 8; small: 24 px, padding 8/6 | `Dropdown` |
| Menu | search field 40 px, padding 16, separator; items like the action panel; max height 280 | `Dropdown` |
| Scrollbar | thumb 6 px fg 40 % rounded; 11 px + fg 5 % track on hover | `::-webkit-scrollbar` 10 px with 2 px transparent border |

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
