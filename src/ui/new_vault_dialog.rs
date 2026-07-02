use std::path::PathBuf;

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};

use crate::config::Theme;

pub struct NewVaultDialog {
    pub name: String,
    pub override_path: Option<PathBuf>,
}

impl Default for NewVaultDialog {
    fn default() -> Self {
        Self {
            name: String::new(),
            override_path: None,
        }
    }
}

impl NewVaultDialog {
    pub fn resolved_path(&self, default_dir: &str) -> PathBuf {
        let base = self.override_path.clone()
            .unwrap_or_else(|| PathBuf::from(default_dir));
        base.join(&self.name)
    }
}

pub fn render_new_vault_dialog(
    frame: &mut Frame,
    theme: &Theme,
    dialog: &NewVaultDialog,
    default_dir: &str,
) {
    let area = frame.area();
    let popup = centered_rect(55, 40, area);
    frame.render_widget(Clear, popup);

    let fg = Theme::parse_color(&theme.fg);
    let accent = Theme::parse_color(&theme.accent);
    let bg = Theme::parse_color(&theme.bg);
    let border_color = Theme::parse_color(&theme.border);
    let inactive = Theme::parse_color(&theme.tab_inactive);
    let link_color = Theme::parse_color(&theme.link);

    let outer = Block::default()
        .title(" Nueva Vault ")
        .title_style(Style::default().fg(accent).add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(accent))
        .style(Style::default().bg(bg));

    let inner = outer.inner(popup);
    frame.render_widget(outer, popup);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // spacing
            Constraint::Length(3), // name input
            Constraint::Length(1), // spacing
            Constraint::Length(3), // path display
            Constraint::Length(1), // spacing
            Constraint::Min(0),    // options
        ])
        .margin(1)
        .split(inner);

    // Name input
    let name_block = Block::default()
        .title(" Nombre ")
        .title_style(Style::default().fg(inactive))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color));
    let name_inner = name_block.inner(chunks[1]);
    frame.render_widget(name_block, chunks[1]);
    let cursor = if dialog.name.is_empty() { "_" } else { "▌" };
    let name_para = Paragraph::new(Span::styled(
        format!("{}{}", dialog.name, cursor),
        Style::default().fg(fg).add_modifier(Modifier::BOLD),
    ));
    frame.render_widget(name_para, name_inner);

    // Path preview
    let base = dialog.override_path.clone()
        .unwrap_or_else(|| PathBuf::from(default_dir));
    let full_path = base.join(&dialog.name);
    let path_str = full_path.to_string_lossy().to_string();

    let is_override = dialog.override_path.is_some();
    let path_label = if is_override { " Ruta (personalizada) " } else { " Se creará en " };
    let path_style = if is_override {
        Style::default().fg(link_color)
    } else {
        Style::default().fg(inactive)
    };

    let path_block = Block::default()
        .title(path_label)
        .title_style(path_style)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color));
    let path_inner = path_block.inner(chunks[3]);
    frame.render_widget(path_block, chunks[3]);
    let path_para = Paragraph::new(Span::styled(path_str, Style::default().fg(fg)));
    frame.render_widget(path_para, path_inner);

    // Options
    let opts = vec![
        Line::from(vec![
            Span::styled("  Tab", Style::default().fg(accent).add_modifier(Modifier::BOLD)),
            Span::styled("  — cambiar ruta solo esta vault", Style::default().fg(inactive)),
        ]),
        Line::from(vec![
            Span::styled("  Enter", Style::default().fg(accent).add_modifier(Modifier::BOLD)),
            Span::styled("  — crear vault", Style::default().fg(inactive)),
        ]),
        Line::from(vec![
            Span::styled("  Esc", Style::default().fg(accent).add_modifier(Modifier::BOLD)),
            Span::styled("  — cancelar", Style::default().fg(inactive)),
        ]),
    ];
    let opts_para = Paragraph::new(opts).alignment(Alignment::Left);
    frame.render_widget(opts_para, chunks[5]);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: ratatui::layout::Rect) -> ratatui::layout::Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
