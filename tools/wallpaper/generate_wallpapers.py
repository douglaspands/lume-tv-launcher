"""Generates the 10 wallpapers for Lume from the approved master images in tools/wallpaper/masters/.

Aurora is produced by aurora06.py (project standard).
The other 9 wallpapers are converted from the approved high-resolution masters,
resampled to 1920x1080 (Full HD, the TV's own resolution) with Lanczos filtering, and calibrated to meet Lume's
strict contrast requirements (top bar contrast and dock contrast >= 4.5:1).

Also generates a single compressed 640x144 RGBA thumbnail atlas containing 128x72
previews of all 10 wallpapers for the home screen editor.
"""
import math
import os
import random
import sys
from PIL import Image
import aurora06

WIDTH, HEIGHT = aurora06.WIDTH, aurora06.HEIGHT
# The calibration below was tuned on 360 rows: row ranges scale with the height.
K = HEIGHT / 360


def smooth(a, b, x):
    t = min(1.0, max(0.0, (x - a) / (b - a)))
    return t * t * (3 - 2 * t)


def clamp(v):
    return max(0, min(255, int(round(v))))


MASTERS = [
    ("aurora", None),  # Generated via aurora06
    ("misty_forest", "1_floresta_noturna.jpg"),
    ("midnight_ocean", "2_ondas_meia_noite.jpg"),
    ("obsidian_canyon", "3_canion_estrelado.jpg"),
    ("silk_wave", "4_fios_luz_cosmica.jpg"),
    ("prismatic_topo", "5_topografia_escura.jpg"),
    ("mirror_lake", "6_lago_espelhado.jpg"),
    ("black_dunes", "7_dunas_negras.jpg"),
    ("glacial_mist", "8_nevoa_glacial.jpg"),
    ("stardust_ether", "9_eter_estelar.jpg"),
]


def get_luma(y_start, y_end, px_data):
    y_start, y_end = int(y_start * K), int(y_end * K)
    tot = sum(
        0.2126 * px_data[y * WIDTH + x][0] + 0.7152 * px_data[y * WIDTH + x][1] + 0.0722 * px_data[y * WIDTH + x][2]
        for y in range(y_start, y_end) for x in range(WIDTH)
    )
    return tot / ((y_end - y_start) * WIDTH)


def relative_luminance(r, g, b):
    def lin(c):
        c /= 255.0
        return c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4
    return 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b)


# The top bar's muted text needs 4.5:1, which caps the brightest pixel behind it at about this
# relative luminance (with a little margin). The average is not enough: at full size the peaks that
# a 640x360 reduction used to smooth out are back.
TOP_BAND = 90 * HEIGHT // 720
TOP_MAX_LUMINANCE = 0.050


def top_protection(raw_pixels):
    """How much to darken the very top rows so the brightest pixel of the band stays under the cap."""
    peak = max(relative_luminance(*raw_pixels[y * WIDTH + x])
               for y in range(TOP_BAND) for x in range(WIDTH))
    if peak <= TOP_MAX_LUMINANCE:
        return 1.0
    return (TOP_MAX_LUMINANCE / peak) ** (1 / 2.2)


def process_master(src_path):
    im = Image.open(src_path).convert("RGB")
    if im.size != (WIDTH, HEIGHT):
        im = im.resize((WIDTH, HEIGHT), Image.Resampling.LANCZOS)
    raw_pixels = list(im.getdata())

    top_l = get_luma(0, 30, raw_pixels)
    bot_l = get_luma(300, 360, raw_pixels)

    top_scale = top_protection(raw_pixels)

    out_pixels = []
    # Triangular dither where the calibration scales the pixels: scaling 8-bit values and rounding
    # again would band the dark gradients (deterministic, seeded by the master's name).
    rng = random.Random(os.path.basename(src_path))
    for y in range(HEIGHT):
        fade_factor = 1.0
        # Gentle dock protection in bottom rows (260..360) so dock always has >= 4.5:1 contrast
        if y >= 260 * K:
            v = (y - 260 * K) / (100.0 * K)
            factor = 0.78 if bot_l >= 18.0 else 0.88
            fade_factor = 1.0 - (1.0 - factor) * smooth(0.0, 1.0, v)
        
        # Subtle top protection if top luma is high
        if y < 35 * K and top_l > 35.0:
            v = y / (35.0 * K)
            fade_factor *= (0.75 + 0.25 * smooth(0.0, 1.0, v))

        if top_scale < 1.0 and y < TOP_BAND:
            # Strongest at the top, fading out at the bottom of the band.
            fade_factor *= top_scale + (1.0 - top_scale) * smooth(0.0, 1.0, y / TOP_BAND)

        for x in range(WIDTH):
            r, g, b = raw_pixels[y * WIDTH + x]
            if fade_factor == 1.0:
                out_pixels.append((r, g, b))
                continue
            n = lambda: rng.random() - rng.random()
            out_pixels.append((clamp(r * fade_factor + n()), clamp(g * fade_factor + n()), clamp(b * fade_factor + n())))

    new_top = get_luma(0, 30, out_pixels)
    new_bot = get_luma(300, 360, out_pixels)
    new_mid = get_luma(120, 240, out_pixels)

    assert new_top < 40.0, f"top luma {new_top} >= 40.0"
    assert new_bot < 25.0, f"bottom luma {new_bot} >= 25.0"
    assert new_mid > new_bot, f"mid luma {new_mid} <= bottom {new_bot}"

    return bytes(c for px in out_pixels for c in px)


def build_thumbnails(masters_dir, assets_dir):
    atlas = Image.new("RGBA", (640, 144), (0, 0, 0, 0))
    for i, (name, filename) in enumerate(MASTERS):
        if filename is None:
            with open(os.path.join(assets_dir, f"aurora-{WIDTH}x{HEIGHT}.rgb"), "rb") as f:
                raw = f.read()
            im = Image.frombytes("RGB", (WIDTH, HEIGHT), raw).convert("RGBA")
        else:
            src = os.path.join(masters_dir, filename)
            im = Image.open(src).convert("RGBA")
        thumb = im.resize((128, 72), Image.Resampling.LANCZOS)
        col = i % 5
        row = i // 5
        atlas.paste(thumb, (col * 128, row * 72))

    dst_rgba = os.path.join(assets_dir, "thumbnails-640x144.rgba")
    with open(dst_rgba, "wb") as f:
        f.write(atlas.tobytes())
    print(f"Generated thumbnail atlas (640x144 RGBA): {dst_rgba}")


def main():
    root = os.path.dirname(os.path.abspath(__file__))
    assets_dir = sys.argv[1] if len(sys.argv) > 1 else os.path.join(
        root, "../../crates/lume-core/assets/wallpaper"
    )
    masters_dir = os.path.join(root, "masters")
    os.makedirs(assets_dir, exist_ok=True)

    # 1. Aurora
    aurora06.main()

    # 2..10. Master images
    for name, filename in MASTERS:
        if filename is None:
            continue
        src = os.path.join(masters_dir, filename)
        rgb_data = process_master(src)
        dst = os.path.join(assets_dir, f"{name}-{WIDTH}x{HEIGHT}.rgb")
        with open(dst, "wb") as f:
            f.write(rgb_data)

        # Star files: keep clean comment unless custom stars are defined
        stars_path = os.path.join(assets_dir, f"{name}-stars.txt")
        if not os.path.exists(stars_path):
            with open(stars_path, "w") as f:
                f.write(f"# 0 stars for {name}\n")

    # 3. Generate thumbnail atlas
    build_thumbnails(masters_dir, assets_dir)

    print(f"Successfully processed all 10 wallpapers and thumbnails into {assets_dir}")


if __name__ == "__main__":
    main()
