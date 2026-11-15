//! The page the TV serves to the phone: built at compile time from `assets/companion/index.html`,
//! the theme and the logo (see `build.rs`), with the language and the texts filled in for each
//! request from its `Accept-Language`, chosen like the interface's (exact variant, then the
//! language, then English) from the same translation files.

use crate::i18n::{StringId, Translator};

const TEMPLATE: &str = include_str!(concat!(env!("OUT_DIR"), "/companion_page.html"));

/// The page's texts: the key its script reads (`L.<key>`) and the string.
const TEXTS: &[(&str, StringId)] = &[
    ("section", StringId::CompanionSection),
    ("req_formats", StringId::CompanionReqFormats),
    ("req_size", StringId::CompanionReqSize),
    ("req_edit", StringId::CompanionReqEdit),
    ("code_label", StringId::CompanionCodeLabel),
    ("code_hint", StringId::CompanionCodeHint),
    ("code_wrong", StringId::CompanionCodeWrong),
    ("choose", StringId::CompanionChoose),
    ("drop", StringId::CompanionDrop),
    ("replace", StringId::CompanionReplace),
    ("send", StringId::CompanionSend),
    ("send_again", StringId::CompanionSendAgain),
    ("applied", StringId::CompanionApplied),
    ("close_tab", StringId::CompanionCloseTab),
    ("declined", StringId::CompanionDeclined),
    ("preparing", StringId::CompanionPreparing),
    ("sending", StringId::CompanionSending),
    ("done", StringId::CompanionDone),
    ("warn_soft", StringId::CompanionWarnSoft),
    ("tile_only", StringId::CompanionTileOnly),
    ("modes_title", StringId::CompanionModesTitle),
    ("mode_contain", StringId::CompanionModeContain),
    ("mode_cover", StringId::CompanionModeCover),
    ("mode_stretch", StringId::CompanionModeStretch),
    ("mode_blur", StringId::CompanionModeBlur),
    ("mode_tile", StringId::CompanionModeTile),
    ("desc_contain", StringId::CompanionDescContain),
    ("desc_cover", StringId::CompanionDescCover),
    ("desc_stretch", StringId::CompanionDescStretch),
    ("desc_blur", StringId::CompanionDescBlur),
    ("desc_tile", StringId::CompanionDescTile),
    ("hint_drag", StringId::CompanionHintDrag),
    ("zoom", StringId::CompanionZoom),
    ("center", StringId::CompanionCenter),
    ("tile_grid", StringId::CompanionTileGrid),
    ("tile_mirror", StringId::CompanionTileMirror),
    ("tile_size", StringId::CompanionTileSize),
    ("err_small", StringId::CompanionErrSmall),
    ("err_format", StringId::CompanionErrFormat),
    ("err_heic", StringId::CompanionErrHeic),
    ("err_big", StringId::CompanionErrBig),
    ("err_read", StringId::CompanionErrRead),
    ("err_busy", StringId::CompanionErrBusy),
    ("err_closed", StringId::CompanionErrClosed),
    ("err_network", StringId::CompanionErrNetwork),
    ("err_browser", StringId::CompanionErrBrowser),
];

/// The languages of an `Accept-Language` header, most wanted first (`q=0` left out).
pub fn preferred_languages(header: &str) -> Vec<&str> {
    let mut weighted: Vec<(f32, usize, &str)> = header
        .split(',')
        .enumerate()
        .filter_map(|(i, part)| {
            let mut pieces = part.split(';');
            let tag = pieces.next()?.trim();
            if tag.is_empty() || tag == "*" {
                return None;
            }
            let q = pieces
                .filter_map(|p| p.trim().strip_prefix("q="))
                .find_map(|q| q.trim().parse::<f32>().ok())
                .unwrap_or(1.0);
            (q > 0.0).then_some((q, i, tag))
        })
        .collect();
    weighted.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    weighted.into_iter().map(|(_, _, tag)| tag).collect()
}

/// Escapes a text for a JSON string inside a `<script>` (no `</script>` can close it).
fn json_string(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '<' => out.push_str("\\u003c"),
            '>' => out.push_str("\\u003e"),
            '&' => out.push_str("\\u0026"),
            '\u{2028}' => out.push_str("\\u2028"),
            '\u{2029}' => out.push_str("\\u2029"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// The texts of a language as the page's `L` object.
fn texts(translator: &Translator) -> String {
    let fields: Vec<String> = TEXTS
        .iter()
        .map(|(key, id)| format!("{}:{}", json_string(key), json_string(&translator.get(*id))))
        .collect();
    format!("{{{}}}", fields.join(","))
}

/// The page in the language of an `Accept-Language` header.
pub fn render(accept_language: &str) -> String {
    let translator = Translator::new(&preferred_languages(accept_language));
    TEMPLATE
        .replacen("__LUME_LANG__", translator.language(), 1)
        .replacen("__LUME_STRINGS__", &texts(&translator), 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_language_follows_the_phone() {
        let page = render("es-ES,es;q=0.9,pt-BR;q=0.8");
        assert!(page.contains(r#"<html lang="es""#), "{}", &page[..200]);
        assert!(page.contains("Fondo de pantalla"));
        let page = render("pt-BR,pt;q=0.9");
        assert!(page.contains("Enviar para a TV"));
        // Japanese has no translation: English.
        let page = render("ja-JP,ja;q=0.9");
        assert!(page.contains(r#"<html lang="en""#));
        assert!(page.contains("Send to the TV"));
        // No header at all: English too.
        assert!(render("").contains("Send to the TV"));
    }

    #[test]
    fn weights_order_the_languages() {
        assert_eq!(
            preferred_languages("en;q=0.5, ru, es;q=0.8, fr;q=0"),
            ["ru", "es", "en"]
        );
    }

    #[test]
    fn the_page_is_complete_and_self_contained() {
        let page = render("en");
        assert!(!page.contains("__LUME_"));
        assert!(!page.contains("{{"));
        for (key, _) in TEXTS {
            assert!(page.contains(&format!("\"{key}\":")), "{key}");
        }
        // No external resource: no http(s) URL loaded by the page.
        for needle in ["src=\"http", "href=\"http", "url(http", "@import"] {
            assert!(!page.contains(needle), "{needle}");
        }
        assert!(page.contains("<svg"));
        // The theme's colors.
        let primary = crate::theme::vars::PRIMARY;
        let hex = format!("#{:02X}{:02X}{:02X}", primary.r, primary.g, primary.b);
        assert!(page.contains(&format!("--primary: {hex};")), "{hex}");
    }

    #[test]
    fn texts_cannot_close_the_script() {
        assert_eq!(
            json_string("a</script>\"b"),
            "\"a\\u003c/script\\u003e\\\"b\""
        );
    }

    /// The logo in the page has the proportions of `logo.rs` (the build reads its constants).
    #[test]
    fn the_logo_has_the_proportions_of_the_tv_logo() {
        let page = render("en");
        let svg = &page[page.find("<svg").unwrap()..page.find("</svg>").unwrap()];
        let expected = crate::logo::svg_numbers();
        for number in expected {
            assert!(svg.contains(&format!("\"{number}\"")), "{number} in {svg}");
        }
    }

    /// The page's QOI encoder (JavaScript), run by `node`, makes what the TV decodes back to the same
    /// pixels: runs, the color index, small and large differences, and a transparent pixel (composed
    /// over black by the page's canvas, so only RGB matters). Skipped without `node`.
    #[test]
    fn the_page_qoi_encoder_round_trips_through_the_tv_decoder() {
        let Ok(version) = std::process::Command::new("node").arg("--version").output() else {
            eprintln!("node not found: the page encoder was not checked");
            return;
        };
        assert!(version.status.success());
        let start = TEMPLATE.find("function qoiEncode(").unwrap();
        let end = start + TEMPLATE[start..].find("\n}\n").unwrap() + 2;
        let encoder = &TEMPLATE[start..end];
        let (w, h) = (257u32, 131u32);
        let mut rgba = Vec::new();
        let mut seed = 12345u32;
        for y in 0..h {
            for x in 0..w {
                seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
                let noise = (seed >> 16) as u8;
                let pixel = match (x / 32 + y / 32) % 4 {
                    0 => [10, 20, 30],                                            // runs
                    1 => [(x * 2) as u8, (y * 3) as u8, (x + y) as u8],           // small steps
                    2 => [noise, noise.wrapping_mul(3), 255 - noise],             // anything
                    _ => [[1, 2, 3], [200, 100, 0], [7, 7, 7]][(x % 3) as usize], // the index
                };
                rgba.extend_from_slice(&[pixel[0], pixel[1], pixel[2], 255]);
            }
        }
        let dir = std::env::temp_dir().join(format!("lume-qoi-js-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let (input, output) = (dir.join("in.rgba"), dir.join("out.qoi"));
        std::fs::write(&input, &rgba).unwrap();
        let script = format!(
            "{encoder}\nconst fs = require('fs');\nconst px = new Uint8Array(fs.readFileSync({:?}));\nfs.writeFileSync({:?}, qoiEncode(px, {w}, {h}));\n",
            input.to_string_lossy(),
            output.to_string_lossy()
        );
        let run = std::process::Command::new("node")
            .arg("-e")
            .arg(script)
            .output()
            .unwrap();
        assert!(
            run.status.success(),
            "{}",
            String::from_utf8_lossy(&run.stderr)
        );
        let encoded = std::fs::read(&output).unwrap();
        let (header, decoded) = qoi::decode_to_vec(&encoded).unwrap();
        assert_eq!((header.width, header.height), (w, h));
        assert_eq!(header.channels, qoi::Channels::Rgb);
        let rgb: Vec<u8> = rgba.chunks(4).flat_map(|p| [p[0], p[1], p[2]]).collect();
        assert!(decoded == rgb, "the decoded pixels differ");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
