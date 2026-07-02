use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Padding, Paragraph, Wrap},
    Frame,
};

use crate::{
    app::App,
    config::Theme,
    modes::{AppMode, ViewMode},
    ui::{
        file_browser::render_file_browser,
        home::render_home,
        new_vault_dialog::render_new_vault_dialog,
        preview::{render_markdown_with_targets, MarkdownPreview},
        settings::{render_settings, render_warning_dialog},
        sidebar::Sidebar,
        tabs::TabBar,
    },
};

pub fn render(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    if app.app_mode == AppMode::Home {
        let recent = app.config.settings.vault.recent.clone();
        render_home(frame, &app.config.theme, &app.home_state, &recent);
        return;
    }

    if app.app_mode == AppMode::FirstTimeSetup {
        if let Some(browser) = &app.file_browser {
            render_file_browser(frame, &app.config.theme, browser);
        }
        return;
    }

    if app.app_mode == AppMode::FileBrowser {
        let recent = app.config.settings.vault.recent.clone();
        render_home(frame, &app.config.theme, &app.home_state, &recent);
        if let Some(browser) = &app.file_browser {
            render_file_browser(frame, &app.config.theme, browser);
        }
        return;
    }

    if app.app_mode == AppMode::NewVaultDialog {
        let recent = app.config.settings.vault.recent.clone();
        render_home(frame, &app.config.theme, &app.home_state, &recent);
        if let Some(dialog) = &app.new_vault_dialog {
            let default_dir = app.config.settings.vault.default_vaults_dir
                .as_deref()
                .unwrap_or("~");
            render_new_vault_dialog(frame, &app.config.theme, dialog, default_dir);
        }
        return;
    }

    // tabs | content | status | hints
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // tabs
            Constraint::Min(0),    // content
            Constraint::Length(1), // status bar
            Constraint::Length(1), // hints bar
        ])
        .split(area);

    render_tabs(frame, app, main_chunks[0]);
    render_content(frame, app, main_chunks[1]);
    render_status(frame, app, main_chunks[2]);
    render_hints(frame, app, main_chunks[3]);

    if app.app_mode == AppMode::Settings {
        let exts = app.extension_manager.extensions.as_slice();
        render_settings(
            frame,
            &app.config.theme,
            &app.settings_tab,
            exts,
            app.ext_selected,
            &app.theme_editor,
            &app.config.user_themes,
        );
    }

    // Warning dialog rendered on top of everything
    if let Some(ref wd) = app.warning_dialog {
        render_warning_dialog(
            frame,
            &app.config.theme,
            &wd.ext_name,
            &wd.ext_version,
            &wd.ext_author,
            &wd.permissions,
            wd.is_enable,
        );
    }
}

fn render_tabs(frame: &mut Frame, app: &App, area: Rect) {
    let names: Vec<&str> = app.tabs.iter().map(|t| t.note.name()).collect();
    let dirty: Vec<bool> = app.tabs.iter().map(|t| t.note.dirty).collect();
    let widget = TabBar {
        tab_names: names,
        active: app.active_tab,
        dirty_flags: dirty,
        theme: &app.config.theme,
    };
    frame.render_widget(widget, area);
}

fn render_content(frame: &mut Frame, app: &mut App, area: Rect) {
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(24),
            Constraint::Min(0),
        ])
        .split(area);

    let sidebar_block = Block::default()
        .title(" VAULT ")
        .borders(Borders::RIGHT)
        .border_type(BorderType::Plain)
        .style(Style::default().bg(Theme::parse_color(&app.config.theme.sidebar_bg)));

    let sidebar_inner = sidebar_block.inner(content_chunks[0]);
    frame.render_widget(sidebar_block, content_chunks[0]);

    if let Some(vault) = &app.vault {
        let sidebar_widget = Sidebar {
            tree: &vault.tree,
            theme: &app.config.theme,
            scroll_offset: app.sidebar_scroll,
        };
        frame.render_widget(sidebar_widget, sidebar_inner);
    }

    render_editor_area(frame, app, content_chunks[1]);
}

fn render_editor_area(frame: &mut Frame, app: &mut App, area: Rect) {
    if app.tabs.is_empty() {
        let theme = &app.config.theme;
        let fg = Theme::parse_color(&theme.fg);
        let bg = Theme::parse_color(&theme.bg);
        let vault_name = app.vault.as_ref()
            .and_then(|v| v.root.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("vault")
            .to_string();
        let msg = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                format!("  Vault: {}", vault_name),
                Style::default().fg(fg),
            )),
        ])
        .style(Style::default().bg(bg));
        frame.render_widget(msg, area);
        return;
    }

    match &app.view_mode {
        ViewMode::Editor => render_textarea(frame, app, area),
        ViewMode::Preview => render_preview(frame, app, area),
        ViewMode::Split => {
            let halves = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(area);
            render_textarea(frame, app, halves[0]);
            render_preview(frame, app, halves[1]);
        }
    }

    if matches!(app.view_mode, ViewMode::Editor) {
        app.preview_copy_hits.clear();
    }
}

fn render_textarea(frame: &mut Frame, app: &mut App, area: Rect) {
    let theme = &app.config.theme;
    let accent = Theme::parse_color(&theme.accent);
    let bg = Theme::parse_color(&theme.bg);
    let fg = Theme::parse_color(&theme.fg);
    let border_color = Theme::parse_color(&theme.border);

    if let Some(tab) = app.tabs.get_mut(app.active_tab) {
        let header_color = Theme::parse_color(&theme.header);
        let block = Block::default()
            .title(format!(" {} ", tab.note.name()))
            .title_style(Style::default().fg(header_color).add_modifier(Modifier::BOLD))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_color))
            .padding(Padding::new(2, 2, 0, 0))
            .style(Style::default().bg(bg));

        if app.app_mode == AppMode::Insert {
            tab.editor.set_block(block);
            tab.editor.set_style(Style::default().fg(fg).bg(bg));
            tab.editor.set_cursor_style(Style::default().bg(accent));
            frame.render_widget(&tab.editor, area);
        } else {
            let inner = block.inner(area);
            frame.render_widget(block, area);
            let content = tab.editor.lines().join("\n");
            let para = Paragraph::new(content)
                .style(Style::default().fg(fg).bg(bg))
                .wrap(Wrap { trim: false });
            frame.render_widget(para, inner);
        }
    }
}

fn render_preview(frame: &mut Frame, app: &mut App, area: Rect) {
    let theme = &app.config.theme;
    let border_color = Theme::parse_color(&theme.border);
    let bg = Theme::parse_color(&theme.bg);

    let header_color = Theme::parse_color(&theme.header);
    let block = Block::default()
        .title(" Preview ")
        .title_style(Style::default().fg(header_color).add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .padding(Padding::new(2, 2, 0, 0))
        .style(Style::default().bg(bg));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if let Some(tab) = app.tabs.get(app.active_tab) {
        let content: String = tab.editor.lines().join("\n");
        let rendered = render_markdown_with_targets(&content, inner.width as usize, theme);
        app.preview_copy_hits.clear();
        for target in rendered.copy_targets {
            let row = target.row;
            let scroll = tab.scroll_preview as usize;
            if row < scroll {
                continue;
            }
            let visible_row = row - scroll;
            if visible_row >= inner.height as usize {
                continue;
            }
            let y = inner.y + visible_row as u16;
            let x_start = inner.x + target.x_start;
            let x_end = inner.x + target.x_end;
            app.preview_copy_hits.push(crate::app::PreviewCopyHit {
                x_start,
                x_end,
                y,
                text: target.text,
            });
        }

        let preview = MarkdownPreview {
            content: &content,
            scroll: tab.scroll_preview,
            theme,
        };
        frame.render_widget(preview, inner);
    } else {
        app.preview_copy_hits.clear();
    }
}

fn render_status(frame: &mut Frame, app: &App, area: Rect) {
    let theme = &app.config.theme;
    let status_bg = Theme::parse_color(&theme.status_bg);
    let accent = Theme::parse_color(&theme.accent);
    let fg = Theme::parse_color(&theme.fg);

    let mode_label = app.app_mode.label();
    let view_label = app.view_mode.label();

    let note_name = app
        .tabs
        .get(app.active_tab)
        .map(|t| t.note.name().to_string())
        .unwrap_or_default();

    let vault_name = app
        .vault
        .as_ref()
        .and_then(|v| v.root.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("—")
        .to_string();

    let dirty_marker = app
        .tabs
        .get(app.active_tab)
        .map(|t| if t.note.dirty { " ●" } else { "" })
        .unwrap_or("");

    let status_text = if app.app_mode == AppMode::Command {
        format!(":{}", app.command_buffer)
    } else {
        format!(
            " {} | {} | {} | vault: {}{}",
            mode_label, view_label, note_name, vault_name, dirty_marker
        )
    };

    let status_msg = app
        .status_msg
        .as_ref()
        .map(|m| format!("  {}", m))
        .unwrap_or_default();

    let left_width = area.width.saturating_sub(status_msg.len() as u16);
    let status = Paragraph::new(Line::from(vec![
        Span::styled(
            format!("{:<width$}", status_text, width = left_width as usize),
            Style::default().fg(accent).bg(status_bg),
        ),
        Span::styled(status_msg, Style::default().fg(fg).bg(status_bg)),
    ]));
    frame.render_widget(status, area);
}

fn render_hints(frame: &mut Frame, app: &App, area: Rect) {
    let theme = &app.config.theme;
    let bg = Theme::parse_color(&theme.border);
    let key_style = Style::default()
        .fg(Theme::parse_color(&theme.accent))
        .add_modifier(Modifier::BOLD);
    let sep_style = Style::default().fg(Theme::parse_color(&theme.tab_inactive)).bg(bg);

    let hints: &[(&str, &str)] = match app.app_mode {
        AppMode::Normal => &[
            ("e/i", "editar"),
            (":", "comando"),
            ("Tab", "sig.tab"),
            ("^W", "cerrar tab"),
            ("r", "renombrar"),
            ("j/k", "sidebar"),
            ("Enter", "abrir"),
            ("g", "link"),
            ("^V", "vista"),
            ("^T", "config"),
            ("^H", "home"),
        ],
        AppMode::Insert => &[
            ("Esc", "normal"),
            ("^S", "guardar"),
        ],
        AppMode::Command => &[
            (":w", "guardar"),
            (":q", "cerrar tab"),
            (":qa", "salir"),
            (":wq", "guardar+cerrar"),
            (":new <nom>", "nueva nota"),
            (":mkdir <nom>", "nueva carpeta"),
            (":vault <ruta>", "cambiar vault"),
            (":home", "home"),
            ("Esc", "cancelar"),
        ],
        AppMode::Settings => &[
            ("Tab", "cambiar tab"),
            ("j/k", "navegar"),
            ("Space", "activar/desactivar"),
            ("Esc", "cerrar"),
        ],
        _ => &[],
    };

    let mut spans: Vec<Span> = Vec::new();
    spans.push(Span::styled(" ", sep_style));
    for (i, (key, desc)) in hints.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("  ", sep_style));
        }
        spans.push(Span::styled(key.to_string(), key_style.bg(bg)));
        spans.push(Span::styled(format!(" {}", desc), sep_style));
    }

    // fill rest of line
    let content_len: usize = hints.iter().map(|(k, d)| k.len() + d.len() + 3).sum::<usize>() + 1;
    let padding = " ".repeat((area.width as usize).saturating_sub(content_len));
    spans.push(Span::styled(padding, sep_style));

    let line = Paragraph::new(Line::from(spans));
    frame.render_widget(line, area);
}
