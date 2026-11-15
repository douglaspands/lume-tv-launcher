//! What the tool screens share: the layout of the settings (items at the left, a `.panel` at the
//! right), the flash warning (`.menu` card) and the hint at the bottom of a full-screen pattern
//! (`.toast`). See "Telas de ferramenta na régua" in the `design-system` spec.

use super::ToolKind;
use crate::app::page::{self, LEFT, WIDTH};
use crate::app::painter::Painter;
use crate::app::{ids, LumeApp};
use crate::i18n::StringId;
use crate::render::Rect;
use crate::text::Align;
use crate::theme::{vars, ComputedStyle, Paint, StateFlags, THEME};
use crate::ui::focus::{FocusId, Focusable, Row};

/// The two columns of the settings, with a 48 px gutter.
pub const COLUMN_WIDTH: f32 = (WIDTH - 48.0) / 2.0;
/// The tool screens have no subtitle: their content starts 36 below the title's baseline, as the
/// editor's ("Canvas, margens e grade" in the `design-system` spec).
pub const ITEMS_TOP: f32 = page::PLAIN_CONTENT_TOP;
pub const ITEM_HEIGHT: f32 = 72.0;
pub const ITEM_PITCH: f32 = 88.0;
pub const PANEL_X: f32 = LEFT + COLUMN_WIDTH + 48.0;
pub const PANEL_WIDTH: f32 = COLUMN_WIDTH;
/// The panel's inner margins (as the statistics panel).
pub const PANEL_PAD_X: f32 = 32.0;
pub const PANEL_PAD_TOP: f32 = 24.0;
/// A label/value row of the panel and the gap to the next one (as the statistics panel).
pub const PANEL_ROW_HEIGHT: f32 = 24.0;
pub const PANEL_ROW_PITCH: f32 = 40.0;
/// Where the item list is clipped when it scrolls: a little above the first item, so its focus
/// ring is not cut.
pub const LIST_VIEW_TOP: f32 = ITEMS_TOP - 16.0;
/// How long the hint over a full-screen pattern stays (`.toast`, 3 s).
pub const HINT_MS: u64 = 3000;

/// The rectangle of the item at `index` of the left column (before scrolling).
pub fn item_rect(index: usize) -> Rect {
    Rect::new(
        LEFT,
        ITEMS_TOP + index as f32 * ITEM_PITCH,
        COLUMN_WIDTH,
        ITEM_HEIGHT,
    )
}

/// The right column's panel, `height` tall.
pub fn panel_rect(height: f32) -> Rect {
    Rect::new(PANEL_X, ITEMS_TOP, PANEL_WIDTH, height)
}

/// Remaining time as `m:ss` (or `h:mm:ss`).
pub fn format_remaining(ms: u64) -> String {
    let secs = ms.div_ceil(1000);
    let (h, m, s) = (secs / 3600, (secs / 60) % 60, secs % 60);
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m}:{s:02}")
    }
}

/// A row of the panel: the label in `--text-muted` at the left, the value in `--text` at the right.
pub fn panel_row(p: &mut Painter, line: Rect, label: &str, value: &str) {
    let label_style = p.style(&THEME.breadcrumb, StateFlags::NONE);
    let mut value_style = label_style;
    value_style.color = vars::TEXT;
    value_style.font_weight = 700;
    p.text(label, line, Align::Start, &label_style);
    p.text(value, line, Align::End, &value_style);
}

/// A bar like the memory bar of the statistics panel; `None` leaves only the track.
pub fn panel_bar(p: &mut Painter, bar: Rect, fraction: Option<f32>) {
    p.list
        .fill(bar, vars::SHAPE_FULL, Paint::Solid(vars::CONTROL));
    if let Some(fraction) = fraction {
        let filled = Rect::new(
            bar.x,
            bar.y,
            (bar.w * fraction.clamp(0.0, 1.0)).max(8.0),
            bar.h,
        );
        p.list
            .fill(filled, vars::SHAPE_FULL, Paint::Solid(vars::PRIMARY));
    }
}

/// Draws the panel box and returns its inner area (inside the margins).
pub fn draw_panel(p: &mut Painter, panel: Rect) -> Rect {
    let style = p.style(&THEME.panel, StateFlags::NONE);
    p.draw_box(panel, &style);
    Rect::new(
        panel.x + PANEL_PAD_X,
        panel.y + PANEL_PAD_TOP,
        panel.w - 2.0 * PANEL_PAD_X,
        panel.h - 2.0 * PANEL_PAD_TOP,
    )
}

/// Draws the panel heading (`.card-heading`, in capitals) at the top of the inner area.
pub fn panel_heading(p: &mut Painter, inner: Rect, text: &str) {
    let heading = p.style(&THEME.card_heading, StateFlags::NONE);
    let line = Rect::new(inner.x, inner.y, inner.w, 20.0);
    // Card headings are capitals, as the statistics panel's ("RECURSOS DA TV").
    p.text(&text.to_uppercase(), line, Align::Start, &heading);
}

/// Draws `text` wrapped to the width of `rect` from its top, one line every `pitch`, and returns
/// how many lines it took.
pub fn wrapped_text(
    p: &mut Painter,
    text: &str,
    rect: Rect,
    pitch: f32,
    style: &ComputedStyle,
) -> usize {
    let text_style = Painter::text_style(style);
    let lines = crate::app::editor::wrap(p, text, &text_style, rect.w);
    for (index, line) in lines.iter().enumerate() {
        let row = Rect::new(rect.x, rect.y + index as f32 * pitch, rect.w, pitch);
        p.text(line, row, Align::Start, style);
    }
    lines.len()
}

impl LumeApp {
    /// The header of a tool: "Configurações › Ferramentas › <tool>" and the tool's name.
    pub(in crate::app) fn draw_tool_header(&self, p: &mut Painter, kind: ToolKind) {
        let t = self.translator;
        page::draw_header(
            p,
            &[
                &t.get(StringId::SettingsTitle),
                &t.get(StringId::SettingsTools),
                &t.get(kind.name()),
            ],
            &t.get(kind.name()),
            None,
            0.0,
        );
    }

    /// Lays out and draws the action items of the left column (`.settings-item`, 72 tall on the
    /// 88 pitch), one per row, and makes them the focus rows. `scroll` moves them up; the items
    /// outside the canvas are not drawn. Returns their rectangles (before scrolling).
    pub(in crate::app) fn draw_tool_items(
        &mut self,
        p: &mut Painter,
        items: &[(FocusId, String)],
        scroll: f32,
    ) -> Vec<Rect> {
        let rects: Vec<Rect> = (0..items.len()).map(item_rect).collect();
        let rows = items
            .iter()
            .zip(&rects)
            .map(|((id, _), rect)| Row::new(vec![Focusable::new(*id, *rect)]))
            .collect();
        self.update_focus(rows);
        let bottom = p.display.visible_rect().bottom();
        let clip = scroll > 0.0 || rects.last().is_some_and(|r| r.bottom() > bottom);
        if clip {
            p.list.push_clip(Rect::new(
                LEFT - 16.0,
                LIST_VIEW_TOP,
                COLUMN_WIDTH + 32.0,
                bottom - LIST_VIEW_TOP,
            ));
        }
        for ((id, label), rect) in items.iter().zip(&rects) {
            let rect = rect.translate(0.0, -scroll);
            if rect.bottom() < LIST_VIEW_TOP || rect.y > bottom {
                continue;
            }
            let progress = self.focus_progress(*id);
            let style = p.focus_style(&THEME.settings_item, progress);
            p.draw_box(rect, &style);
            let inner = Rect::new(
                rect.x + style.padding[3],
                rect.y,
                rect.w - style.padding[3] - style.padding[1],
                rect.h,
            );
            p.text(label, inner, Align::Start, &style);
        }
        if clip {
            p.list.pop_clip();
        }
        rects
    }
}

/// The warning before a mode that flashes colors quickly (`diagnostic-tools`, "Proteção contra
/// flashes"): a `.menu` card with the text and "Start"/"Cancel", the focus on "Cancel".
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FlashWarning {
    /// Whether the focus is on "Start" (it starts on "Cancel").
    on_start: bool,
}

/// What a key did to the warning.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WarningAnswer {
    /// Still open.
    Open,
    /// OK on "Start".
    Start,
    /// OK on "Cancel", or Back.
    Cancel,
}

const WARNING_WIDTH: f32 = 560.0;
const WARNING_PADDING: f32 = 32.0;
const WARNING_TITLE_HEIGHT: f32 = 40.0;
const WARNING_BUTTON_HEIGHT: f32 = 56.0;
const WARNING_BUTTON_WIDTH: f32 = 160.0;
const WARNING_GAP: f32 = 24.0;
const WARNING_BUTTON_GAP: f32 = 16.0;

impl FlashWarning {
    pub fn new() -> FlashWarning {
        FlashWarning::default()
    }

    /// Whether the focus is on "Start".
    #[cfg(test)]
    pub fn on_start(&self) -> bool {
        self.on_start
    }

    pub fn key(&mut self, key: crate::keys::Key) -> WarningAnswer {
        use crate::keys::Key;
        match key {
            Key::Left | Key::Right | Key::Up | Key::Down => {
                self.on_start = !self.on_start;
                WarningAnswer::Open
            }
            Key::Ok if self.on_start => WarningAnswer::Start,
            Key::Ok | Key::Back => WarningAnswer::Cancel,
            _ => WarningAnswer::Open,
        }
    }

    /// Draws the card in the middle of the canvas.
    pub fn draw(&self, p: &mut Painter, translator: &crate::i18n::Translator) {
        let card = p.style(&THEME.menu, StateFlags::NONE);
        let note = p.style(&THEME.menu_note, StateFlags::NONE);
        let mut title = p.style(&THEME.menu_item, StateFlags::NONE);
        title.font_weight = 700;
        let inner_w = WARNING_WIDTH - 2.0 * WARNING_PADDING;
        let text = translator.get(StringId::ToolsFlashText);
        let lines = crate::app::editor::wrap(p, &text, &Painter::text_style(&note), inner_w);
        let pitch = note.line_height.max(24.0);
        let height = WARNING_PADDING
            + WARNING_TITLE_HEIGHT
            + lines.len() as f32 * pitch
            + WARNING_GAP
            + WARNING_BUTTON_HEIGHT
            + WARNING_PADDING;
        let rect = Rect::new(
            (crate::display::CANVAS_WIDTH - WARNING_WIDTH) / 2.0,
            ((crate::display::CANVAS_HEIGHT - height) / 2.0 / 4.0).round() * 4.0,
            WARNING_WIDTH,
            height,
        );
        p.draw_box(rect, &card);
        let mut y = rect.y + WARNING_PADDING;
        let x = rect.x + WARNING_PADDING;
        p.text(
            &translator.get(StringId::ToolsFlashTitle),
            Rect::new(x, y, inner_w, WARNING_TITLE_HEIGHT),
            Align::Start,
            &title,
        );
        y += WARNING_TITLE_HEIGHT;
        for line in &lines {
            p.text(line, Rect::new(x, y, inner_w, pitch), Align::Start, &note);
            y += pitch;
        }
        y += WARNING_GAP;
        let buttons = [
            (StringId::ToolsStart, self.on_start, ids::WARNING_START),
            (StringId::ToolsCancel, !self.on_start, ids::WARNING_CANCEL),
        ];
        let mut bx =
            rect.right() - WARNING_PADDING - 2.0 * WARNING_BUTTON_WIDTH - WARNING_BUTTON_GAP;
        for (label, focused, _) in buttons {
            let style = p.focus_style(&THEME.menu_item, if focused { 1.0 } else { 0.0 });
            let button = Rect::new(bx, y, WARNING_BUTTON_WIDTH, WARNING_BUTTON_HEIGHT);
            p.draw_box(button, &style);
            p.text(&translator.get(label), button, Align::Center, &style);
            bx += WARNING_BUTTON_WIDTH + WARNING_BUTTON_GAP;
        }
    }
}

/// A short text at the bottom of a full-screen pattern (`.toast`) that goes away after 3 s.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Hint {
    text: String,
    until: u64,
}

impl Hint {
    pub fn show(&mut self, text: String, now_ms: u64) {
        self.text = text;
        self.until = now_ms + HINT_MS;
    }

    pub fn hide(&mut self) {
        self.until = 0;
    }

    pub fn is_visible(&self, now_ms: u64) -> bool {
        now_ms < self.until
    }

    /// When the hint must go (a frame is due then), while it is visible.
    pub fn deadline(&self, now_ms: u64) -> Option<u64> {
        self.is_visible(now_ms).then_some(self.until)
    }

    /// Draws the hint when it is visible, centered at the bottom of the canvas.
    pub fn draw(&self, p: &mut Painter, now_ms: u64) {
        if !self.is_visible(now_ms) || self.text.is_empty() {
            return;
        }
        let style = p.style(&THEME.toast, StateFlags::NONE);
        let width = p.measure(&self.text, &style) + style.padding[1] + style.padding[3];
        let height = style.font_size * 1.6 + style.padding[0] + style.padding[2];
        let visible = p.display.visible_rect();
        let rect = Rect::new(
            visible.x + (visible.w - width) / 2.0,
            720.0 - 96.0,
            width,
            height,
        );
        p.draw_box(rect, &style);
        p.text(&self.text, rect, Align::Center, &style);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys::Key;

    #[test]
    fn the_layout_is_on_the_ruler() {
        crate::app::assert_on_ruler(&[
            ("COLUMN_WIDTH", COLUMN_WIDTH),
            ("ITEMS_TOP", ITEMS_TOP),
            ("ITEM_HEIGHT", ITEM_HEIGHT),
            ("ITEM_PITCH", ITEM_PITCH),
            ("PANEL_X", PANEL_X),
            ("PANEL_PAD_X", PANEL_PAD_X),
            ("PANEL_PAD_TOP", PANEL_PAD_TOP),
            ("PANEL_ROW_HEIGHT", PANEL_ROW_HEIGHT),
            ("PANEL_ROW_PITCH", PANEL_ROW_PITCH),
            ("LIST_VIEW_TOP", LIST_VIEW_TOP),
            ("WARNING_WIDTH", WARNING_WIDTH),
            ("WARNING_PADDING", WARNING_PADDING),
            ("WARNING_TITLE_HEIGHT", WARNING_TITLE_HEIGHT),
            ("WARNING_BUTTON_HEIGHT", WARNING_BUTTON_HEIGHT),
            ("WARNING_BUTTON_WIDTH", WARNING_BUTTON_WIDTH),
            ("WARNING_GAP", WARNING_GAP),
            ("WARNING_BUTTON_GAP", WARNING_BUTTON_GAP),
        ]);
    }

    #[test]
    fn the_warning_starts_on_cancel_and_back_refuses() {
        let mut warning = FlashWarning::new();
        assert!(!warning.on_start());
        assert_eq!(warning.key(Key::Ok), WarningAnswer::Cancel);
        let mut warning = FlashWarning::new();
        assert_eq!(warning.key(Key::Left), WarningAnswer::Open);
        assert!(warning.on_start());
        assert_eq!(
            warning.key(Key::Back),
            WarningAnswer::Cancel,
            "Back never starts"
        );
        let mut warning = FlashWarning::new();
        warning.key(Key::Right);
        assert_eq!(warning.key(Key::Ok), WarningAnswer::Start);
    }

    #[test]
    fn the_hint_lasts_three_seconds() {
        let mut hint = Hint::default();
        assert_eq!(hint.deadline(0), None);
        hint.show("Red".into(), 1000);
        assert!(hint.is_visible(3999));
        assert_eq!(hint.deadline(1000), Some(4000));
        assert!(!hint.is_visible(4000));
        assert_eq!(hint.deadline(4000), None);
    }

    #[test]
    fn remaining_time_is_minutes_and_seconds() {
        assert_eq!(format_remaining(0), "0:00");
        assert_eq!(format_remaining(1), "0:01");
        assert_eq!(format_remaining(600_000), "10:00");
        assert_eq!(format_remaining(3_600_000), "1:00:00");
    }
}
