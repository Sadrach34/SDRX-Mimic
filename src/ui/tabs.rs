use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Widget,
};

use crate::config::Theme;

pub struct TabBar<'a> {
    pub tab_names: Vec<&'a str>,
    pub active: usize,
    pub dirty_flags: Vec<bool>,
    pub theme: &'a Theme,
}

impl<'a> Widget for TabBar<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let accent = Theme::parse_color(&self.theme.tab_active);
        let inactive = Theme::parse_color(&self.theme.tab_inactive);
        let bg = Theme::parse_color(&self.theme.bg);

        let mut spans: Vec<Span> = Vec::new();
        for (i, name) in self.tab_names.iter().enumerate() {
            let dirty = self.dirty_flags.get(i).copied().unwrap_or(false);
            let indicator = if dirty { "●" } else { " " };
            let label = format!(" {} {}{} ", indicator, name, " ");

            let style = if i == self.active {
                Style::default()
                    .fg(bg)
                    .bg(accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(inactive).bg(bg)
            };
            spans.push(Span::styled(label, style));
            spans.push(Span::styled(" ", Style::default().bg(bg)));
        }

        let line = Line::from(spans);
        buf.set_line(area.x, area.y, &line, area.width);

        // fill rest of row
        let used: u16 = self
            .tab_names
            .iter()
            .enumerate()
            .map(|(i, n)| {
                let dirty = self.dirty_flags.get(i).copied().unwrap_or(false);
                let d = if dirty { 1 } else { 1 };
                (n.len() + 4 + d) as u16
            })
            .sum();
        for x in (area.x + used)..area.right() {
            buf[(x, area.y)].set_char(' ').set_style(Style::default().bg(bg));
        }
    }
}
