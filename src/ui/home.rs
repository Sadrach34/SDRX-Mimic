use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::config::Theme;

pub struct HomeState {
    pub selected: usize,
    pub input_mode: HomeInput,
    pub input_buffer: String,
}

#[derive(PartialEq)]
pub enum HomeInput {
    None,
    NewVault,
    OpenPath,
}

impl Default for HomeState {
    fn default() -> Self {
        Self {
            selected: 0,
            input_mode: HomeInput::None,
            input_buffer: String::new(),
        }
    }
}

const LOGO: &str = r#"
  ███████╗██████╗ ██████╗ ██╗  ██╗    ███╗   ███╗██╗███╗   ███╗██╗ ██████╗
  ██╔════╝██╔══██╗██╔══██╗╚██╗██╔╝    ████╗ ████║██║████╗ ████║██║██╔════╝
  ███████╗██║  ██║██████╔╝ ╚███╔╝     ██╔████╔██║██║██╔████╔██║██║██║
  ╚════██║██║  ██║██╔══██╗ ██╔██╗     ██║╚██╔╝██║██║██║╚██╔╝██║██║██║
  ███████║██████╔╝██║  ██║██╔╝ ██╗    ██║ ╚═╝ ██║██║██║ ╚═╝ ██║██║╚██████╗
  ╚══════╝╚═════╝ ╚═╝  ╚═╝╚═╝  ╚═╝   ╚═╝     ╚═╝╚═╝╚═╝     ╚═╝╚═╝ ╚═════╝
"#;

pub fn render_home(
    frame: &mut Frame,
    theme: &Theme,
    state: &HomeState,
    recent_vaults: &[String],
) {
    let area = frame.area();
    let fg = Theme::parse_color(&theme.fg);
    let accent = Theme::parse_color(&theme.accent);
    let header_color = Theme::parse_color(&theme.header);
    let bg = Theme::parse_color(&theme.bg);
    let border_color = Theme::parse_color(&theme.border);
    let inactive = Theme::parse_color(&theme.tab_inactive);

    // fill background
    let bg_block = Block::default().style(Style::default().bg(bg));
    frame.render_widget(bg_block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8),  // logo
            Constraint::Length(2),  // subtitle
            Constraint::Min(10),    // vault list
            Constraint::Length(3),  // input box (conditionally shown)
            Constraint::Length(2),  // help line
        ])
        .margin(2)
        .split(area);

    // Logo
    let logo_lines: Vec<Line> = LOGO
        .lines()
        .map(|l| Line::from(Span::styled(l.to_string(), Style::default().fg(accent).add_modifier(Modifier::BOLD))))
        .collect();
    let logo = Paragraph::new(logo_lines).alignment(Alignment::Center);
    frame.render_widget(logo, chunks[0]);

    // Subtitle
    let subtitle = Paragraph::new(Line::from(vec![
        Span::styled("Tu vault de notas personal. Compatible con ", Style::default().fg(inactive)),
        Span::styled("Obsidian", Style::default().fg(accent)),
        Span::styled(".", Style::default().fg(inactive)),
    ]))
    .alignment(Alignment::Center);
    frame.render_widget(subtitle, chunks[1]);

    // Vault list
    let list_block = Block::default()
        .title(" Mis Vaults ")
        .title_style(Style::default().fg(accent).add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(bg));

    let items: Vec<ListItem> = if recent_vaults.is_empty() {
        vec![ListItem::new(Line::from(Span::styled(
            "  Sin vaults guardadas. Presiona 'n' para crear una.",
            Style::default().fg(inactive),
        )))]
    } else {
        recent_vaults
            .iter()
            .enumerate()
            .map(|(i, v)| {
                let is_sel = i == state.selected;
                let icon = if is_sel { "▸ " } else { "  " };
                let style = if is_sel {
                    Style::default().fg(accent).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(fg)
                };
                ListItem::new(Line::from(Span::styled(format!("{}{}", icon, v), style)))
            })
            .collect()
    };

    let mut list_state = ListState::default();
    if !recent_vaults.is_empty() {
        list_state.select(Some(state.selected));
    }

    frame.render_stateful_widget(
        List::new(items).block(list_block),
        chunks[2],
        &mut list_state,
    );

    // Input box
    let input_area = chunks[3];
    if state.input_mode != HomeInput::None {
        let title = match state.input_mode {
            HomeInput::NewVault => " Nueva vault — ruta: ",
            HomeInput::OpenPath => " Abrir vault — ruta: ",
            HomeInput::None => "",
        };
        let input_block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(accent));
        let inner = input_block.inner(input_area);
        frame.render_widget(input_block, input_area);
        let input_para = Paragraph::new(Span::styled(
            format!("{}_", state.input_buffer),
            Style::default().fg(fg),
        ));
        frame.render_widget(input_para, inner);
    }

    // Help line
    let help = " Enter=abrir   n=nueva vault   o=abrir existente   s=cambiar dir default   d=eliminar   q=salir";
    let help_para = Paragraph::new(Span::styled(help, Style::default().fg(inactive)))
        .alignment(Alignment::Center);
    frame.render_widget(help_para, chunks[4]);
}
