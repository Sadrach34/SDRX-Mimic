use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};

use crate::{
    config::Theme,
    modes::SettingsTab,
    ui::{
        extension_list::{render_extension_list, ExtScopeView},
        theme_editor::{render_theme_editor, ThemeEditorState},
    },
};

pub fn render_settings(
    frame: &mut Frame,
    theme: &Theme,
    tab: &SettingsTab,
    scopes: &[ExtScopeView],
    ext_selected: usize,
    theme_state: &ThemeEditorState,
    user_themes: &[(String, Theme)],
) {
    let area = centered_rect(80, 90, frame.area());
    frame.render_widget(Clear, area);

    let accent = Theme::parse_color(&theme.accent);
    let fg = Theme::parse_color(&theme.fg);
    let bg = Theme::parse_color(&theme.bg);
    let border_col = Theme::parse_color(&theme.border);
    let inactive = Theme::parse_color(&theme.tab_inactive);
    let tab_active_bg = Theme::parse_color(&theme.tab_active);
    let tab_inactive_bg = Theme::parse_color(&theme.tab_inactive);

    let outer = Block::default()
        .title(" SDRX Mimic — Configuración ")
        .title_style(Style::default().fg(accent).add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_col))
        .style(Style::default().bg(bg));

    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(inner);

    // Tab bar
    let ext_style = if *tab == SettingsTab::Extensions {
        Style::default().fg(accent).bg(tab_active_bg).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(fg).bg(tab_inactive_bg)
    };
    let theme_style = if *tab == SettingsTab::Themes {
        Style::default().fg(accent).bg(tab_active_bg).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(fg).bg(tab_inactive_bg)
    };

    let tab_bar = Paragraph::new(Line::from(vec![
        Span::styled(" Extensiones [BETA] ", ext_style),
        Span::styled("  ", Style::default().bg(bg)),
        Span::styled(" Temas ", theme_style),
        Span::styled("  Tab=cambiar   Esc=cerrar", Style::default().fg(inactive).bg(bg)),
    ]));
    frame.render_widget(tab_bar, chunks[0]);

    match tab {
        SettingsTab::Extensions => {
            render_extension_list(frame, theme, scopes, ext_selected, chunks[1]);
        }
        SettingsTab::Themes => {
            // Render theme editor content inline (reuse existing render_theme_editor)
            // theme_editor renders as a centered overlay so we call it directly
            render_theme_editor(frame, theme, theme_state, user_themes);
        }
    }
}

pub fn render_warning_dialog(
    frame: &mut Frame,
    theme: &Theme,
    ext_name: &str,
    ext_version: &str,
    ext_author: &str,
    permissions: &[String],
    is_enable: bool,
) {
    let accent = Theme::parse_color(&theme.accent);
    let fg = Theme::parse_color(&theme.fg);
    let bg = Theme::parse_color(&theme.bg);
    let border_col = Theme::parse_color(&theme.border);
    let yellow = ratatui::style::Color::Yellow;
    let inactive = Theme::parse_color(&theme.tab_inactive);

    let area = centered_rect(60, 50, frame.area());
    frame.render_widget(Clear, area);

    let action_verb = if is_enable { "ACTIVAR" } else { "INSTALAR" };

    let perms_text = if permissions.is_empty() {
        "ninguno".to_string()
    } else {
        permissions.join(", ")
    };

    let has_dangerous = permissions.iter().any(|p| p == "fs.write" || p == "process.run");

    let mut lines = vec![
        Line::from(Span::styled(
            format!("  ⚠  EXTENSIÓN DE TERCEROS — {}  ", action_verb),
            Style::default().fg(yellow).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Extensión: ", Style::default().fg(accent)),
            Span::styled(
                format!("\"{}\" v{} por {}", ext_name, ext_version, ext_author),
                Style::default().fg(fg),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Permisos:  ", Style::default().fg(accent)),
            Span::styled(perms_text, Style::default().fg(fg)),
        ]),
        Line::from(""),
    ];

    if has_dangerous {
        lines.push(Line::from(Span::styled(
            "  ⚠ Esta extensión tiene permisos peligrosos (fs.write / process.run).",
            Style::default().fg(yellow),
        )));
        lines.push(Line::from(""));
    }

    lines.extend([
        Line::from(Span::styled(
            "  Las extensiones son código externo no revisado por SDRX Mimic.",
            Style::default().fg(fg),
        )),
        Line::from(Span::styled(
            "  Pueden contener código malicioso. Instala solo de fuentes confiables.",
            Style::default().fg(fg),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  El creador de SDRX Mimic no se responsabiliza de daños",
            Style::default().fg(inactive),
        )),
        Line::from(Span::styled(
            "  causados por extensiones de terceros.",
            Style::default().fg(inactive),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(" Y  Confirmar ", Style::default().fg(bg).bg(accent).add_modifier(Modifier::BOLD)),
            Span::styled("    ", Style::default()),
            Span::styled(" N / Esc  Cancelar ", Style::default().fg(fg).bg(border_col)),
        ]),
    ]);

    let dialog = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(yellow))
                .style(Style::default().bg(bg)),
        );
    frame.render_widget(dialog, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
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
