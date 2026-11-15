# lume theme format

A lume theme is a CSS file that uses a small **subset** of the language. There are no scripts, no `@import`, no URLs and no network access. The default theme is `themes/lume/theme.css`, compiled into the app by the `build.rs` of `lume-core`.

Anything unsupported produces a **warning** (file, line and reason) and is ignored. Invalid syntax, invalid values and undefined variables produce an **error** and the build fails.

The ` ```css ` blocks in this document are checked by the `docs_examples_parse_without_diagnostics` test; the ` ```css-invalid ` blocks show what produces a diagnostic.

## Variables

Variables only exist in `:root` and are used with `var()`, with an optional fallback value:

```css
:root {
  --accent: #4DB3FF;
  --radius-tile: 14px;
  --focus-ring: 3px solid #FFFFFF;
}

.tile {
  border-radius: var(--radius-tile);
  outline: var(--focus-ring);
  color: var(--text-muted, rgba(255, 255, 255, 0.7));
}
```

## Color roles

The default theme names its colors by the role they play, so a new theme mostly changes a few variables:

| Variable | Role | In the default theme |
|---|---|---|
| `--primary` | the color of focus and control: focus outlines, the tab underline while the top bar has the focus, the settings button, main actions | `#4DB3FF` (blue) |
| `--on-primary` | text and icons drawn on top of `--primary` | `var(--bg)` (`#0C171F`) |
| `--secondary` | the neutral light color: text, and the tab underline when the focus is in the content | `#F1F4F9` (near white, `neutral` T96) |

`--text` follows `--secondary`. The older names `--accent` and `--on-accent` still work and are aliases of
`--primary` and `--on-primary`. Other variables (`--bg`, the `--surface-*` shades, `--text-muted`, `--danger-*`) keep
their own values, generated from tonal palettes (below).

## Tonal palettes

Every neutral and danger color of the default theme is a tone of a tonal palette (Material Design 3, HCT color space):
a palette is a hue and a chroma, and a tone is a lightness from 0 (black) to 100 (white). The theme declares its
palettes in a comment of `:root`, one `@palette <name> <hue> <chroma>` per line, and each generated token names its
palette and tone in a comment right after its value:

```css
:root {
  /* @palette surface 245 12
     @palette surface-variant 246 18
     @palette neutral 240 6
     @palette error 25 84 */
  --bg: #0C171F; /* surface T7 */
  --text-muted: #B5B9BE; /* neutral T75 */
  --danger-bg: #690005; /* error T20 */
}
```

| Palette | Hue, chroma | Tokens (tone) |
|---|---|---|
| `surface` | 245, 12 | `--bg` 7, `--surface` 10, `--surface-row` 13, `--surface-icon` 14, `--menu-text` 15, `--control-disabled` 16 |
| `surface-variant` | 246, 18 | `--surface-focus` 19, `--control` 24, `--border` 27 |
| `neutral` | 240, 6 | `--secondary` and `--menu-surface` 96, `--text-soft` 90, `--text-muted` 75, `--menu-text-muted` 44, `--text-disabled` 41 |
| `error` | 25, 84 | `--danger-bg` 20, `--menu-danger` 40, `--danger-fg` 80 |

The build reads only the hex; the comments are for the generator. Do not edit those hex by hand: change the palette or
the tone and run, from the repository root,

```
uv run scripts/tonal-palette.py           # rewrites the hex of themes/lume/theme.css
uv run scripts/tonal-palette.py --check   # only compares; exits with 1 if a hex is out of date
```

(another theme file can be passed as the last argument). `--primary` is a fixed seed and is not generated, and the
brand colors of the tiles (`--brand-*`, `--tile-*`, `--dot-*`) stay outside the palettes.

**Contrast by tone.** The tone difference sets the contrast: a text at least 50 tones away from its background has at
least 4.5:1, and an outline or graphic at least 40 tones away has at least 3:1. The theme test
`the_pairs_of_the_components_keep_the_tone_distance` checks the pairs the components use (`--text-disabled` is left out).

**Material Design 3 roles.** The theme also has the M3 role names, as aliases of the lume tokens; a new component picks
its color by role:

| M3 role | lume token |
|---|---|
| `--surface-container-lowest` | `--bg` |
| `--surface-container-low` | `--surface` |
| `--surface-container` | `--surface-row` |
| `--surface-container-high` | `--surface-focus` |
| `--on-surface` | `--text` |
| `--on-surface-variant` | `--text-muted` |
| `--outline` | `--border` |
| `--error` | `--menu-danger` |
| `--error-container` | `--danger-bg` |
| `--on-error-container` | `--danger-fg` |
| `--inverse-surface` | `--menu-surface` |
| `--inverse-on-surface` | `--menu-text` |

A variable can use another one (`--accent: var(--primary);`), so a theme can set the roles once and derive the rest:

```css
:root {
  --primary: #7C5CFF;
  --on-primary: #FFFFFF;
  --secondary: #FFFFFF;
  --accent: var(--primary);
  --focus-ring: 4px solid var(--primary);
}
.tile:focus { outline: var(--focus-ring); }
```

## Edges

Every shape and every clipped image (tile corners, round favorites, focus outlines, round buttons) is antialiased by
the shaders over one ramp whose width is `--edge-softness`, and the icons of the icon font (inputs, gear...) get the
same ramp when they are rasterized; text keeps the font's own edges. Unlike the other lengths, it is in pixels of the
render buffer, not of the 1280x720 canvas, so an edge looks the same at 720p and 1080p. Keep it between 1 and 1.5:
under 1 the edges stair-step once the TV upscales and sharpens the picture, over 1.5 they look blurred. The default is
1.5, chosen on the 4K panel of the reference TV.

```css
:root {
  --edge-softness: 1.5px;
}
```

## Selectors

Only the components below, with at most one pseudo-class: `:focus` (D-pad focus), `:active` (selected) and `:disabled`.

`.app` `.topbar` `.brand` `.tab` `.clock` `.icon-button` `.button` `.button-secondary` `.button-danger` `.section-title` `.tile` `.tile-label` `.dock` `.dock-label` `.editor-row` `.chip` `.panel` `.breadcrumb` `.settings-item` `.menu` `.menu-item` `.menu-note` `.menu-button` `.menu-button-danger` `.keycap` `.toast` `.page-title` `.page-subtitle` `.card-heading` `.about-heading` `.about-body` `.about-license`

`.brand` is the "Lume" wordmark in the top bar; `.page-title` and `.page-subtitle` are the header of secondary screens; `.card-heading` is the uppercase heading inside a card (TV statistics, favorites order); `.about-heading`, `.about-body` and `.about-license` are the three kinds of text of the About screen. No screen sets a font size, weight or letter spacing in code: they all come from these components.

```css
.tab { color: rgba(255, 255, 255, 0.7); }
.tab:active { color: #FFFFFF; }
.tab:focus { color: #FFFFFF; }
.icon-button:focus { outline: 3px solid #4DB3FF; }
.tile:focus { outline: 4px solid #4DB3FF; }
.button:disabled { opacity: 0.4; }

/* Selector list */
.chip, .button { border-radius: 12px; }
```

The top bar draws no outline around a focused tab: `.tab:focus` only changes the text. The underline (`--tab-underline` thick) sits under the focused tab while the focus is on the tabs, and under the active tab otherwise; it is the accent blue while the focus is in the top bar and white when it goes down to the content; moving the focus onto a tab also shows that tab, with no OK needed. The settings button (`.icon-button:focus`) uses the outline.

Combinators (`.a .b`, `>`), `#id`, tags, `*` and other pseudo-classes produce a warning:

```css-invalid
div { color: white; }       /* warning: unsupported selector */
.tile:hover { opacity: 1; } /* warning: unsupported pseudo-class */
```

## Properties

Lengths are always in `px` of the 1280×720 logical canvas (or `0`).

| Property | Value |
|---|---|
| `background` | a color or `linear-gradient(...)` |
| `color` | a color |
| `opacity` | a number from 0 to 1 |
| `border-radius` | a length |
| `outline` | `<width> solid <color>` or `none` |
| `outline-offset` | a length |
| `box-shadow` | `<x> <y> <blur> [<spread>] <color>` (one shadow) or `none` |
| `font-size` | a length on the type scale: 16, 18, 20, 22, 28 or 40px (any other size produces a warning and is drawn at the nearest one; below 16px is too small for a TV) |
| `font-weight` | an integer (e.g. 400, 700) |
| `letter-spacing` | a length |
| `padding` | 1 to 4 lengths |
| `gap` | a length |
| `width`, `height` | a length |
| `transform` | only `scale(<n>)` |
| `transition` | `<duration> [<curve>]` |

### Colors

`#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa`, `rgb(r, g, b)`, `rgba(r, g, b, a)`, `transparent`, `white` and `black`.

```css
.panel { background: rgba(255, 255, 255, 0.06); color: #fff; }
.toast { background: rgb(20 30 40 / 90%); }
```

### Gradients

`linear-gradient([<angle>deg | to <side>,] <color> [<position>%], ...)` with 2 or 3 stops. Without an angle the default is 180deg (top to bottom). Stops without a position are spread evenly.

```css
.panel {
  background: linear-gradient(135deg, #1A1040 0%, #3B1F6E 60%, #12081F 100%);
}
.chip { background: linear-gradient(to right, #4DB3FF, #7C5CFF); }
```

### Focus, shadow and animation

```css
.tile {
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.35);
  transition: 150ms ease-out;
}
.tile:focus {
  transform: scale(1.06);
  outline: 3px solid #FFFFFF;
  outline-offset: 2px;
  box-shadow: 0 16px 40px rgba(0, 0, 0, 0.5);
  transition: 200ms cubic-bezier(0.2, 0, 0, 1);
}
```

Curves: `linear`, `ease`, `ease-in`, `ease-out`, `ease-in-out` and `cubic-bezier(x1, y1, x2, y2)`. Durations in `ms` or `s`.

### Typography and spacing

The type scale has six even sizes, so the text lands on whole pixels at 1.5x (1080p). The default theme names them as variables, and every text component uses one:

```css
:root {
  --font-title: 40px;   /* screen title */
  --font-brand: 28px;   /* "Lume" and the clock in the top bar */
  --font-primary: 22px; /* tabs, sections, settings items, tiles, editor rows, About headings */
  --font-body: 20px;    /* base text, About body */
  --font-label: 18px;   /* buttons, dock bubble, toast, subtitle, breadcrumb, tile label, chip, panel */
  --font-caption: 16px; /* third-party licenses, card headings */
}
.page-title { font-size: var(--font-title); font-weight: 800; letter-spacing: -0.8px; }
.card-heading { font-size: var(--font-caption); font-weight: 800; letter-spacing: 1.2px; }
```

Lengths follow the 4-unit ruler of the design system (`openspec/specs/design-system`, "Régua de layout"): positions, sizes, padding, gaps and radii are multiples of 4.

```css
.section-title { font-size: 22px; font-weight: 700; letter-spacing: 0.2px; }
.tile-label { font-size: 18px; font-weight: 500; padding: 8px 12px; gap: 8px; }
.button { width: 160px; height: 48px; }
```

### Dock

The favorites on the Home screen sit in a floating bar. `.dock` styles the bar and `.dock-label` the bubble with the name of the focused app. How big the icons are, how much they grow when focused (the Magnification) and the spacing are fixed in the code, not in the theme.

```css
.dock {
  border-radius: 28px;
  background: rgba(15, 23, 30, 0.55);
  outline: 1px solid rgba(255, 255, 255, 0.14);
  box-shadow: 0 16px 40px rgba(0, 0, 0, 0.45);
}
.dock-label { padding: 8px 16px; border-radius: 12px; background: rgba(0, 0, 0, 0); color: #F1F4F9; font-size: 18px; font-weight: 700; }
```

## What is not supported

```css-invalid
@import url(https://example.com/a.css);   /* warning: ignored, no network access */
.panel { background: url(image.png); }     /* warning: url() is not supported */
.tile { filter: blur(4px); }               /* warning: unsupported property */
.tile { font-size: 12px; }                 /* warning: too small to read on a TV */
.tile { font-size: 17px; }                 /* warning: not on the type scale, drawn at 18px */
.tile { color: var(--does-not-exist); }    /* error: undefined variable */
.tile { border-radius: 12; }               /* error: missing the px unit */
.tile { color: white;                      /* error: block without a closing brace */
```
