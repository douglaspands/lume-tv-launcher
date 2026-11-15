# /// script
# requires-python = ">=3.10"
# dependencies = ["materialyoucolor==3.0.4"]
# ///
"""Rewrite the tonal colors of a lume theme from its declared palettes.

The theme declares its palettes in a comment, one per line:

    @palette surface 245 12      (name, HCT hue, HCT chroma)

and each generated token names its palette and tone in a comment after the value:

    --bg: #0C171F; /* surface T7 */

This script computes every tone with the Material Color Utilities (HCT) and writes the hex back.
With --check it only reports the tokens whose hex differs and exits with 1 if there is any.

usage: uv run scripts/tonal-palette.py [--check] [THEME.css]   (default: themes/lume/theme.css)
"""

import re
import sys
from pathlib import Path

from materialyoucolor.palettes.tonal_palette import TonalPalette

PALETTE = re.compile(r"@palette\s+([a-z][a-z0-9-]*)\s+(\d+(?:\.\d+)?)\s+(\d+(?:\.\d+)?)")
TOKEN = re.compile(
    r"(?P<head>--[a-z0-9-]+:\s*)#(?P<hex>[0-9A-Fa-f]{6})(?P<mid>;\s*/\*\s*)"
    r"(?P<palette>[a-z][a-z0-9-]*)\s+T(?P<tone>\d+)(?P<tail>\b[^*]*\*/)"
)


def tone_hex(palette: TonalPalette, tone: int) -> str:
    return f"{palette.tone(tone) & 0xFFFFFF:06X}"


def main() -> int:
    args = [a for a in sys.argv[1:] if a != "--check"]
    check = "--check" in sys.argv[1:]
    path = Path(args[0]) if args else Path(__file__).resolve().parent.parent / "themes/lume/theme.css"
    source = path.read_text()

    palettes = {
        name: TonalPalette.from_hue_and_chroma(float(hue), float(chroma))
        for name, hue, chroma in PALETTE.findall(source)
    }
    if not palettes:
        print(f"{path}: no @palette declared", file=sys.stderr)
        return 2

    errors: list[str] = []
    changed: list[str] = []

    def replace(m: re.Match) -> str:
        name, tone = m["palette"], int(m["tone"])
        if name not in palettes:
            errors.append(f"{m['head'].strip()} unknown palette '{name}'")
            return m[0]
        want = tone_hex(palettes[name], tone)
        if want != m["hex"].upper():
            changed.append(f"{m['head'].strip()} #{m['hex']} -> #{want} ({name} T{tone})")
        return f"{m['head']}#{want}{m['mid']}{name} T{tone}{m['tail']}"

    out = TOKEN.sub(replace, source)
    count = len(TOKEN.findall(source))
    for e in errors:
        print(f"{path}: {e}", file=sys.stderr)
    if errors:
        return 2
    for c in changed:
        print(c)
    if check:
        print(f"{count} tonal tokens, {len(changed)} out of date")
        return 1 if changed else 0
    if changed:
        path.write_text(out)
    print(f"{count} tonal tokens, {len(changed)} rewritten")
    return 0


if __name__ == "__main__":
    sys.exit(main())
