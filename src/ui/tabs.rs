use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Widget,
};

use crate::config::Theme;

const BRAND: &str = "Mimic";

/// Screen-space hit region for one tab: clicking within [x_start, close_x_start)
/// switches to the tab, clicking within [close_x_start, close_x_end) closes it.
#[derive(Clone, Copy, Debug)]
pub struct TabRect {
    pub x_start: u16,
    pub close_x_start: u16,
    pub close_x_end: u16,
    pub y: u16,
}

pub struct TabBar<'a> {
    pub tab_names: Vec<&'a str>,
    pub active: usize,
    pub dirty_flags: Vec<bool>,
    pub theme: &'a Theme,
}

impl<'a> Widget for TabBar<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let bg = Theme::parse_color(&self.theme.bg);
        let (spans, _rects, used) = build_tabs(
            &self.tab_names,
            self.active,
            &self.dirty_flags,
            self.theme,
            area,
        );

        let line = Line::from(spans);
        buf.set_line(area.x, area.y, &line, area.width);

        for x in (area.x + used)..area.right() {
            buf[(x, area.y)].set_char(' ').set_style(Style::default().bg(bg));
        }
    }
}

/// Computes tab hit-regions for mouse handling without rendering anything.
pub fn compute_tab_rects(
    tab_names: &[&str],
    active: usize,
    dirty_flags: &[bool],
    theme: &Theme,
    area: Rect,
) -> Vec<TabRect> {
    build_tabs(tab_names, active, dirty_flags, theme, area).1
}

fn build_tabs<'a>(
    tab_names: &[&'a str],
    active: usize,
    dirty_flags: &[bool],
    theme: &Theme,
    area: Rect,
) -> (Vec<Span<'a>>, Vec<TabRect>, u16) {
    let bg = Theme::parse_color(&theme.bg);
    let accent = Theme::parse_color(&theme.tab_active);
    let inactive = Theme::parse_color(&theme.tab_inactive);
    let header = Theme::parse_color(&theme.header);
    let dim = Theme::parse_color(&theme.border);

    let mut spans: Vec<Span> = Vec::new();
    let mut x = area.x;

    let brand = format!(" {} ", BRAND);
    spans.push(Span::styled(
        brand.clone(),
        Style::default().fg(header).bg(bg).add_modifier(Modifier::BOLD),
    ));
    x += brand.len() as u16;

    let sep = "///////////////////";
    spans.push(Span::styled(sep, Style::default().fg(dim).bg(bg)));
    x += sep.len() as u16;
    spans.push(Span::styled(" ", Style::default().bg(bg)));
    x += 1;

    let mut rects = Vec::new();
    for (i, name) in tab_names.iter().enumerate() {
        let dirty = dirty_flags.get(i).copied().unwrap_or(false);
        let indicator = if dirty { "●" } else { " " };
        let name_part = format!(" {} {}", indicator, name);
        let close_part = " ✕ ";

        let is_active = i == active;
        let style = if is_active {
            Style::default().fg(bg).bg(accent).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(inactive).bg(bg)
        };
        let close_style = if is_active {
            Style::default().fg(bg).bg(accent)
        } else {
            Style::default().fg(dim).bg(bg)
        };

        let tab_start = x;
        spans.push(Span::styled(name_part.clone(), style));
        x += name_part.len() as u16;
        let close_x_start = x;
        spans.push(Span::styled(close_part, close_style));
        x += close_part.len() as u16;
        let close_x_end = x;

        rects.push(TabRect {
            x_start: tab_start,
            close_x_start,
            close_x_end,
            y: area.y,
        });

        spans.push(Span::styled("│", Style::default().fg(dim).bg(bg)));
        x += 1;
    }

    (spans, rects, x - area.x)
}
