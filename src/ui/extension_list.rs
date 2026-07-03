use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::{config::Theme, extensions::ExtensionEntry};

pub struct ExtScopeView<'a> {
    pub label: &'a str,
    pub extensions: &'a [ExtensionEntry],
}

pub fn render_extension_list(
    frame: &mut Frame,
    theme: &Theme,
    scopes: &[ExtScopeView],
    selected: usize,
    area: Rect,
) {
    let accent = Theme::parse_color(&theme.accent);
    let fg = Theme::parse_color(&theme.fg);
    let bg = Theme::parse_color(&theme.bg);
    let border_col = Theme::parse_color(&theme.border);
    let inactive = Theme::parse_color(&theme.tab_inactive);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(5)])
        .split(area);

    let mut items: Vec<ListItem> = Vec::new();
    let mut flat_idx = 0usize;
    let mut selected_item_pos: Option<usize> = None;
    let mut selected_entry: Option<&ExtensionEntry> = None;
    let mut any_entries = false;

    for scope in scopes {
        items.push(ListItem::new(Line::from(Span::styled(
            format!("— {} —", scope.label),
            Style::default().fg(inactive).add_modifier(Modifier::BOLD),
        ))));

        for e in scope.extensions {
            any_entries = true;
            let is_sel = flat_idx == selected;
            let enabled_icon = if e.manifest.enabled { "●" } else { "○" };
            let enabled_color = if e.manifest.enabled { accent } else { inactive };
            let sel_icon = if is_sel { "▸" } else { " " };

            let style = if is_sel {
                Style::default().fg(accent).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(fg)
            };

            let danger = if e.manifest.has_dangerous_permissions() { " ⚠" } else { "" };

            items.push(ListItem::new(Line::from(vec![
                Span::styled(format!("{} ", sel_icon), style),
                Span::styled(format!("{} ", enabled_icon), Style::default().fg(enabled_color)),
                Span::styled(format!("{} v{}", e.manifest.name, e.manifest.version), style),
                Span::styled(danger, Style::default().fg(ratatui::style::Color::Yellow)),
                Span::styled(
                    format!("  — {}", e.manifest.author),
                    Style::default().fg(inactive),
                ),
            ])));

            if is_sel {
                selected_item_pos = Some(items.len() - 1);
                selected_entry = Some(e);
            }
            flat_idx += 1;
        }
    }

    if !any_entries {
        items.push(ListItem::new(Line::from(Span::styled(
            "  Sin extensiones instaladas.",
            Style::default().fg(inactive),
        ))));
    }

    let list_block = Block::default()
        .title(" Extensiones [BETA] ")
        .title_style(Style::default().fg(accent).add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_col))
        .style(Style::default().bg(bg));

    let mut list_state = ListState::default();
    list_state.select(selected_item_pos);

    frame.render_stateful_widget(
        List::new(items).block(list_block),
        chunks[0],
        &mut list_state,
    );

    // Detail panel for selected extension
    if let Some(entry) = selected_entry {
        let m = &entry.manifest;
        let status = if m.enabled { "ACTIVA" } else { "INACTIVA" };
        let perms = if m.permissions.is_empty() {
            "ninguno".to_string()
        } else {
            m.permissions.join(", ")
        };
        let detail = Paragraph::new(vec![
            Line::from(vec![
                Span::styled("Descripción: ", Style::default().fg(accent)),
                Span::styled(m.description.clone(), Style::default().fg(fg)),
            ]),
            Line::from(vec![
                Span::styled("Lenguaje: ", Style::default().fg(accent)),
                Span::styled(format!("{:?}", m.language), Style::default().fg(fg)),
                Span::styled("   Estado: ", Style::default().fg(accent)),
                Span::styled(status, Style::default().fg(if m.enabled { accent } else { inactive })),
            ]),
            Line::from(vec![
                Span::styled("Permisos: ", Style::default().fg(accent)),
                Span::styled(perms, Style::default().fg(fg)),
            ]),
            Line::from(Span::styled(
                " Space/Enter=activar/desactivar   Del=desinstalar",
                Style::default().fg(inactive),
            )),
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(border_col))
                .style(Style::default().bg(bg)),
        );
        frame.render_widget(detail, chunks[1]);
    }
}
