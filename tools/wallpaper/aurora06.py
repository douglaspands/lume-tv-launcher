"""The lume wallpaper (aurora 06): a smooth 640x360 base plus the positions of crisp stars.

Usage: python3 tools/wallpaper/aurora06.py [ASSETS_DIR]

Writes aurora-640x360.rgb (raw RGB, top row first) and aurora-stars.txt into ASSETS_DIR (default
crates/lume-core/assets/wallpaper). Only the standard library is needed. The renderer draws the
stars as 1 or 2 pixel points over the enlarged base (see Painter::wallpaper)."""
import math, random, struct, sys, zlib

TAU = math.pi * 2
PETROL = (15, 23, 30)
ACCENT = (77, 179, 255)
CYAN = (70, 215, 255)
INDIGO = (70, 90, 230)
SEED = 21


def smooth(a, b, x):
    t = min(1.0, max(0.0, (x - a) / (b - a)))
    return t * t * (3 - 2 * t)


def mix(c1, c2, t):
    return tuple(a + (b - a) * t for a, b in zip(c1, c2))


def column_noise(seed, fa, fb):
    rnd = random.Random(seed)
    pa, pb = rnd.random() * TAU, rnd.random() * TAU
    return lambda u: math.sin(TAU * fa * u + pa) * 0.6 + math.sin(TAU * fb * u + pb) * 0.4


LAYERS = [
    dict(base=0.52, amp=0.08, f1=1.3, f2=2.7, rf1=22, rf2=41, ray=0.5, h=0.18, gain=0.95, low=ACCENT, high=INDIGO),
    dict(base=0.42, amp=0.07, f1=1.7, f2=3.3, rf1=18, rf2=37, ray=0.55, h=0.13, gain=0.65, low=CYAN, high=ACCENT),
]


def stars(w, h, n=420):
    rnd = random.Random(SEED)
    out = []
    for _ in range(n):
        out.append((rnd.randrange(w), rnd.randrange(int(h * 0.7)), 0.25 + rnd.random() * 0.75))
    return out


def base(w, h, edge_px):
    """The smooth aurora, rows of (r, g, b) floats 0-255, without stars."""
    cols = []
    for i, L in enumerate(LAYERS):
        wave = column_noise(SEED * 10 + i, L["f1"], L["f2"])
        rays = column_noise(SEED * 100 + i, L["rf1"], L["rf2"])
        edge = [(L["base"] + L["amp"] * wave(x / w)) * h for x in range(w)]
        ray = [1.0 - L["ray"] + L["ray"] * (0.5 + 0.5 * rays(x / w)) for x in range(w)]
        cols.append((edge, ray))
    rows = []
    for y in range(h):
        v = y / h
        sky = mix((5, 9, 18), PETROL, smooth(0.0, 1.0, v))
        top_fade = smooth(0.0, 0.12, v)
        dark = 1.0 - 0.6 * smooth(0.60, 1.0, v)
        row = []
        for x in range(w):
            r, g, b = sky
            for (edge, ray), L in zip(cols, LAYERS):
                d = edge[x] - y
                if d < -edge_px:
                    continue
                inten = math.exp(-d / (L["h"] * h)) if d >= 0 else 1.0 + d / edge_px
                inten *= ray[x] * L["gain"] * top_fade
                t = min(1.0, max(0.0, d / (L["h"] * h * 2.2)))
                c = mix(L["low"], L["high"], t)
                r += c[0] * inten
                g += c[1] * inten
                b += c[2] * inten
            row.append((255 * (1 - math.exp(-r / 150)) * dark,
                        255 * (1 - math.exp(-g / 150)) * dark,
                        255 * (1 - math.exp(-b / 150)) * dark))
        rows.append(row)
    return rows


def quantize(rows):
    return [[(min(255, int(r)), min(255, int(g)), min(255, int(b))) for r, g, b in row] for row in rows]


def upscale(rows, w, h):
    sh, sw = len(rows), len(rows[0])
    xs = []
    for x in range(w):
        fx = (x + 0.5) * sw / w - 0.5
        x0 = max(0, min(sw - 1, int(math.floor(fx))))
        x1 = min(sw - 1, x0 + 1)
        xs.append((x0, x1, min(1.0, max(0.0, fx - math.floor(fx)))))
    out = []
    for y in range(h):
        fy = (y + 0.5) * sh / h - 0.5
        y0 = max(0, min(sh - 1, int(math.floor(fy))))
        y1 = min(sh - 1, y0 + 1)
        ty = min(1.0, max(0.0, fy - math.floor(fy)))
        a, b = rows[y0], rows[y1]
        row = []
        for x0, x1, tx in xs:
            p00, p10, p01, p11 = a[x0], a[x1], b[x0], b[x1]
            row.append(tuple(
                (p00[c] * (1 - tx) + p10[c] * tx) * (1 - ty) + (p01[c] * (1 - tx) + p11[c] * tx) * ty
                for c in range(3)))
        out.append(row)
    return out


def add_stars(rows, w, h, scale=1):
    for x, y, k in stars(w, h):
        size = 2 if k > 0.8 else 1
        for dy in range(size):
            for dx in range(size):
                if x + dx < w and y + dy < h:
                    r, g, b = rows[y + dy][x + dx]
                    rows[y + dy][x + dx] = (min(255, r + 200 * k), min(255, g + 210 * k), min(255, b + 235 * k))
    return rows


def main():
    import os
    out = sys.argv[1] if len(sys.argv) > 1 else os.path.join(
        os.path.dirname(os.path.abspath(__file__)), "../../crates/lume-core/assets/wallpaper")
    os.makedirs(out, exist_ok=True)
    # The base's edge is as soft as 3 of its own texels, so 640x360 reproduces a 1080p render.
    rows = quantize(base(640, 360, 3.0))
    with open(os.path.join(out, "aurora-640x360.rgb"), "wb") as f:
        f.write(bytes(c for row in rows for px in row for c in px))
    with open(os.path.join(out, "aurora-stars.txt"), "w") as f:
        f.write("# x y brightness(0-255), in 1920x1080 pixels; from tools/wallpaper/aurora06.py (seed 21)\n")
        for x, y, k in stars(1920, 1080):
            f.write(f"{x} {y} {round(k * 255)}\n")


if __name__ == "__main__":
    main()
