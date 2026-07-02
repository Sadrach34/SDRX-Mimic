use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};

use crate::config::Theme;

#[derive(Debug, Clone, PartialEq)]
pub enum ThemeEditorFocus {
    Presets,
    Fields,
}

pub struct ThemeEditorState {
    pub focus: ThemeEditorFocus,
    pub selected_preset: usize,
    pub selected_field: usize,
    pub editing: bool,
    pub input_buffer: String,
    pub exporting: bool,
    pub export_input: String,
    pub importing: bool,
    pub import_input: String,
}

impl Default for ThemeEditorState {
    fn default() -> Self {
        Self {
            focus: ThemeEditorFocus::Presets,
            selected_preset: 0,
            selected_field: 0,
            editing: false,
            input_buffer: String::new(),
            exporting: false,
            export_input: String::new(),
            importing: false,
            import_input: String::new(),
        }
    }
}

pub fn render_theme_editor(
    frame: &mut Frame,
    theme: &Theme,
    state: &ThemeEditorState,
    user_themes: &[(String, Theme)],
) {
    let area = centered_rect(58, 88, frame.area());
    frame.render_widget(Clear, area);

    let accent = Theme::parse_color(&theme.accent);
    let fg = Theme::parse_color(&theme.fg);
    let bg = Theme::parse_color(&theme.bg);
    let border_col = Theme::parse_color(&theme.border);
    let inactive = Theme::parse_color(&theme.tab_inactive);

    let outer = Block::default()
        .title(" Tema — Ctrl+T cerrar ")
        .title_style(Style::default().fg(accent).add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(accent))
        .style(Style::default().bg(bg));

    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    // Split: presets top | divider | fields | help
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // presets row
            Constraint::Length(1),  // divider label
            Constraint::Min(0),     // color fields
            Constraint::Length(1),  // help
        ])
        .split(inner);

    // ── Presets row ──────────────────────────────────────────────────────────
    let fixed_names = Theme::preset_names();
    let all_preset_count = fixed_names.len() + user_themes.len();
    let mut preset_spans: Vec<Span> = Vec::new();
    preset_spans.push(Span::styled(" ", Style::default().bg(bg)));
    let is_focus = state.focus == ThemeEditorFocus::Presets;
    for (i, name) in fixed_names.iter().enumerate() {
        let is_sel = i == state.selected_preset;
        let style = if is_sel && is_focus {
            Style::default().fg(bg).bg(accent).add_modifier(Modifier::BOLD)
        } else if is_sel {
            Style::default().fg(accent).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(inactive).bg(bg)
        };
        preset_spans.push(Span::styled(format!(" {} ", name), style));
        preset_spans.push(Span::styled("  ", Style::default().bg(bg)));
    }
    for (ui, (name, _)) in user_themes.iter().enumerate() {
        let i = fixed_names.len() + ui;
        let is_sel = i == state.selected_preset;
        let style = if is_sel && is_focus {
            Style::default().fg(bg).bg(accent).add_modifier(Modifier::BOLD)
        } else if is_sel {
            Style::default().fg(accent).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(inactive).bg(bg)
        };
        preset_spans.push(Span::styled(format!(" {} ", name), style));
        preset_spans.push(Span::styled("  ", Style::default().bg(bg)));
    }
    let _ = all_preset_count;

    // nav hint for presets
    let preset_hint = if state.focus == ThemeEditorFocus::Presets {
        " ←/→ elegir  Enter=aplicar  ↓=colores"
    } else {
        " Tab=presets"
    };
    let preset_block = Block::default()
        .title(preset_hint)
        .title_style(Style::default().fg(inactive))
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(border_col));
    let preset_inner = preset_block.inner(chunks[0]);
    frame.render_widget(preset_block, chunks[0]);
    frame.render_widget(Paragraph::new(Line::from(preset_spans)), preset_inner);

    // ── Section label ────────────────────────────────────────────────────────
    let label_style = if state.focus == ThemeEditorFocus::Fields {
        Style::default().fg(accent).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(inactive)
    };
    let hint_label = if state.selected_preset == 3 {
        " Colores — Enter=editar"
    } else {
        " Colores — Enter auto-cambia a Custom"
    };
    frame.render_widget(
        Paragraph::new(Span::styled(hint_label, label_style)),
        chunks[1],
    );

    // ── Color fields ─────────────────────────────────────────────────────────
    let field_names = [
        "bg", "fg", "accent", "header", "link",
        "border", "status_bg", "tab_active", "tab_inactive", "sidebar_bg", "cursor",
    ];
    let field_values = [
        &theme.bg, &theme.fg, &theme.accent, &theme.header, &theme.link,
        &theme.border, &theme.status_bg, &theme.tab_active, &theme.tab_inactive,
        &theme.sidebar_bg, &theme.cursor,
    ];

    let fields_active = state.focus == ThemeEditorFocus::Fields;

    let rows: Vec<Line> = field_names
        .iter()
        .zip(field_values.iter())
        .enumerate()
        .map(|(i, (name, val))| {
            let is_sel = i == state.selected_field && fields_active;
            let display_val = if is_sel && state.editing {
                state.input_buffer.clone()
            } else {
                (*val).clone()
            };
            let parsed = Theme::parse_color(val);

            let (name_style, val_style) = if is_sel {
                (
                    Style::default().fg(accent).add_modifier(Modifier::BOLD),
                    Style::default().fg(fg),
                )
            } else {
                (Style::default().fg(fg), Style::default().fg(inactive))
            };

            Line::from(vec![
                Span::styled(format!("  {:<12}", name), name_style),
                Span::styled(format!("{:>8}  ", display_val), val_style),
                Span::styled("██", Style::default().fg(parsed)),
            ])
        })
        .collect();

    let fields_para = Paragraph::new(rows).style(Style::default().bg(bg));
    frame.render_widget(fields_para, chunks[2]);

    // ── Help bar / export / import input ─────────────────────────────────────
    if state.exporting {
        let export_line = Line::from(vec![
            Span::styled(" Exportar como: ", Style::default().fg(accent).add_modifier(Modifier::BOLD)),
            Span::styled(state.export_input.clone(), Style::default().fg(fg)),
            Span::styled("█", Style::default().fg(accent)),
            Span::styled("  Esc=cancelar", Style::default().fg(inactive)),
        ]);
        frame.render_widget(Paragraph::new(export_line), chunks[3]);
    } else if state.importing {
        let import_line = Line::from(vec![
            Span::styled(" Importar tema: ", Style::default().fg(accent).add_modifier(Modifier::BOLD)),
            Span::styled(state.import_input.clone(), Style::default().fg(fg)),
            Span::styled("█", Style::default().fg(accent)),
            Span::styled("  Esc=cancelar", Style::default().fg(inactive)),
        ]);
        frame.render_widget(Paragraph::new(import_line), chunks[3]);
    } else {
        let help = if state.editing {
            " Enter=confirmar  Esc=cancelar"
        } else if state.focus == ThemeEditorFocus::Presets {
            " ←/→ preset  Enter=aplicar  e=exportar  i=importar  ↓=colores  Esc=cerrar"
        } else {
            " ↑/↓ campo  Enter=editar  e=exportar  i=importar  Tab=presets  Esc=cerrar"
        };
        frame.render_widget(
            Paragraph::new(Span::styled(help, Style::default().fg(inactive))),
            chunks[3],
        );
    }
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
