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
| `--on-primary` | text and icons drawn on top of `--primary` | `#0F171E` |
| `--secondary` | the neutral light color: text, and the tab underline when the focus is in the content | `#F2F5F7` (near white) |

`--text` follows `--secondary`. The older names `--accent` and `--on-accent` still work and are aliases of
`--primary` and `--on-primary`. Other variables (`--bg`, the `--surface-*` shades, `--text-muted`, `--danger-*`) keep
their own values.

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

## Selectors

Only the components below, with at most one pseudo-class: `:focus` (D-pad focus), `:active` (selected) and `:disabled`.

`.app` `.topbar` `.tab` `.clock` `.icon-button` `.button` `.section-title` `.tile` `.tile-label` `.dock` `.dock-label` `.editor-row` `.chip` `.panel` `.breadcrumb` `.settings-item` `.toast`

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
| `font-size` | a length (below 14px produces a warning) |
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

```css
.section-title { font-size: 22px; font-weight: 700; letter-spacing: 0.2px; }
.tile-label { font-size: 16px; font-weight: 500; padding: 8px 12px; gap: 6px; }
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
.dock-label { padding: 8px 18px; border-radius: 12px; background: rgba(15, 23, 30, 0.9); color: #F2F5F7; font-size: 18px; font-weight: 700; }
```

## What is not supported

```css-invalid
@import url(https://example.com/a.css);   /* warning: ignored, no network access */
.panel { background: url(image.png); }     /* warning: url() is not supported */
.tile { filter: blur(4px); }               /* warning: unsupported property */
.tile { font-size: 12px; }                 /* warning: too small to read on a TV */
.tile { color: var(--does-not-exist); }    /* error: undefined variable */
.tile { border-radius: 12; }               /* error: missing the px unit */
.tile { color: white;                      /* error: block without a closing brace */
```
