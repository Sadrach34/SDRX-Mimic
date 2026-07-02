use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Widget,
};

use crate::config::Theme;
use crate::vault::tree::FileTree;

pub struct Sidebar<'a> {
    pub tree: &'a FileTree,
    pub theme: &'a Theme,
    pub scroll_offset: usize,
}

impl<'a> Widget for Sidebar<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let fg = Theme::parse_color(&self.theme.fg);
        let accent = Theme::parse_color(&self.theme.accent);
        let sidebar_bg = Theme::parse_color(&self.theme.sidebar_bg);
        let link_color = Theme::parse_color(&self.theme.link);

        let vis = self.tree.visible_indices();
        let visible_rows = area.height as usize;
        let start = self.scroll_offset;

        for (row_idx, &entry_idx) in vis.iter().skip(start).take(visible_rows).enumerate() {
            let y = area.y + row_idx as u16;
            let entry = &self.tree.entries[entry_idx];
            let is_selected = entry_idx == self.tree.selected;

            let indent = "  ".repeat(entry.depth);
            let icon = if entry.is_dir {
                if self.tree.collapsed.contains(&entry.path) { "▸ " } else { "▾ " }
            } else { "  " };
            let label = format!("{}{}{}", indent, icon, entry.name);

            let style = if is_selected {
                Style::default()
                    .fg(accent)
                    .bg(sidebar_bg)
                    .add_modifier(Modifier::BOLD)
            } else if entry.is_dir {
                Style::default().fg(fg).bg(sidebar_bg)
            } else {
                Style::default().fg(link_color).bg(sidebar_bg)
            };

            let line = Line::from(vec![Span::styled(
                format!("{:<width$}", label, width = area.width as usize),
                style,
            )]);
            buf.set_line(area.x, y, &line, area.width);
        }

        let rendered_count = vis.len().saturating_sub(start).min(visible_rows);
        // fill empty rows
        for row_idx in rendered_count..visible_rows {
            let y = area.y + row_idx as u16;
            for x in area.x..area.right() {
                buf[(x, y)].set_char(' ').set_style(Style::default().bg(sidebar_bg));
            }
        }
    }
}
