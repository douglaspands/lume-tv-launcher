//! The carousel's slides: which apps get one and what each shows.

use super::config::HomeConfig;
use crate::catalog::{App, Catalog};
use crate::i18n::{Arg, PluralId, StringId, Translator};
use crate::theme::Color;
use crate::usage::UsageHistory;

/// What pressing "Open" on a slide does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SlideAction {
    OpenApp(String),
    /// The welcome slide leads to the Apps tab.
    ShowApps,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Slide {
    /// The label at the top: the app's name (or "lume" on the welcome slide).
    pub badge: String,
    pub title: String,
    /// The line below the title.
    pub context: String,
    /// The app's color, which tints the slide's gradient.
    pub color: Color,
    pub button: String,
    pub action: SlideAction,
}

const MINUTE: u64 = 60;
const HOUR: u64 = 60 * MINUTE;
const DAY: u64 = 24 * HOUR;

/// "Opened 2 hours ago", from how long ago the last opening was.
pub fn opened_ago(translator: &Translator, seconds_ago: u64) -> String {
    let (id, count) = if seconds_ago < MINUTE {
        return translator
            .get(StringId::CarouselContextJustNow)
            .into_owned();
    } else if seconds_ago < HOUR {
        (PluralId::CarouselContextMinutesAgo, seconds_ago / MINUTE)
    } else if seconds_ago < DAY {
        (PluralId::CarouselContextHoursAgo, seconds_ago / HOUR)
    } else {
        (PluralId::CarouselContextDaysAgo, seconds_ago / DAY)
    };
    let count = count as i64;
    translator.plural(id, count, &[Arg::Int(count)])
}

/// The context line of an app's slide, chosen from the local history in this order:
/// new on the TV, most used of the week, opened some time ago, and a hint to start.
fn context_line(
    app: &App,
    most_used: Option<&str>,
    usage: &UsageHistory,
    now: u64,
    translator: &Translator,
) -> String {
    if usage.is_new(&app.package, now) {
        translator.get(StringId::CarouselContextNew).into_owned()
    } else if most_used == Some(app.package.as_str()) {
        translator
            .get(StringId::CarouselContextMostUsed)
            .into_owned()
    } else if let Some(last) = usage.last_opened(&app.package) {
        opened_ago(translator, now.saturating_sub(last))
    } else {
        translator.get(StringId::CarouselContextStart).into_owned()
    }
}

/// The most recently installed app (seen for the first time in the last 7 days) that has no slide.
fn newest_unlisted<'a>(
    config: &HomeConfig,
    catalog: &'a Catalog,
    usage: &UsageHistory,
    now: u64,
) -> Option<&'a App> {
    catalog
        .apps()
        .iter()
        .filter(|app| {
            !config.carousel_apps.contains(&app.package) && usage.is_new(&app.package, now)
        })
        .max_by_key(|app| usage.first_seen(&app.package))
}

/// Builds the slides for the current configuration, catalog and history.
///
/// - one slide per selected app that is installed, in the configured order;
/// - a "New: <app>" slide first, for the newest app installed in the last 7 days that is not
///   in the carousel;
/// - when there are none, a single welcome slide that leads to the Apps tab.
pub fn build_slides(
    config: &HomeConfig,
    catalog: &Catalog,
    usage: &UsageHistory,
    now: u64,
    translator: &Translator,
) -> Vec<Slide> {
    let open = translator.get(StringId::CarouselOpen).into_owned();
    let most_used = usage.most_used(catalog.apps().iter().map(|a| a.package.as_str()), now);
    let mut slides = Vec::new();

    if let Some(app) = newest_unlisted(config, catalog, usage, now) {
        slides.push(Slide {
            badge: app.label.clone(),
            title: translator.format(StringId::CarouselNewApp, &[Arg::Str(&app.label)]),
            context: translator.get(StringId::CarouselContextNew).into_owned(),
            color: app.color,
            button: open.clone(),
            action: SlideAction::OpenApp(app.package.clone()),
        });
    }
    for package in &config.carousel_apps {
        let Some(app) = catalog.get(package) else {
            continue;
        };
        slides.push(Slide {
            badge: app.label.clone(),
            title: app.label.clone(),
            context: context_line(app, most_used, usage, now, translator),
            color: app.color,
            button: open.clone(),
            action: SlideAction::OpenApp(app.package.clone()),
        });
    }

    if slides.is_empty() {
        slides.push(Slide {
            badge: translator.get(StringId::AppName).into_owned(),
            title: translator.get(StringId::CarouselWelcomeTitle).into_owned(),
            context: translator
                .get(StringId::CarouselWelcomeContext)
                .into_owned(),
            color: crate::theme::vars::HERO_MID,
            button: translator.get(StringId::CarouselWelcomeButton).into_owned(),
            action: SlideAction::ShowApps,
        });
    }
    slides
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::Discovered;
    use crate::home::config::{Interval, Section, SectionSetting};

    const NOW: u64 = 1_800_000_000;
    const HOUR: u64 = 3600;
    const DAY: u64 = 86_400;
    const NETFLIX: &str = "com.netflix.ninja";
    const YOUTUBE: &str = "com.google.android.youtube.tv";
    const PRIME: &str = "com.amazon.amazonvideo.livingroom";
    const GLOBOPLAY: &str = "com.globo.globotv";

    fn catalog(packages: &[(&str, &str)]) -> Catalog {
        let entries: Vec<Discovered> = packages
            .iter()
            .map(|(package, label)| Discovered {
                package: package.to_string(),
                label: label.to_string(),
                activity: "Main".into(),
                leanback: true,
                icon: None,
            })
            .collect();
        Catalog::from_discovered(&entries, "app.lume.tv")
    }

    fn config(carousel: &[&str]) -> HomeConfig {
        HomeConfig {
            sections: vec![
                SectionSetting {
                    section: Section::Carousel,
                    visible: true,
                },
                SectionSetting {
                    section: Section::Favorites,
                    visible: true,
                },
            ],
            carousel_apps: carousel.iter().map(|s| s.to_string()).collect(),
            interval: Interval::Seconds5,
            favorites: vec![],
        }
    }

    fn old_usage(packages: &[&str]) -> UsageHistory {
        let mut usage = UsageHistory::new();
        usage.mark_baseline(packages.iter().copied());
        usage
    }

    fn pt() -> Translator {
        Translator::new(&["pt-BR"])
    }

    fn en() -> Translator {
        Translator::new(&["en"])
    }

    #[test]
    fn a_selected_installed_app_gets_a_slide() {
        let catalog = catalog(&[(NETFLIX, "Netflix")]);
        let slides = build_slides(
            &config(&[NETFLIX]),
            &catalog,
            &old_usage(&[NETFLIX]),
            NOW,
            &pt(),
        );
        assert_eq!(slides.len(), 1);
        let slide = &slides[0];
        assert_eq!(
            (
                slide.title.as_str(),
                slide.badge.as_str(),
                slide.button.as_str()
            ),
            ("Netflix", "Netflix", "Abrir")
        );
        assert_eq!(slide.color, crate::theme::vars::BRAND_NETFLIX);
        assert_eq!(slide.action, SlideAction::OpenApp(NETFLIX.into()));
    }

    #[test]
    fn slides_follow_the_configured_order_and_skip_uninstalled_apps() {
        let catalog = catalog(&[(NETFLIX, "Netflix"), (YOUTUBE, "YouTube")]);
        let usage = old_usage(&[NETFLIX, YOUTUBE]);
        let slides = build_slides(
            &config(&[YOUTUBE, PRIME, NETFLIX]),
            &catalog,
            &usage,
            NOW,
            &pt(),
        );
        let titles: Vec<_> = slides.iter().map(|s| s.title.as_str()).collect();
        assert_eq!(titles, ["YouTube", "Netflix"]);
    }

    #[test]
    fn the_hint_is_shown_when_there_is_no_history() {
        let catalog = catalog(&[(NETFLIX, "Netflix")]);
        let slides = build_slides(
            &config(&[NETFLIX]),
            &catalog,
            &old_usage(&[NETFLIX]),
            NOW,
            &pt(),
        );
        assert_eq!(slides[0].context, "Toque em Abrir para começar");
        let slides = build_slides(
            &config(&[NETFLIX]),
            &catalog,
            &old_usage(&[NETFLIX]),
            NOW,
            &en(),
        );
        assert_eq!(slides[0].context, "Select Open to get started");
    }

    #[test]
    fn the_most_used_app_of_the_week() {
        let catalog = catalog(&[(NETFLIX, "Netflix"), (YOUTUBE, "YouTube")]);
        let mut usage = old_usage(&[NETFLIX, YOUTUBE]);
        usage.record_open(YOUTUBE, NOW - 3 * DAY);
        usage.record_open(YOUTUBE, NOW - 2 * DAY);
        usage.record_open(NETFLIX, NOW - 2 * DAY);
        let slides = build_slides(&config(&[YOUTUBE, NETFLIX]), &catalog, &usage, NOW, &pt());
        assert_eq!(slides[0].context, "Mais usado da semana");
        // Netflix was opened too, but is not the most used: it shows when it was opened.
        assert_eq!(slides[1].context, "Aberto há 2 dias");
    }

    #[test]
    fn opened_some_time_ago_uses_the_right_unit_and_plural() {
        let catalog = catalog(&[(PRIME, "Prime Video"), (NETFLIX, "Netflix")]);
        let mut usage = old_usage(&[PRIME, NETFLIX]);
        // Netflix has more weekly opens, so Prime Video is not "most used".
        usage.record_open(NETFLIX, NOW - 5 * DAY);
        usage.record_open(NETFLIX, NOW - 4 * DAY);
        usage.record_open(PRIME, NOW - 2 * HOUR);
        let slides = build_slides(&config(&[PRIME]), &catalog, &usage, NOW, &pt());
        assert_eq!(slides[0].context, "Aberto há 2 horas");
        let slides = build_slides(&config(&[PRIME]), &catalog, &usage, NOW, &en());
        assert_eq!(slides[0].context, "Opened 2 hours ago");
    }

    #[test]
    fn time_units_and_singulars() {
        let (pt, en) = (pt(), en());
        assert_eq!(opened_ago(&pt, 10), "Aberto agora há pouco");
        assert_eq!(opened_ago(&pt, 59), "Aberto agora há pouco");
        assert_eq!(opened_ago(&pt, 60), "Aberto há 1 minuto");
        assert_eq!(opened_ago(&pt, 5 * 60 + 30), "Aberto há 5 minutos");
        assert_eq!(opened_ago(&pt, 3600), "Aberto há 1 hora");
        assert_eq!(opened_ago(&pt, 3 * 3600 + 100), "Aberto há 3 horas");
        assert_eq!(opened_ago(&pt, 24 * 3600), "Aberto há 1 dia");
        assert_eq!(opened_ago(&pt, 3 * 86_400), "Aberto há 3 dias");
        assert_eq!(opened_ago(&en, 3600), "Opened 1 hour ago");
        assert_eq!(opened_ago(&en, 7200), "Opened 2 hours ago");
        assert_eq!(opened_ago(&en, 30 * 86_400), "Opened 30 days ago");
    }

    #[test]
    fn a_new_app_in_the_carousel_says_new_on_your_tv() {
        let catalog = catalog(&[(NETFLIX, "Netflix")]);
        let mut usage = UsageHistory::new();
        usage.mark_seen(NETFLIX, NOW - DAY);
        // Even if it was also opened, being new comes first.
        usage.record_open(NETFLIX, NOW - HOUR);
        let slides = build_slides(&config(&[NETFLIX]), &catalog, &usage, NOW, &pt());
        assert_eq!(slides[0].context, "Novo no seu TV");
    }

    #[test]
    fn a_recently_installed_app_outside_the_carousel_gets_a_new_slide_first() {
        let catalog = catalog(&[(NETFLIX, "Netflix"), (GLOBOPLAY, "Globoplay")]);
        let mut usage = old_usage(&[NETFLIX]);
        usage.mark_seen(GLOBOPLAY, NOW - DAY);
        let slides = build_slides(&config(&[NETFLIX]), &catalog, &usage, NOW, &pt());
        assert_eq!(slides.len(), 2);
        assert_eq!(slides[0].title, "Novo: Globoplay");
        assert_eq!(slides[0].badge, "Globoplay");
        assert_eq!(slides[0].button, "Abrir");
        assert_eq!(slides[0].action, SlideAction::OpenApp(GLOBOPLAY.into()));
        assert_eq!(slides[1].title, "Netflix");
    }

    #[test]
    fn only_the_newest_installation_gets_a_slide() {
        let catalog = catalog(&[
            (YOUTUBE, "YouTube"),
            (GLOBOPLAY, "Globoplay"),
            (PRIME, "Prime Video"),
        ]);
        let mut usage = old_usage(&[YOUTUBE]);
        usage.mark_seen(GLOBOPLAY, NOW - 3 * DAY);
        usage.mark_seen(PRIME, NOW - DAY);
        let slides = build_slides(&config(&[YOUTUBE]), &catalog, &usage, NOW, &en());
        assert_eq!(slides.len(), 2);
        assert_eq!(slides[0].title, "New: Prime Video");
    }

    #[test]
    fn installations_older_than_a_week_get_no_slide() {
        let catalog = catalog(&[(NETFLIX, "Netflix"), (GLOBOPLAY, "Globoplay")]);
        let mut usage = old_usage(&[NETFLIX]);
        usage.mark_seen(GLOBOPLAY, NOW - 8 * DAY);
        let slides = build_slides(&config(&[NETFLIX]), &catalog, &usage, NOW, &pt());
        assert_eq!(slides.len(), 1);
    }

    #[test]
    fn a_recent_app_already_in_the_carousel_is_not_duplicated() {
        let catalog = catalog(&[(GLOBOPLAY, "Globoplay")]);
        let mut usage = UsageHistory::new();
        usage.mark_seen(GLOBOPLAY, NOW - DAY);
        let slides = build_slides(&config(&[GLOBOPLAY]), &catalog, &usage, NOW, &pt());
        assert_eq!(slides.len(), 1);
        assert_eq!(slides[0].title, "Globoplay");
    }

    #[test]
    fn the_welcome_slide_leads_to_the_apps_tab() {
        // Nothing selected is installed and nothing was installed recently.
        let catalog = catalog(&[("com.example.x", "Example")]);
        let usage = old_usage(&["com.example.x"]);
        let slides = build_slides(&config(&[NETFLIX]), &catalog, &usage, NOW, &pt());
        assert_eq!(slides.len(), 1);
        assert_eq!(slides[0].action, SlideAction::ShowApps);
        assert_eq!(slides[0].title, "Bem-vindo ao lume");
        assert_eq!(slides[0].badge, "lume");
        // An empty catalog and config also gets the welcome slide.
        let slides = build_slides(
            &config(&[]),
            &Catalog::new("app.lume.tv"),
            &UsageHistory::new(),
            NOW,
            &en(),
        );
        assert_eq!(slides[0].title, "Welcome to lume");
        assert_eq!(slides[0].button, "See all apps");
    }

    #[test]
    fn slides_carry_no_third_party_imagery() {
        // A slide is plain text and a color: nothing else can be rendered from it.
        let catalog = catalog(&[(NETFLIX, "Netflix")]);
        let slide = &build_slides(
            &config(&[NETFLIX]),
            &catalog,
            &old_usage(&[NETFLIX]),
            NOW,
            &pt(),
        )[0];
        let Slide {
            badge,
            title,
            context,
            color: _,
            button,
            action: _,
        } = slide;
        assert!(![badge, title, context, button]
            .iter()
            .any(|s| s.contains("http")));
    }
}
