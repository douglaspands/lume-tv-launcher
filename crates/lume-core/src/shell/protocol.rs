//! The text format the Java bridge and the Rust side use to talk to each other.
//!
//! Everything is plain text so the JNI boundary is a single call with strings:
//! lists have one record per line, fields are separated by tabs.

use crate::catalog::color::ICON_SIZE;
use crate::catalog::Discovered;

/// What the `changed` command answered.
#[derive(Debug, PartialEq, Eq)]
pub enum Changed {
    /// The system has no baseline yet: scan everything.
    Unknown,
    /// Packages that were installed, updated or removed since the last call.
    Packages(Vec<String>),
}

/// Parses the `apps` and `query` answers: `package<TAB>label<TAB>activity<TAB>1|0` per line.
/// Malformed lines are skipped.
pub fn parse_apps(text: &str) -> Vec<Discovered> {
    text.lines()
        .filter_map(|line| {
            let mut fields = line.split('\t');
            let (package, label, activity, leanback) = (
                fields.next()?,
                fields.next()?,
                fields.next()?,
                fields.next()?,
            );
            if package.is_empty() || activity.is_empty() || fields.next().is_some() {
                return None;
            }
            Some(Discovered {
                package: package.to_string(),
                label: if label.is_empty() {
                    package.to_string()
                } else {
                    label.to_string()
                },
                activity: activity.to_string(),
                leanback: leanback == "1",
                icon: None,
            })
        })
        .collect()
}

/// Parses the `icon` answer: `ICON_SIZE` squared pixels as 8 hexadecimal digits each (`AARRGGBB`).
pub fn parse_icon(text: &str) -> Option<Vec<u32>> {
    let text = text.trim();
    if text.len() != ICON_SIZE * ICON_SIZE * 8 || !text.is_ascii() {
        return None;
    }
    text.as_bytes()
        .chunks(8)
        .map(|chunk| u32::from_str_radix(std::str::from_utf8(chunk).ok()?, 16).ok())
        .collect()
}

pub fn parse_changed(text: &str) -> Changed {
    let text = text.trim();
    if text == "-" {
        return Changed::Unknown;
    }
    Changed::Packages(
        text.lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(String::from)
            .collect(),
    )
}

/// Parses the `clock` answer: `<1|0 for 24-hour><TAB><UTC offset in seconds>`.
pub fn parse_clock(text: &str) -> Option<(bool, i32)> {
    let (is_24h, offset) = text.trim().split_once('\t')?;
    Some((is_24h == "1", offset.parse().ok()?))
}

/// Parses the `locales` answer: BCP-47 tags separated by commas, in order of preference.
pub fn parse_locales(text: &str) -> Vec<String> {
    text.trim()
        .split(',')
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(String::from)
        .collect()
}

/// The argument of the `start` command.
pub fn start_argument(package: &str, activity: &str) -> String {
    format!("{package}\n{activity}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apps_are_parsed_one_per_line() {
        let apps = parse_apps(
            "com.netflix.ninja\tNetflix\tcom.netflix.Main\t1\ncom.demo.x\tDemo\t.Main\t0\n",
        );
        assert_eq!(apps.len(), 2);
        assert_eq!(apps[0].package, "com.netflix.ninja");
        assert_eq!(apps[0].label, "Netflix");
        assert_eq!(apps[0].activity, "com.netflix.Main");
        assert!(apps[0].leanback && !apps[1].leanback);
        assert!(apps[0].icon.is_none());
    }

    #[test]
    fn labels_in_any_script_survive() {
        let apps = parse_apps("tv.iqiyi\t爱奇艺\t.Main\t1\nru.yandex\tЯндекс\t.Main\t1\n");
        assert_eq!(apps[0].label, "爱奇艺");
        assert_eq!(apps[1].label, "Яндекс");
    }

    #[test]
    fn malformed_lines_are_skipped_and_empty_labels_use_the_package() {
        let apps = parse_apps("\nnot enough fields\n\tNo package\t.Main\t1\npkg.a\t\t.Main\t1\npkg.b\tB\t.Main\t1\textra\n");
        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].label, "pkg.a");
        assert!(parse_apps("").is_empty());
    }

    #[test]
    fn icons_are_parsed_from_hex() {
        let hex = "FF336699".repeat(ICON_SIZE * ICON_SIZE);
        let pixels = parse_icon(&hex).unwrap();
        assert_eq!(pixels.len(), ICON_SIZE * ICON_SIZE);
        assert!(pixels.iter().all(|&p| p == 0xFF33_6699));
        assert!(parse_icon("").is_none());
        assert!(parse_icon(&"FF336699".repeat(10)).is_none());
        assert!(parse_icon(&"ZZZZZZZZ".repeat(ICON_SIZE * ICON_SIZE)).is_none());
        assert!(
            parse_icon(&format!("{hex}\n")).is_some(),
            "a trailing newline is fine"
        );
    }

    #[test]
    fn changed_packages() {
        assert_eq!(parse_changed("-"), Changed::Unknown);
        assert_eq!(parse_changed("-\n"), Changed::Unknown);
        assert_eq!(parse_changed(""), Changed::Packages(vec![]));
        assert_eq!(
            parse_changed("a.b\n\nc.d\n"),
            Changed::Packages(vec!["a.b".into(), "c.d".into()])
        );
    }

    #[test]
    fn clock_and_locales() {
        assert_eq!(parse_clock("1\t-10800"), Some((true, -10800)));
        assert_eq!(parse_clock("0\t19800\n"), Some((false, 19800)));
        assert_eq!(parse_clock("garbage"), None);
        assert_eq!(parse_clock("1\tx"), None);
        assert_eq!(parse_locales("pt-BR,en-US"), ["pt-BR", "en-US"]);
        assert_eq!(parse_locales(" ja , ,pt-BR\n"), ["ja", "pt-BR"]);
        assert!(parse_locales("").is_empty());
    }

    #[test]
    fn the_start_argument_has_two_lines() {
        assert_eq!(start_argument("a.b", "a.b.Main"), "a.b\na.b.Main");
    }
}
