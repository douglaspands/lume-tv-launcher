# Wallpaper

Every screen of lume is drawn over one wallpaper: a northern-lights aurora in the brand colors (the
accent blue over the petrol background, with a few stars). The project makes it itself; no photo or
art from anyone else is used. It is one fixed wallpaper in this version; there is no setting to
change it.

## How the image is made

`tools/wallpaper/aurora06.py` (plain Python 3, no packages) draws two layers of aurora curtains and writes,
into `crates/lume-core/assets/wallpaper/`:

- `aurora-640x360.rgb`: the smooth base, raw RGB, top row first. The edge of the curtains is softened to
  about 3 of its own texels, so 640x360 enlarged to 1080p looks like a render made at 1080p
  (average error under 1/255). The top and the lower half are already dark, for the top bar and the dock.
- `aurora-stars.txt`: 420 stars (`x y brightness`, in 1920x1080 pixels, from a fixed random seed), all in
  the upper 70% of the picture.

To change the look, edit the script and run it again from the repository root:

```
python3 tools/wallpaper/aurora06.py
```

Running it without changes gives the same files byte for byte.

## How it gets on the screen

1. `crates/lume-core/build.rs` embeds both files (`include_bytes!` and a list of stars).
2. At runtime the renderer uploads the base (675 KB) as an opaque RGB texture (linear filter, clamped to
   the edge) the first time a frame draws it, and frees it with the rest of the graphics memory when lume
   goes to the background.
3. Every frame starts with `Painter::wallpaper`: one quad of that texture over the whole visible area, cropped
   to cover it, then the stars as points of 1 or 2 pixels of the render buffer. Stars are not part of the
   enlarged texture, so they stay as sharp as the interface (on a 4K screen lume renders at 1080p).
4. On screens that are mostly text (Apps, Settings, About, editor) the wallpaper is drawn already darkened by about 78%
   towards the theme's background color (`Painter::wallpaper(true)`), so every text keeps at least 4.5:1 of contrast
   over the brightest part of the aurora. There is no second layer over the screen: the image is drawn at 22% opacity
   over the `--bg` the frame is cleared to, and so are the stars. (A full-screen translucent layer cost more than half
   of the frames of the Apps grid on the TV.) Home shows the wallpaper as it is.

## Checks

- `cargo test -p lume-desktop --test wallpaper`: the frame rendered at 1920x1080 differs from
  `mockup/wallpaper-aurora-lume@1080p.png` by at most 2/255 on average, and the stars are sharp points at 720p and 1080p.
- `cargo test -p lume-core wallpaper` and `cargo test -p lume-desktop --test contrast`: the contrast of the
  texts over the wallpaper stays at 4.5:1 or more. A change to the image or to the 78% that breaks it fails here.

## Where to change it

`lume_core::wallpaper::current()` is the only place the renderer asks for the wallpaper. To use another
image, replace the two asset files (or change the script) and run `cargo test --workspace`.

To look at the result: `cargo run -p lume-desktop -- --screenshot home home.png`.
