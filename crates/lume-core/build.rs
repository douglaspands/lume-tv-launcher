//! Compiles the theme and the translations into generated Rust:
//!
//! - `themes/lume/theme.css` -> `theme_generated.rs`
//! - `android/app/src/main/res/values*/strings.xml` -> `strings_generated.rs`
//! - `assets/wallpaper/<id>-*` -> `wallpaper_<id>_stars.rs`
//! - `assets/companion/index.html` + the theme + `src/logo.rs` -> `companion_page.html`
//!
//! Warnings are forwarded as `cargo:warning`; any error fails the build.

use lume_css::{Color, Curve, Decl, Paint, Severity, State, Transition};
use std::fmt::Write as _;
use std::path::PathBuf;

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../../Cargo.lock");
    println!("cargo:rerun-if-changed=../lume-app/Cargo.toml");
    compile_keys(&manifest);
    compile_strings(&manifest);
    let sheet = compile_theme(&manifest);
    compile_companion(&manifest, &sheet);
    compile_wallpaper(&manifest);
    list_third_party(&manifest);
}

fn compile_keys(manifest: &std::path::Path) {
    println!("cargo:rerun-if-env-changed=LUME_KEYS_FILE");

    let mut up = vec![19];
    let mut down = vec![20];
    let mut left = vec![21];
    let mut right = vec![22];
    let mut ok = vec![23, 66, 160, 96];
    let mut back = vec![4, 111, 97];
    let mut home = vec![3];
    let mut menu = vec![82, 174];
    let mut inputs = vec![178];

    if let Ok(raw) = std::env::var("LUME_KEYS_FILE") {
        let raw = raw.trim();
        if !raw.is_empty() {
            let path = if std::path::Path::new(raw).is_relative() {
                let p = manifest.join("../../").join(raw);
                if p.exists() {
                    p
                } else {
                    manifest.join(raw)
                }
            } else {
                std::path::PathBuf::from(raw)
            };
            println!("cargo:rerun-if-changed={}", path.display());

            let content = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("cannot read LUME_KEYS_FILE {}: {e}", path.display()));

            let mut lines = content.lines();
            let header = lines.next().unwrap_or_else(|| {
                panic!("invalid LUME_KEYS_FILE {}: file is empty", path.display())
            });
            if header != "lume=1" {
                panic!(
                    "invalid LUME_KEYS_FILE {}: expected header 'lume=1', found '{header}'",
                    path.display()
                );
            }

            let mut complete = false;
            let allowed_keys = [
                "up", "down", "left", "right", "ok", "back", "home", "menu", "inputs",
            ];

            for (index, line) in lines.enumerate() {
                let line_num = index + 2;
                if complete {
                    panic!(
                        "invalid LUME_KEYS_FILE {}: unexpected content after '#end' (line {line_num})",
                        path.display()
                    );
                }
                if line == "#end" {
                    complete = true;
                } else if line.is_empty() || line.starts_with('#') {
                    continue;
                } else {
                    let (key, val) = line.split_once('=').unwrap_or_else(|| {
                        panic!(
                            "invalid LUME_KEYS_FILE {}: line {line_num} has no '='",
                            path.display()
                        )
                    });
                    if !allowed_keys.contains(&key) {
                        panic!(
                            "invalid LUME_KEYS_FILE {}: unknown key '{key}' on line {line_num}",
                            path.display()
                        );
                    }
                    let codes: Vec<i32> = if val.trim().is_empty() {
                        Vec::new()
                    } else {
                        val.split(',')
                            .map(|item| {
                                item.trim().parse::<i32>().unwrap_or_else(|_| {
                                    panic!(
                                        "invalid LUME_KEYS_FILE {}: invalid i32 keycode '{item}' on line {line_num}",
                                        path.display()
                                    )
                                })
                            })
                            .collect()
                    };

                    match key {
                        "up" => up = codes,
                        "down" => down = codes,
                        "left" => left = codes,
                        "right" => right = codes,
                        "ok" => ok = codes,
                        "back" => back = codes,
                        "home" => home = codes,
                        "menu" => menu = codes,
                        "inputs" => inputs = codes,
                        _ => unreachable!(),
                    }
                }
            }

            if !complete {
                panic!(
                    "invalid LUME_KEYS_FILE {}: missing '#end' marker",
                    path.display()
                );
            }
        }
    }

    let mut code = String::from("// @generated by build.rs. Do not edit.\n\n");
    writeln!(code, "pub const DEFAULT_UP: &[i32] = &{:?};", up).unwrap();
    writeln!(code, "pub const DEFAULT_DOWN: &[i32] = &{:?};", down).unwrap();
    writeln!(code, "pub const DEFAULT_LEFT: &[i32] = &{:?};", left).unwrap();
    writeln!(code, "pub const DEFAULT_RIGHT: &[i32] = &{:?};", right).unwrap();
    writeln!(code, "pub const DEFAULT_OK: &[i32] = &{:?};", ok).unwrap();
    writeln!(code, "pub const DEFAULT_BACK: &[i32] = &{:?};", back).unwrap();
    writeln!(code, "pub const DEFAULT_HOME: &[i32] = &{:?};", home).unwrap();
    writeln!(code, "pub const DEFAULT_MENU: &[i32] = &{:?};", menu).unwrap();
    writeln!(code, "pub const DEFAULT_INPUTS: &[i32] = &{:?};", inputs).unwrap();

    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("keys_generated.rs");
    std::fs::write(out, code).expect("cannot write keys_generated.rs");
}

/// Writes `third_party_generated.rs`: the open-source components the app ships, for the
/// "About" screen. The Rust crates come from `cargo tree` for the Android library.
fn list_third_party(manifest: &std::path::Path) {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let output = std::process::Command::new(cargo)
        .current_dir(manifest)
        .args([
            "tree",
            "--locked",
            "--package",
            "lume-app",
            "--target",
            "aarch64-linux-android",
            "--edges",
            "normal",
            "--prefix",
            "none",
            "--format",
            "{p}|{l}",
        ])
        .output();
    let mut crates: Vec<(String, String, String)> = Vec::new();
    match output {
        Ok(out) if out.status.success() => {
            for line in String::from_utf8_lossy(&out.stdout).lines() {
                let Some((package, license)) = line.split_once('|') else { continue };
                let mut parts = package.split_whitespace();
                let (Some(name), Some(version)) = (parts.next(), parts.next()) else { continue };
                // Crates of this workspace have a path in parentheses; they are lume itself.
                if package.contains('(') && !package.contains("registry") {
                    continue;
                }
                // `cargo tree` marks a repeated subtree with "(*)".
                let license = license.trim().trim_end_matches("(*)").trim();
                let license = if license.is_empty() { "see the crate" } else { license };
                let entry = (name.to_string(), version.trim_start_matches('v').to_string(), license.to_string());
                if !crates.contains(&entry) {
                    crates.push(entry);
                }
            }
            crates.sort();
        }
        _ => println!("cargo:warning=cannot list the Rust dependencies; the About screen will list only fonts and icons"),
    }
    let mut code = String::from("// @generated by build.rs. Do not edit.\n\n");
    code.push_str("/// Rust crates in the app: name, version, license.\npub const CRATES: &[(&str, &str, &str)] = &[\n");
    for (name, version, license) in &crates {
        writeln!(code, "    ({name:?}, {version:?}, {license:?}),").unwrap();
    }
    code.push_str("];\n");
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("third_party_generated.rs");
    std::fs::write(out, code).expect("cannot write third_party_generated.rs");
}

fn compile_strings(manifest: &std::path::Path) {
    let res_dir = manifest.join("../../android/app/src/main/res");
    println!("cargo:rerun-if-changed={}", res_dir.display());
    let compiled = lume_strings::compile_directory(&res_dir)
        .unwrap_or_else(|e| panic!("cannot compile the translations: {e}"));
    for warning in &compiled.warnings {
        println!("cargo:warning={warning}");
    }
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("strings_generated.rs");
    std::fs::write(out, compiled.code).expect("cannot write strings_generated.rs");
}

/// Embeds the positions of the stars and the thumbnail atlas of the
/// 6 wallpapers generated by `tools/wallpaper/`; the images themselves are assets of the APK
/// (see `docs/wallpaper.md`).
fn compile_wallpaper(manifest: &std::path::Path) {
    let dir = manifest.join("assets/wallpaper");
    println!("cargo:rerun-if-changed={}", dir.display());
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());

    let wallpapers = [
        "aurora",
        "mountain_stars",
        "mirror_lake",
        "misty_forest",
        "glass_facade",
        "synthwave_grid",
    ];

    for name in wallpapers {
        let stars_name = format!("{name}-stars.txt");
        let text = std::fs::read_to_string(dir.join(&stars_name))
            .unwrap_or_else(|e| panic!("cannot read {stars_name}: {e}"));
        let stars = text
            .lines()
            .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
            .map(|l| {
                let n: Vec<&str> = l.split_whitespace().collect();
                assert_eq!(n.len(), 3, "bad star line '{l}' in {stars_name}");
                format!("({}, {}, {})", n[0], n[1], n[2])
            })
            .collect::<Vec<_>>()
            .join(", ");
        std::fs::write(
            out_dir.join(format!("wallpaper_{name}_stars.rs")),
            format!("&[{stars}]"),
        )
        .expect("cannot write the stars");
    }

    let thumbs_path = dir.join("thumbnails-640x144.rgba");
    if thumbs_path.exists() {
        let rgba = std::fs::read(&thumbs_path).expect("cannot read thumbnails-640x144.rgba");
        assert_eq!(rgba.len(), 640 * 144 * 4);
        let compressed = miniz_oxide::deflate::compress_to_vec(&rgba, 9);
        std::fs::write(
            out_dir.join("wallpaper_thumbnails_640x144.deflate"),
            &compressed,
        )
        .unwrap();
    }
}

fn compile_theme(manifest: &std::path::Path) -> lume_css::Stylesheet {
    let theme_path = manifest.join("../../themes/lume/theme.css");
    println!("cargo:rerun-if-changed={}", theme_path.display());

    let source = std::fs::read_to_string(&theme_path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", theme_path.display()));
    let display_path = "themes/lume/theme.css";
    let parsed = lume_css::parse(&source, display_path);

    let mut failed = false;
    for d in &parsed.diagnostics {
        match d.severity {
            Severity::Warning => println!("cargo:warning={d}"),
            Severity::Error => {
                eprintln!("{d}");
                failed = true;
            }
        }
    }
    if failed {
        panic!("the theme {display_path} has errors");
    }

    let code = generate(&parsed.stylesheet);
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("theme_generated.rs");
    std::fs::write(out, code).expect("cannot write theme_generated.rs");
    parsed.stylesheet
}

/// The phone page: the template with the theme's colors and shapes and the logo as SVG, drawn from
/// the same constants as `src/logo.rs`, so the page never drifts from the TV.
fn compile_companion(manifest: &std::path::Path, sheet: &lume_css::Stylesheet) {
    let template_path = manifest.join("assets/companion/index.html");
    let logo_path = manifest.join("src/logo.rs");
    println!("cargo:rerun-if-changed={}", template_path.display());
    println!("cargo:rerun-if-changed={}", logo_path.display());
    let mut page = std::fs::read_to_string(&template_path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", template_path.display()));
    let css_color = |name: &str| {
        let c = sheet
            .color_variable(&format!("--{name}"))
            .unwrap_or_else(|| panic!("the theme has no color --{name}"));
        if c.a == 255 {
            format!("#{:02X}{:02X}{:02X}", c.r, c.g, c.b)
        } else {
            format!(
                "rgba({}, {}, {}, {:.3})",
                c.r,
                c.g,
                c.b,
                f32::from(c.a) / 255.0
            )
        }
    };
    for name in [
        "bg",
        "surface",
        "surface-row",
        "border",
        "primary",
        "on-primary",
        "text",
        "text-muted",
        "danger-fg",
    ] {
        page = page.replace(&format!("{{{{{name}}}}}"), &css_color(name));
    }
    for name in ["radius", "shape-lg"] {
        let px = sheet
            .length_variable(&format!("--{name}"))
            .unwrap_or_else(|| panic!("the theme has no length --{name}"));
        page = page.replace(&format!("{{{{{name}}}}}"), &format!("{px}px"));
    }
    let logo = std::fs::read_to_string(&logo_path).expect("cannot read src/logo.rs");
    let svg = logo_svg(&logo, &css_color("primary"));
    page = page.replace("{{logo}}", &svg);
    if let Some(at) = page.find("{{") {
        panic!(
            "assets/companion/index.html: unknown value {}",
            &page[at..page[at..].find("}}").map_or(page.len(), |e| at + e + 2)]
        );
    }
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("companion_page.html");
    std::fs::write(out, page).expect("cannot write companion_page.html");
}

/// The logo mark of `src/logo.rs` as SVG in a 100x100 box: a rounded square in `square` color
/// with the two bars of the L and the dot cut out via `<mask>` so they are truly transparent,
/// read from the `const NAME: f32 = value;` lines.
fn logo_svg(source: &str, square: &str) -> String {
    let constant = |name: &str| -> f32 {
        source
            .lines()
            .find_map(|line| {
                let rest = line.trim().strip_prefix(&format!("const {name}: f32 = "))?;
                rest.trim_end_matches(';').trim().parse().ok()
            })
            .unwrap_or_else(|| panic!("src/logo.rs has no const {name}: f32"))
    };
    let k = |v: f32| (v * 100.0 * 1000.0).round() / 1000.0;
    let (corner, stem_x, stem_w, stem_top) = (
        constant("CORNER"),
        constant("STEM_X"),
        constant("STEM_WIDTH"),
        constant("STEM_TOP"),
    );
    let (bottom, foot_right, foot_top) = (
        constant("BOTTOM"),
        constant("FOOT_RIGHT"),
        constant("FOOT_TOP"),
    );
    let (dot_x, dot_y, dot_r) = (constant("DOT_X"), constant("DOT_Y"), constant("DOT_RADIUS"));
    let foot_h = bottom - foot_top;
    format!(
        r##"<svg viewBox="0 0 100 100" aria-hidden="true"><defs><mask id="lume-mark"><rect width="100" height="100" fill="#fff"/><rect x="{}" y="{}" width="{}" height="{}" rx="{}" fill="#000"/><rect x="{}" y="{}" width="{}" height="{}" rx="{}" fill="#000"/><circle cx="{}" cy="{}" r="{}" fill="#000"/></mask></defs><rect width="100" height="100" rx="{}" fill="{square}" mask="url(#lume-mark)"/></svg>"##,
        k(stem_x),
        k(stem_top),
        k(stem_w),
        k(bottom - stem_top),
        k(stem_w / 2.0),
        k(stem_x),
        k(foot_top),
        k(foot_right - stem_x),
        k(foot_h),
        k(foot_h / 2.0),
        k(dot_x),
        k(dot_y),
        k(dot_r),
        k(corner),
    )
}

fn generate(sheet: &lume_css::Stylesheet) -> String {
    let mut out = String::new();
    out.push_str("// @generated by build.rs from themes/lume/theme.css. Do not edit.\n\n");

    // One field per component, in the order of `lume_css::COMPONENTS`.
    out.push_str("/// Every component's rules.\n#[derive(Clone, Copy, Debug, PartialEq)]\npub struct Theme {\n");
    for c in lume_css::COMPONENTS {
        writeln!(out, "    pub {}: ComponentTheme,", field_name(c)).unwrap();
    }
    out.push_str("}\n\n");

    out.push_str("/// The compiled-in theme.\npub const THEME: Theme = Theme {\n");
    for c in lume_css::COMPONENTS {
        writeln!(
            out,
            "    {}: ComponentTheme {{ base: {}, focus: {}, active: {}, disabled: {} }},",
            field_name(c),
            style_literal(sheet, c, State::Base),
            style_literal(sheet, c, State::Focus),
            style_literal(sheet, c, State::Active),
            style_literal(sheet, c, State::Disabled),
        )
        .unwrap();
    }
    out.push_str("};\n\n");

    // Color, length and transition variables become typed constants.
    out.push_str("/// Theme variables (`--name` becomes `NAME`).\n#[allow(dead_code)]\npub mod vars {\n    use super::{Color, Curve, Transition};\n\n");
    for (name, _) in &sheet.variables {
        let ident = name
            .trim_start_matches("--")
            .replace('-', "_")
            .to_ascii_uppercase();
        if let Some(c) = sheet.color_variable(name) {
            writeln!(out, "    pub const {ident}: Color = {};", color(c)).unwrap();
        } else if let Some(px) = sheet.length_variable(name) {
            writeln!(out, "    pub const {ident}: f32 = {px:?};").unwrap();
        } else if let Some(t) = sheet.transition_variable(name) {
            writeln!(
                out,
                "    pub const {ident}: Transition = {};",
                transition(&t)
            )
            .unwrap();
        }
    }
    out.push_str("}\n");
    out
}

fn field_name(component: &str) -> String {
    component.replace('-', "_")
}

fn color(c: Color) -> String {
    format!("Color {{ r: {}, g: {}, b: {}, a: {} }}", c.r, c.g, c.b, c.a)
}

fn transition(t: &Transition) -> String {
    let curve = match t.curve {
        Curve::Linear => "Curve::Linear".to_string(),
        Curve::Ease => "Curve::Ease".to_string(),
        Curve::EaseIn => "Curve::EaseIn".to_string(),
        Curve::EaseOut => "Curve::EaseOut".to_string(),
        Curve::EaseInOut => "Curve::EaseInOut".to_string(),
        Curve::CubicBezier(a, b, c, d) => format!("Curve::CubicBezier({a:?}, {b:?}, {c:?}, {d:?})"),
    };
    format!(
        "Transition {{ duration_ms: {:?}, curve: {curve} }}",
        t.duration_ms
    )
}

fn some(inner: String) -> String {
    format!("Some({inner})")
}

fn style_literal(sheet: &lume_css::Stylesheet, component: &str, state: State) -> String {
    // Later declarations win, so apply them in order over an empty style.
    let mut fields: Vec<(&str, String)> = Vec::new();
    let mut set = |name: &'static str, value: String| {
        fields.retain(|(n, _)| *n != name);
        fields.push((name, value));
    };
    for decl in sheet.decls(component, state) {
        match decl {
            Decl::Background(Paint::Solid(c)) => {
                set("background", some(format!("Paint::Solid({})", color(*c))))
            }
            Decl::Background(Paint::Gradient(g)) => {
                let stops: Vec<String> = g
                    .stops
                    .iter()
                    .map(|s| {
                        format!("GradientStop {{ color: {}, position: {:?} }}", color(s.color), s.position)
                    })
                    .collect();
                set(
                    "background",
                    some(format!(
                        "Paint::Gradient(Gradient::new({:?}, &[{}]))",
                        g.angle_deg,
                        stops.join(", ")
                    )),
                );
            }
            Decl::Color(c) => set("color", some(color(*c))),
            Decl::Opacity(v) => set("opacity", some(format!("{v:?}"))),
            Decl::BorderRadius(v) => set("border_radius", some(format!("{v:?}"))),
            Decl::Outline(o) => set(
                "outline",
                some(format!("Outline {{ width: {:?}, color: {} }}", o.width, color(o.color))),
            ),
            Decl::OutlineOffset(v) => set("outline_offset", some(format!("{v:?}"))),
            Decl::BoxShadow(None) => set("box_shadow", some("Shadow::NONE".into())),
            Decl::BoxShadow(Some(s)) => set(
                "box_shadow",
                some(format!(
                    "Shadow {{ offset_x: {:?}, offset_y: {:?}, blur: {:?}, spread: {:?}, color: {} }}",
                    s.offset_x,
                    s.offset_y,
                    s.blur,
                    s.spread,
                    color(s.color)
                )),
            ),
            Decl::FontSize(v) => set("font_size", some(format!("{v:?}"))),
            Decl::FontWeight(v) => set("font_weight", some(format!("{v}"))),
            Decl::LetterSpacing(v) => set("letter_spacing", some(format!("{v:?}"))),
            Decl::LineHeight(v) => set("line_height", some(format!("{v:?}"))),
            Decl::Padding(p) => set("padding", some(format!("[{:?}, {:?}, {:?}, {:?}]", p[0], p[1], p[2], p[3]))),
            Decl::Gap(v) => set("gap", some(format!("{v:?}"))),
            Decl::Width(v) => set("width", some(format!("{v:?}"))),
            Decl::Height(v) => set("height", some(format!("{v:?}"))),
            Decl::TransformScale(v) => set("scale", some(format!("{v:?}"))),
            Decl::Transition(t) => {
                set("transition", some(transition(t)));
            }
        }
    }
    if fields.is_empty() {
        return "Style::EMPTY".to_string();
    }
    let body: Vec<String> = fields.iter().map(|(n, v)| format!("{n}: {v}")).collect();
    format!("Style {{ {}, ..Style::EMPTY }}", body.join(", "))
}
