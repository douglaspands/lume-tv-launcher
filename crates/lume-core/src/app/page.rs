//! The header of the secondary screens: breadcrumb path and title.

use super::painter::Painter;
use crate::logo;
use crate::render::Rect;
use crate::theme::{vars, StateFlags, THEME};

pub const LEFT: f32 = 64.0;
pub const WIDTH: f32 = 1152.0;
const BREADCRUMB_BASELINE: f32 = 52.0;
const TITLE_BASELINE: f32 = 99.0;
const SUBTITLE_BASELINE: f32 = 130.0;

/// Draws the logo, the path (`Settings › Home screen › Edit`) and the title.
/// `scroll_y` moves everything up when the page scrolls.
pub fn draw_header(
    p: &mut Painter,
    crumbs: &[&str],
    title: &str,
    subtitle: Option<&str>,
    scroll_y: f32,
) {
    logo::draw_mark(
        p.list,
        Rect::new(LEFT, 33.0 - scroll_y, 24.0, 24.0),
        vars::PRIMARY,
        vars::BG,
    );
    let muted = p.style(&THEME.breadcrumb, StateFlags::NONE);
    let current = p.style(&THEME.breadcrumb, StateFlags::ACTIVE);
    let mut x = LEFT + 36.0;
    for (index, crumb) in crumbs.iter().enumerate() {
        let last = index + 1 == crumbs.len();
        let style = if last { &current } else { &muted };
        let text_style = Painter::text_style(style);
        p.text.draw(
            p.list,
            crumb,
            x,
            BREADCRUMB_BASELINE - scroll_y,
            &text_style,
            style.color,
        );
        x += p.text.measure(crumb, &text_style);
        if !last {
            let separator = "›";
            x += 10.0;
            p.text.draw(
                p.list,
                separator,
                x,
                BREADCRUMB_BASELINE - scroll_y,
                &text_style,
                muted.color,
            );
            x += p.text.measure(separator, &text_style) + 10.0;
        }
    }
    let mut title_style = p.style(&THEME.section_title, StateFlags::NONE);
    title_style.font_size = 40.0;
    title_style.font_weight = 800;
    title_style.letter_spacing = -0.8;
    let shown = p
        .text
        .ellipsize(title, &Painter::text_style(&title_style), 900.0)
        .into_owned();
    p.text.draw(
        p.list,
        &shown,
        LEFT,
        TITLE_BASELINE - scroll_y,
        &Painter::text_style(&title_style),
        title_style.color,
    );
    if let Some(subtitle) = subtitle {
        let mut style = p.style(&THEME.breadcrumb, StateFlags::NONE);
        style.font_size = 18.0;
        let shown = p
            .text
            .ellipsize(subtitle, &Painter::text_style(&style), 960.0)
            .into_owned();
        p.text.draw(
            p.list,
            &shown,
            LEFT,
            SUBTITLE_BASELINE - scroll_y,
            &Painter::text_style(&style),
            style.color,
        );
    }
}
