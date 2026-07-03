use std::path::PathBuf;

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::config::Theme;

#[derive(Clone, PartialEq)]
pub enum FileBrowserPurpose {
    /// Primera vez: elegir directorio default
    FirstTimeSetup,
    /// Cambiar directorio default desde settings
    SetDefaultDir,
    /// Abrir vault existente
    OpenVault,
    /// Override de ruta para una vault nueva específica
    NewVaultOverridePath,
}

pub struct FileBrowserState {
    pub current_path: PathBuf,
    pub entries: Vec<PathBuf>,
    pub selected: usize,
    pub purpose: FileBrowserPurpose,
    /// Buffer del nombre cuando se está creando una carpeta nueva; None = no está creando
    pub new_folder_input: Option<String>,
}

impl FileBrowserState {
    pub fn new(purpose: FileBrowserPurpose) -> Self {
        let start = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
        let mut state = Self {
            current_path: start.clone(),
            entries: Vec::new(),
            selected: 0,
            purpose,
            new_folder_input: None,
        };
        state.load_entries();
        state
    }

    pub fn start_create_folder(&mut self) {
        self.new_folder_input = Some(String::new());
    }

    pub fn cancel_create_folder(&mut self) {
        self.new_folder_input = None;
    }

    pub fn push_char(&mut self, c: char) {
        if let Some(buf) = self.new_folder_input.as_mut() {
            buf.push(c);
        }
    }

    pub fn pop_char(&mut self) {
        if let Some(buf) = self.new_folder_input.as_mut() {
            buf.pop();
        }
    }

    /// Crea la carpeta con el nombre acumulado en current_path, recarga la lista
    /// y selecciona la carpeta recién creada. Devuelve Err si falla la creación.
    pub fn confirm_create_folder(&mut self) -> std::io::Result<()> {
        let name = self.new_folder_input.take().unwrap_or_default();
        let name = name.trim();
        if name.is_empty() {
            return Ok(());
        }
        let new_dir = self.current_path.join(name);
        std::fs::create_dir_all(&new_dir)?;
        self.load_entries();
        if let Some(idx) = self.entries.iter().position(|p| p == &new_dir) {
            self.selected = idx;
        }
        Ok(())
    }

    pub fn load_entries(&mut self) {
        let mut entries: Vec<PathBuf> = std::fs::read_dir(&self.current_path)
            .into_iter()
            .flatten()
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .filter(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| !n.starts_with('.'))
                    .unwrap_or(false)
            })
            .collect();
        entries.sort();
        self.entries = entries;
        self.selected = 0;
    }

    pub fn enter_selected(&mut self) {
        if let Some(path) = self.entries.get(self.selected).cloned() {
            self.current_path = path;
            self.load_entries();
        }
    }

    pub fn go_up(&mut self) {
        if let Some(parent) = self.current_path.parent().map(|p| p.to_path_buf()) {
            self.current_path = parent;
            self.load_entries();
        }
    }

    pub fn move_down(&mut self) {
        if !self.entries.is_empty() && self.selected + 1 < self.entries.len() {
            self.selected += 1;
        }
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn selected_path(&self) -> PathBuf {
        self.current_path.clone()
    }

    pub fn title(&self) -> &str {
        match self.purpose {
            FileBrowserPurpose::FirstTimeSetup => " Primera vez — elige dónde guardar tus vaults ",
            FileBrowserPurpose::SetDefaultDir => " Cambiar directorio default ",
            FileBrowserPurpose::OpenVault => " Abrir Vault existente ",
            FileBrowserPurpose::NewVaultOverridePath => " Ruta para esta vault (solo esta vez) ",
        }
    }
}

pub fn render_file_browser(frame: &mut Frame, theme: &Theme, state: &FileBrowserState) {
    let area = frame.area();

    let popup = centered_rect(60, 75, area);
    frame.render_widget(Clear, popup);

    let fg = Theme::parse_color(&theme.fg);
    let accent = Theme::parse_color(&theme.accent);
    let bg = Theme::parse_color(&theme.bg);
    let border_color = Theme::parse_color(&theme.border);
    let inactive = Theme::parse_color(&theme.tab_inactive);
    let link_color = Theme::parse_color(&theme.link);

    let outer_block = Block::default()
        .title(state.title())
        .title_style(Style::default().fg(accent).add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(accent))
        .style(Style::default().bg(bg));

    let inner = outer_block.inner(popup);
    frame.render_widget(outer_block, popup);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // current path
            Constraint::Min(0),    // entries list
            Constraint::Length(2), // help
        ])
        .split(inner);

    // Current path header
    let path_str = state.current_path.to_string_lossy().to_string();
    let path_para = Paragraph::new(Line::from(vec![
        Span::styled("  ", Style::default()),
        Span::styled("", Style::default().fg(accent)),
        Span::styled(format!(" {} ", path_str), Style::default().fg(fg).add_modifier(Modifier::BOLD)),
    ]))
    .block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(border_color)),
    );
    frame.render_widget(path_para, chunks[0]);

    // Directory list
    let items: Vec<ListItem> = if state.entries.is_empty() {
        vec![ListItem::new(Line::from(Span::styled(
            "  (directorio vacío)",
            Style::default().fg(inactive),
        )))]
    } else {
        state
            .entries
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("?");
                let is_sel = i == state.selected;
                let (icon, style) = if is_sel {
                    ("▸ ", Style::default().fg(accent).add_modifier(Modifier::BOLD))
                } else {
                    ("  ", Style::default().fg(link_color))
                };
                ListItem::new(Line::from(Span::styled(
                    format!("{}{}/", icon, name),
                    style,
                )))
            })
            .collect()
    };

    let mut list_state = ListState::default();
    if !state.entries.is_empty() {
        list_state.select(Some(state.selected));
    }

    frame.render_stateful_widget(List::new(items), chunks[1], &mut list_state);

    // Help line
    let action_label = match state.purpose {
        FileBrowserPurpose::FirstTimeSetup | FileBrowserPurpose::SetDefaultDir => "Espacio=usar como default",
        FileBrowserPurpose::OpenVault => "Espacio=abrir vault aquí",
        FileBrowserPurpose::NewVaultOverridePath => "Espacio=usar esta ruta",
    };
    let help_text = if state.new_folder_input.is_some() {
        "  Enter=crear  Esc=cancelar".to_string()
    } else {
        format!(
            "  j/k=mover  Enter=entrar  ←/h=subir  n=nueva carpeta  {}  Esc=cancelar",
            action_label
        )
    };
    let help = Paragraph::new(Line::from(vec![Span::styled(
        help_text,
        Style::default().fg(inactive),
    )]))
    .alignment(Alignment::Left)
    .block(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(border_color)),
    );
    frame.render_widget(help, chunks[2]);

    // Overlay de input para nombre de carpeta nueva
    if let Some(input) = &state.new_folder_input {
        let input_popup = centered_rect(50, 15, area);
        frame.render_widget(Clear, input_popup);
        let input_block = Block::default()
            .title(" Nueva carpeta ")
            .title_style(Style::default().fg(accent).add_modifier(Modifier::BOLD))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(accent))
            .style(Style::default().bg(bg));
        let input_para = Paragraph::new(Line::from(vec![
            Span::styled(format!(" {}", input), Style::default().fg(fg)),
            Span::styled("▏", Style::default().fg(accent)),
        ]))
        .block(input_block);
        frame.render_widget(input_para, input_popup);
    }
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
