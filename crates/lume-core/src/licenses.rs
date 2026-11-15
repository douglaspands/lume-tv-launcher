//! Third-party components shown on the About screen.

include!(concat!(env!("OUT_DIR"), "/third_party_generated.rs"));

/// Fonts, icons, images and the phone page's HEIC decoder shipped in the app: name, license. The
/// decoder's versions follow `assets/companion/heic/SOURCES.txt` (a test checks them).
pub const ASSETS: &[(&str, &str)] = &[
    ("Plus Jakarta Sans (font)", "SIL Open Font License 1.1"),
    ("Manrope (font, Cyrillic)", "SIL Open Font License 1.1"),
    ("Lucide (icons)", "ISC License"),
    ("Wallpapers (images)", "Unsplash License"),
    ("libheif 1.23.7 (HEIC decoder, phone page)", "LGPL-3.0"),
    ("libde265 1.1.3 (HEIC decoder, phone page)", "LGPL-3.0"),
];

static LINES: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();

/// Cached slice of the third-party list, as `name version: license`.
pub fn cached_lines() -> &'static [String] {
    LINES.get_or_init(|| {
        let mut lines: Vec<String> = ASSETS
            .iter()
            .map(|(name, license)| format!("{name}: {license}"))
            .collect();
        lines.extend(
            CRATES
                .iter()
                .map(|(name, version, license)| format!("{name} {version}: {license}")),
        );
        lines
    })
}

/// Every line of the third-party list, as `name version: license`.
pub fn lines() -> Vec<String> {
    cached_lines().to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_list_has_the_fonts_and_the_crates_of_the_app() {
        let lines = lines();
        assert!(lines[0].contains("Plus Jakarta Sans") && lines[0].contains("Open Font License"));
        assert!(lines[1].contains("Manrope") && lines[1].contains("Open Font License"));
        assert!(lines[2].contains("Lucide") && lines[2].contains("ISC"));
        // The renderer's crates are shipped, with their licenses.
        for name in ["fontdue", "glow", "bytemuck", "ttf-parser"] {
            assert!(
                lines.iter().any(|l| l.starts_with(&format!("{name} "))),
                "{name} is missing: {lines:#?}"
            );
        }
        // lume's own crates are not third-party.
        assert!(!lines.iter().any(|l| l.starts_with("lume-")));
        // Every entry says which license it is under.
        assert!(CRATES.iter().all(|(_, _, license)| !license.is_empty()));
    }

    #[test]
    fn the_heic_decoder_is_listed_with_the_versions_it_was_built_from() {
        let sources = include_str!("../assets/companion/heic/SOURCES.txt");
        for library in ["libheif", "libde265"] {
            let built = sources
                .lines()
                .find_map(|l| l.trim().strip_prefix(&format!("{library} ")))
                .and_then(|rest| rest.split_whitespace().next())
                .unwrap_or_else(|| panic!("{library} is not in SOURCES.txt"));
            let line = format!("{library} {built} (HEIC decoder, phone page): LGPL-3.0");
            assert!(lines().contains(&line), "{line} is missing");
        }
    }

    #[test]
    fn crates_are_listed_once_and_sorted() {
        let names: Vec<&str> = CRATES.iter().map(|(n, _, _)| *n).collect();
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(names, sorted);
        let mut deduped = CRATES.to_vec();
        deduped.dedup();
        assert_eq!(deduped.len(), CRATES.len());
    }
}
