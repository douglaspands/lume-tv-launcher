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

## Selectors

Only the components below, with at most one pseudo-class: `:focus` (D-pad focus), `:active` (selected) and `:disabled`.

`.app` `.topbar` `.tab` `.clock` `.icon-button` `.hero` `.hero-badge` `.hero-title` `.hero-subtitle` `.button` `.pager-dot` `.section-title` `.tile` `.tile-label` `.editor-row` `.chip` `.panel` `.breadcrumb` `.settings-item` `.toast`

```css
.tab { color: rgba(255, 255, 255, 0.7); }
.tab:active { color: #FFFFFF; }
.tab:focus { outline: 3px solid #FFFFFF; }
.button:disabled { opacity: 0.4; }

/* Selector list */
.chip, .button { border-radius: 12px; }
```

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
.hero {
  background: linear-gradient(135deg, #1A1040 0%, #3B1F6E 60%, #12081F 100%);
}
.hero-badge { background: linear-gradient(to right, #4DB3FF, #7C5CFF); }
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

## What is not supported

```css-invalid
@import url(https://example.com/a.css);   /* warning: ignored, no network access */
.hero { background: url(image.png); }      /* warning: url() is not supported */
.tile { filter: blur(4px); }               /* warning: unsupported property */
.tile { font-size: 12px; }                 /* warning: too small to read on a TV */
.tile { color: var(--does-not-exist); }    /* error: undefined variable */
.tile { border-radius: 12; }               /* error: missing the px unit */
.tile { color: white;                      /* error: block without a closing brace */
```
