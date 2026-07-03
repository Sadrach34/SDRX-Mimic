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

    // Pinta todo el frame con el bg del tema antes de cualquier widget: sin esto,
    // celdas que ningún widget toca (huecos entre paneles, bordes) quedan con el
    // bg "default" del terminal, que en kitty/alacritty con transparencia se ve
    // a través del fondo del escritorio en vez del color del tema.
    frame.render_widget(
        Block::default().style(Style::default().bg(Theme::parse_color(&app.config.theme.bg))),
        area,
    );

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
        let mut scopes = vec![crate::ui::extension_list::ExtScopeView {
            label: "Global",
            extensions: app.extension_manager.extensions.as_slice(),
        }];
        if let Some(vm) = &app.vault_extension_manager {
            scopes.push(crate::ui::extension_list::ExtScopeView {
                label: "Vault",
                extensions: vm.extensions.as_slice(),
            });
        }
        render_settings(
            frame,
            &app.config.theme,
            &app.settings_tab,
            &scopes,
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

fn render_tabs(frame: &mut Frame, app: &mut App, area: Rect) {
    let names: Vec<&str> = app.tabs.iter().map(|t| t.note.name()).collect();
    let dirty: Vec<bool> = app.tabs.iter().map(|t| t.note.dirty).collect();

    app.tab_hits = crate::ui::tabs::compute_tab_rects(
        &names,
        app.active_tab,
        &dirty,
        &app.config.theme,
        area,
    );

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

    let sidebar_border = if app.focus == crate::app::Focus::Sidebar {
        Theme::parse_color(&app.config.theme.accent)
    } else {
        Theme::parse_color(&app.config.theme.border)
    };
    let sidebar_block = Block::default()
        .title(" VAULT ")
        .borders(Borders::RIGHT)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(sidebar_border))
        .style(Style::default().bg(Theme::parse_color(&app.config.theme.sidebar_bg)));

    let sidebar_inner = sidebar_block.inner(content_chunks[0]);
    frame.render_widget(sidebar_block, content_chunks[0]);
    app.sidebar_area = Some(sidebar_inner);

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
    app.editor_area = Some(area);
    let theme = &app.config.theme;
    let accent = Theme::parse_color(&theme.accent);
    let bg = Theme::parse_color(&theme.bg);
    let fg = Theme::parse_color(&theme.fg);
    let border_color = if app.focus == crate::app::Focus::Editor {
        accent
    } else {
        Theme::parse_color(&theme.border)
    };

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
            let inner = block.inner(area);
            frame.render_widget(block, area);

            let logical_lines: Vec<String> = tab.editor.lines().to_vec();
            let cursor = tab.editor.cursor();
            let selection = tab.editor.selection_range();

            let digits = logical_lines.len().max(1).to_string().len().max(2);
            let gutter_width = digits as u16 + 1; // number + trailing space
            let text_width = inner.width.saturating_sub(gutter_width) as usize;

            let wrapped = crate::ui::wrap::wrap_editor_lines(&logical_lines, text_width, cursor, selection);
            let border_dim = Theme::parse_color(&theme.border);
            let sel_bg = Theme::parse_color(&theme.accent);

            let visible = inner.height as usize;
            if visible > 0 {
                if wrapped.cursor_screen_row < tab.editor_scroll as usize {
                    tab.editor_scroll = wrapped.cursor_screen_row as u16;
                } else if wrapped.cursor_screen_row >= tab.editor_scroll as usize + visible {
                    tab.editor_scroll = (wrapped.cursor_screen_row + 1 - visible) as u16;
                }
            }
            let scroll = tab.editor_scroll as usize;

            for (i, line) in wrapped.lines.iter().skip(scroll).take(visible).enumerate() {
                let gutter_text = if line.is_first_segment {
                    format!("{:>width$} ", line.logical_row + 1, width = digits)
                } else {
                    " ".repeat(digits + 1)
                };
                let mut spans = vec![Span::styled(gutter_text, Style::default().fg(border_dim).bg(bg))];
                match line.highlight {
                    Some((h_start, h_end)) => {
                        let chars: Vec<char> = line.text.chars().collect();
                        let before: String = chars[..h_start].iter().collect();
                        let mid: String = chars[h_start..h_end].iter().collect();
                        let after: String = chars[h_end..].iter().collect();
                        if !before.is_empty() {
                            spans.push(Span::styled(before, Style::default().fg(fg).bg(bg)));
                        }
                        spans.push(Span::styled(mid, Style::default().fg(bg).bg(sel_bg)));
                        if !after.is_empty() {
                            spans.push(Span::styled(after, Style::default().fg(fg).bg(bg)));
                        }
                    }
                    None => {
                        spans.push(Span::styled(line.text.clone(), Style::default().fg(fg).bg(bg)));
                    }
                }
                let text_line = Line::from(spans);
                frame.buffer_mut().set_line(inner.x, inner.y + i as u16, &text_line, inner.width);
            }
            let drawn = wrapped.lines.len().saturating_sub(scroll).min(visible);
            for row in drawn..visible {
                for x in inner.x..inner.right() {
                    frame.buffer_mut()[(x, inner.y + row as u16)]
                        .set_char(' ')
                        .set_style(Style::default().bg(bg));
                }
            }

            if wrapped.cursor_screen_row >= scroll && wrapped.cursor_screen_row < scroll + visible {
                let cx = inner.x + gutter_width + wrapped.cursor_screen_col as u16;
                let cy = inner.y + (wrapped.cursor_screen_row - scroll) as u16;
                frame.set_cursor_position((cx, cy));
            }
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
    let border_color = if app.focus == crate::app::Focus::Preview {
        Theme::parse_color(&theme.accent)
    } else {
        Theme::parse_color(&theme.border)
    };
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

    app.preview_area = Some(inner);

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

        let note_dir = tab.note.path.parent().map(|p| p.to_path_buf());
        let vault_root = app.vault.as_ref().map(|v| v.root.clone());
        let scroll = tab.scroll_preview as usize;

        let preview = MarkdownPreview {
            content: &content,
            scroll: tab.scroll_preview,
            theme,
        };
        frame.render_widget(preview, inner);

        blit_images(frame, app, &rendered.images, note_dir.as_deref(), vault_root.as_deref(), scroll, inner);
    } else {
        app.preview_copy_hits.clear();
    }
}

/// Decodes/scales images referenced in the preview (cached per path+size)
/// and paints them into the placeholder boxes left by the markdown
/// renderer: half-block mosaic art always, plus a queued kitty-protocol
/// escape overlay when the terminal supports it.
fn blit_images(
    frame: &mut Frame,
    app: &mut App,
    images: &[crate::ui::preview::ImageSpec],
    note_dir: Option<&std::path::Path>,
    vault_root: Option<&std::path::Path>,
    scroll: usize,
    inner: Rect,
) {
    for img in images {
        if img.row < scroll {
            continue;
        }
        let box_top_visible_row = img.row - scroll;
        if box_top_visible_row >= inner.height as usize {
            continue;
        }
        let is_web = img.path.starts_with("http://") || img.path.starts_with("https://");

        let resolved = if is_web {
            std::path::PathBuf::from(&img.path)
        } else {
            let p = std::path::Path::new(&img.path);
            if p.is_absolute() {
                p.to_path_buf()
            } else {
                // Wikilinks in this app resolve relative to the vault root
                // (see links::resolve_link), so try that convention first
                // and fall back to note-relative for notes kept outside a
                // vault-root-anchored assets layout.
                let vault_candidate = vault_root.map(|d| d.join(p));
                match &vault_candidate {
                    Some(c) if c.exists() => c.clone(),
                    _ => match note_dir {
                        Some(dir) => dir.join(p),
                        None => vault_candidate.unwrap_or_else(|| p.to_path_buf()),
                    },
                }
            }
        };

        let decoded = if is_web {
            fetch_web_image(app, &img.path)
        } else {
            app.decoded_image_cache
                .entry(resolved.clone())
                .or_insert_with(|| crate::ui::images::load_image(&resolved))
                .clone()
        };
        let Some(decoded) = decoded else { continue };

        // Fit to the image's real aspect ratio instead of stretching it to
        // fill the whole placeholder box, then center the result inside it.
        let (fit_cols, fit_rows) =
            crate::ui::images::fit_cells(decoded.width(), decoded.height(), img.inner_width, img.inner_height);
        let off_cols = (img.inner_width.saturating_sub(fit_cols)) / 2;
        let off_rows = (img.inner_height.saturating_sub(fit_rows)) / 2;

        let key = (resolved, fit_cols, fit_rows);
        let prepared = app
            .image_cache
            .entry(key)
            .or_insert_with(|| Some(crate::ui::images::prepare_from_image(&decoded, fit_cols, fit_rows)))
            .clone();
        let Some(prepared) = prepared else { continue };

        let screen_x = inner.x + img.col_start + off_cols;
        let screen_y = inner.y + box_top_visible_row as u16 + off_rows;
        let max_rows = (inner.height as usize).saturating_sub(box_top_visible_row + off_rows as usize);
        let rows_to_draw = (fit_rows as usize).min(max_rows);

        let buf = frame.buffer_mut();
        for (i, line) in prepared.mosaic.iter().take(rows_to_draw).enumerate() {
            buf.set_line(screen_x, screen_y + i as u16, line, fit_cols);
        }

        if let Some(bytes) = prepared.kitty_bytes {
            if rows_to_draw == fit_rows as usize {
                app.pending_kitty_draws.push((screen_x, screen_y, bytes));
            }
        }
    }
}

/// Looks up a web image's decode state, kicking off a background download
/// on first sight of a URL. Never blocks the render loop: while the fetch
/// is in flight (or failed) this just returns `None`, leaving the markdown
/// renderer's placeholder box visible until the next frame it's ready.
fn fetch_web_image(app: &App, url: &str) -> Option<image::DynamicImage> {
    let mut map = app.web_images.lock().unwrap();
    match map.get(url) {
        Some(crate::app::WebImageStatus::Ready(img)) => Some(img.clone()),
        Some(_) => None,
        None => {
            map.insert(url.to_string(), crate::app::WebImageStatus::Loading);
            let url_owned = url.to_string();
            let handle = app.web_images.clone();
            std::thread::spawn(move || {
                let result = crate::ui::images::fetch_image(&url_owned);
                let mut map = handle.lock().unwrap();
                map.insert(
                    url_owned,
                    match result {
                        Some(img) => crate::app::WebImageStatus::Ready(img),
                        None => crate::app::WebImageStatus::Failed,
                    },
                );
            });
            None
        }
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
