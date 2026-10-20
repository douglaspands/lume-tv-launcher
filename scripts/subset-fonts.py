#!/usr/bin/env python3
"""Regenerates the font assets embedded in lume.

Downloads pinned upstream sources, subsets them and writes:
  crates/lume-core/assets/fonts/PlusJakartaSans-<weight>.ttf   (Latin-1 + Latin Extended-A)
  crates/lume-core/assets/fonts/lucide-subset.ttf              (only the icons lume uses)
  crates/lume-core/assets/fonts/OFL.txt, LUCIDE-LICENSE.txt
  crates/lume-core/src/icons.rs                                (icon code points)

Requirements: python3 and `pip install fonttools`.
Usage: scripts/subset-fonts.py
"""

import json
import pathlib
import urllib.request

from fontTools import subset
from fontTools.ttLib import TTFont, newTable
from fontTools.ttLib.tables._k_e_r_n import KernTable_format_0

PLUS_JAKARTA_COMMIT = "18d1cd2f7ea10481919d2f05c1f7064b7307fc26"
PLUS_JAKARTA_URL = (
    "https://raw.githubusercontent.com/tokotype/PlusJakartaSans/"
    + PLUS_JAKARTA_COMMIT
    + "/{path}"
)
LUCIDE_VERSION = "1.59.0"
LUCIDE_URL = "https://unpkg.com/lucide-static@" + LUCIDE_VERSION + "/{path}"

# Weights the theme uses (see themes/lume/theme.css).
WEIGHTS = {"Medium": 500, "Bold": 700, "ExtraBold": 800}

# Latin-1 + Latin Extended-A, plus the punctuation the UI strings use.
TEXT_UNICODES = (
    list(range(0x0020, 0x007F))
    + list(range(0x00A0, 0x0100))
    + list(range(0x0100, 0x0180))
    + [0x2013, 0x2014, 0x2018, 0x2019, 0x201C, 0x201D, 0x2022, 0x2026, 0x2039, 0x203A, 0x20AC]
)

# Lucide icon names used by the UI, mapped to Rust constant names.
ICONS = {
    "settings": "SETTINGS",
    "log-in": "INPUTS",
    "chevron-up": "CHEVRON_UP",
    "chevron-down": "CHEVRON_DOWN",
    "chevron-left": "CHEVRON_LEFT",
    "chevron-right": "CHEVRON_RIGHT",
    "trash-2": "TRASH",
    "x": "CLOSE",
    "check": "CHECK",
    "plus": "PLUS",
    "play": "PLAY",
    "eye": "EYE",
    "eye-off": "EYE_OFF",
}

# Kerning pairs smaller than this many font units (of 1000 per em) are invisible
# at TV sizes and are dropped to keep the legacy `kern` table small.
MIN_KERN_UNITS = 10
# Kerning is kept for Basic Latin only; accented letters fall back to no kerning.
KERN_UNICODES = range(0x0021, 0x007F)

ROOT = pathlib.Path(__file__).resolve().parent.parent
FONTS = ROOT / "crates/lume-core/assets/fonts"


def download(url: str) -> bytes:
    with urllib.request.urlopen(url) as response:
        return response.read()


def gpos_kern_pairs(font: TTFont, glyphs: list[str]) -> dict[tuple[str, str], int]:
    """Expands the GPOS `kern` feature (pair adjustment) into plain glyph pairs."""
    gpos = font["GPOS"].table
    lookup_indices = set()
    for record in gpos.FeatureList.FeatureRecord:
        if record.FeatureTag == "kern":
            lookup_indices.update(record.Feature.LookupListIndex)

    wanted = set(glyphs)
    pairs: dict[tuple[str, str], int] = {}

    def advance(value) -> int:
        return getattr(value, "XAdvance", 0) or 0 if value is not None else 0

    for index in sorted(lookup_indices):
        for subtable in gpos.LookupList.Lookup[index].SubTable:
            if subtable.LookupType == 9:  # extension
                subtable = subtable.ExtSubTable
            if subtable.LookupType != 2:  # pair adjustment
                continue
            first_glyphs = [g for g in subtable.Coverage.glyphs if g in wanted]
            if subtable.Format == 1:
                for glyph, pair_set in zip(subtable.Coverage.glyphs, subtable.PairSet):
                    if glyph not in wanted:
                        continue
                    for record in pair_set.PairValueRecord:
                        if record.SecondGlyph in wanted:
                            pairs.setdefault((glyph, record.SecondGlyph), advance(record.Value1))
            else:
                classes1 = subtable.ClassDef1.classDefs
                classes2 = subtable.ClassDef2.classDefs
                for first in first_glyphs:
                    row = subtable.Class1Record[classes1.get(first, 0)].Class2Record
                    for second in glyphs:
                        value = advance(row[classes2.get(second, 0)].Value1)
                        pairs.setdefault((first, second), value)
    return {pair: v for pair, v in pairs.items() if abs(v) >= MIN_KERN_UNITS}


def replace_gpos_with_kern_table(font: TTFont) -> None:
    """fontdue only reads the legacy `kern` table, so convert GPOS kerning into one."""
    cmap = font.getBestCmap()
    glyphs = [cmap[c] for c in KERN_UNICODES if c in cmap]
    pairs = gpos_kern_pairs(font, glyphs)
    if len(pairs) > 10920:
        raise SystemExit(f"{len(pairs)} kerning pairs do not fit one kern subtable")
    kern = newTable("kern")
    kern.version = 0
    subtable = KernTable_format_0()
    subtable.coverage = 1
    subtable.format = 0
    subtable.kernTable = pairs
    kern.kernTables = [subtable]
    font["kern"] = kern
    del font["GPOS"]


def subset_font(data: bytes, unicodes: list[int], out: pathlib.Path, kern: bool) -> None:
    options = subset.Options()
    options.layout_features = ["kern"] if kern else []
    options.hinting = False
    options.name_IDs = [0, 1, 2, 3, 4, 5, 6, 13, 14]
    options.notdef_outline = True
    options.drop_tables += ["GSUB", "GDEF", "DSIG", "gasp", "cvt ", "fpgm", "prep"]
    font = TTFont(__import__("io").BytesIO(data))
    subsetter = subset.Subsetter(options)
    subsetter.populate(unicodes=unicodes)
    subsetter.subset(font)
    if kern:
        replace_gpos_with_kern_table(font)
    font.save(out)
    print(f"{out.relative_to(ROOT)}: {out.stat().st_size} bytes")


def main() -> None:
    FONTS.mkdir(parents=True, exist_ok=True)

    for name in WEIGHTS:
        data = download(PLUS_JAKARTA_URL.format(path=f"fonts/ttf/PlusJakartaSans-{name}.ttf"))
        subset_font(data, TEXT_UNICODES, FONTS / f"PlusJakartaSans-{name}.ttf", kern=True)
    (FONTS / "OFL.txt").write_bytes(download(PLUS_JAKARTA_URL.format(path="OFL.txt")))

    info = json.loads(download(LUCIDE_URL.format(path="font/info.json")))
    codepoints = {}
    for icon, const in ICONS.items():
        codepoints[const] = int(info[icon]["encodedCode"].lstrip("\\"), 16)
    subset_font(
        download(LUCIDE_URL.format(path="font/lucide.ttf")),
        list(codepoints.values()),
        FONTS / "lucide-subset.ttf",
        kern=False,
    )
    (FONTS / "LUCIDE-LICENSE.txt").write_bytes(download(LUCIDE_URL.format(path="LICENSE")))

    lines = [
        "//! Icon code points in `assets/fonts/lucide-subset.ttf`.",
        "//!",
        f"//! @generated by scripts/subset-fonts.py from Lucide {LUCIDE_VERSION}. Do not edit.",
        "",
    ]
    for const, code in codepoints.items():
        lines.append(f"pub const {const}: char = '\\u{{{code:x}}}';")
    (ROOT / "crates/lume-core/src/icons.rs").write_text("\n".join(lines) + "\n")
    print("crates/lume-core/src/icons.rs written")


if __name__ == "__main__":
    main()
