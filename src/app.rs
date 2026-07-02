use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};


use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::DefaultTerminal;
use tui_textarea::TextArea;

use crate::{
    config::{Config, Theme},
    extensions::ExtensionManager,
    extensions::hooks::HookEvent,
    modes::{AppMode, SettingsTab, ViewMode},
    ui::{
        file_browser::{FileBrowserPurpose, FileBrowserState},
        home::HomeState,
        layout::render,
        new_vault_dialog::NewVaultDialog,
        theme_editor::ThemeEditorState,
    },
    vault::{links, Note, Vault},
};

pub struct Tab {
    pub note: Note,
    pub editor: TextArea<'static>,
    pub scroll_preview: u16,
}

impl Tab {
    pub fn new(note: Note) -> Self {
        let content = note.content.clone();
        // tui-textarea 0.7 no soporta word wrap — limitación del widget
        let mut editor = TextArea::from(content.lines().map(String::from).collect::<Vec<_>>());
        editor.set_line_number_style(ratatui::style::Style::default());
        Self {
            note,
            editor,
            scroll_preview: 0,
        }
    }

    pub fn sync_content(&mut self) {
        let new_content = self.editor.lines().join("\n");
        if new_content != self.note.content {
            self.note.content = new_content;
            self.note.dirty = true;
        }
    }
}

pub struct WarningDialog {
    pub ext_name: String,
    pub ext_version: String,
    pub ext_author: String,
    pub permissions: Vec<String>,
    pub is_enable: bool,   // true=toggle enable, false=install
    pub ext_idx: Option<usize>, // index in extension_manager.extensions (for enable)
}

#[derive(Clone, Debug, Default)]
pub struct PreviewCopyHit {
    pub x_start: u16,
    pub x_end: u16,
    pub y: u16,
    pub text: String,
}

pub struct App {
    pub vault: Option<Vault>,
    pub tabs: Vec<Tab>,
    pub active_tab: usize,
    pub sidebar_scroll: usize,
    pub view_mode: ViewMode,
    pub app_mode: AppMode,
    pub config: Config,
    pub command_buffer: String,
    pub status_msg: Option<String>,
    pub status_msg_time: Option<Instant>,
    pub theme_editor: ThemeEditorState,
    pub home_state: HomeState,
    pub file_browser: Option<FileBrowserState>,
    pub new_vault_dialog: Option<NewVaultDialog>,
    pub last_edit_time: Option<Instant>,
    pub last_vault_refresh: Instant,
    pub should_quit: bool,
    pub extension_manager: ExtensionManager,
    pub settings_tab: SettingsTab,
    pub ext_selected: usize,
    pub warning_dialog: Option<WarningDialog>,
    pub preview_copy_hits: Vec<PreviewCopyHit>,
}

impl App {
    pub fn new_at_home(config: Config) -> Self {
        let first_time = !config.settings.vault.has_default_dir();
        let (initial_mode, initial_browser) = if first_time {
            (
                AppMode::FirstTimeSetup,
                Some(FileBrowserState::new(FileBrowserPurpose::FirstTimeSetup)),
            )
        } else {
            (AppMode::Home, None)
        };
        let active_preset = config.active_preset;
        let view_mode = config.settings.view_mode.clone();
        let mut extension_manager = ExtensionManager::new();
        extension_manager.load_all();
        Self {
            vault: None,
            tabs: Vec::new(),
            active_tab: 0,
            sidebar_scroll: 0,
            view_mode,
            app_mode: initial_mode,
            config,
            command_buffer: String::new(),
            status_msg: None,
            status_msg_time: None,
            theme_editor: ThemeEditorState { selected_preset: active_preset, ..ThemeEditorState::default() },
            home_state: HomeState::default(),
            file_browser: initial_browser,
            new_vault_dialog: None,
            last_edit_time: None,
            last_vault_refresh: Instant::now(),
            should_quit: false,
            extension_manager,
            settings_tab: SettingsTab::Extensions,
            ext_selected: 0,
            warning_dialog: None,
            preview_copy_hits: Vec::new(),
        }
    }

    pub fn new_with_vault(vault_path: &Path, config: Config) -> std::io::Result<Self> {
        let vault = Vault::open(vault_path)?;
        let mut app = Self::new_at_home(config);
        app.vault = Some(vault);
        app.app_mode = AppMode::Normal;
        app.new_vault_dialog = None;
        Ok(app)
    }

    pub fn open_vault(&mut self, path_str: &str, create: bool) {
        let path = PathBuf::from(path_str);
        if create {
            let _ = std::fs::create_dir_all(&path);
        }
        match Vault::open(&path) {
            Ok(v) => {
                self.tabs.clear();
                self.active_tab = 0;
                self.config.settings.vault.add_recent(path_str);
                self.config.save();
                self.vault = Some(v);
                self.app_mode = AppMode::Normal;
            }
            Err(e) => {
                self.set_status(format!("Error abriendo vault: {}", e));
            }
        }
    }

    pub fn open_note(&mut self, path: &Path) {
        if let Some(idx) = self.tabs.iter().position(|t| t.note.path == path) {
            self.active_tab = idx;
            return;
        }
        match Note::load(path) {
            Ok(note) => {
                let note_path = note.path.to_string_lossy().to_string();
                let note_content = note.content.clone();
                self.tabs.push(Tab::new(note));
                self.active_tab = self.tabs.len() - 1;
                self.fire_hook(HookEvent::NoteOpen { path: note_path, content: note_content });
            }
            Err(e) => self.set_status(format!("Error abriendo nota: {}", e)),
        }
    }

    pub fn close_active_tab(&mut self) {
        if self.tabs.is_empty() {
            return;
        }
        self.tabs.remove(self.active_tab);
        if self.active_tab >= self.tabs.len() && !self.tabs.is_empty() {
            self.active_tab = self.tabs.len() - 1;
        }
    }

    pub fn next_tab(&mut self) {
        if !self.tabs.is_empty() {
            self.active_tab = (self.active_tab + 1) % self.tabs.len();
        }
    }

    pub fn prev_tab(&mut self) {
        if !self.tabs.is_empty() {
            self.active_tab = if self.active_tab == 0 {
                self.tabs.len() - 1
            } else {
                self.active_tab - 1
            };
        }
    }

    pub fn save_active(&mut self) {
        let result = self.tabs.get_mut(self.active_tab).map(|tab| {
            tab.sync_content();
            let name = tab.note.name().to_string();
            let path = tab.note.path.to_string_lossy().to_string();
            let content = tab.note.content.clone();
            tab.note.save().map(|_| (name, path, content))
        });
        match result {
            Some(Ok((name, path, content))) => {
                self.set_status(format!("Guardado: {}", name));
                self.fire_hook(HookEvent::NoteSave { path, content });
            }
            Some(Err(e)) => self.set_status(format!("Error al guardar: {}", e)),
            None => {}
        }
    }

    pub fn create_note(&mut self, name: &str) {
        if let Some(vault) = &self.vault {
            let base_dir = vault.tree.selected_entry()
                .map(|e| {
                    if e.is_dir {
                        e.path.clone()
                    } else {
                        e.path.parent().unwrap_or(&vault.root).to_path_buf()
                    }
                })
                .unwrap_or_else(|| vault.root.clone());

            let filename = if name.ends_with(".md") {
                name.to_string()
            } else {
                format!("{}.md", name)
            };
            let path = base_dir.join(&filename);
            match Note::create(&path) {
                Ok(note) => {
                    if let Some(v) = &mut self.vault {
                        v.refresh();
                    }
                    self.tabs.push(Tab::new(note));
                    self.active_tab = self.tabs.len() - 1;
                    self.app_mode = AppMode::Insert;
                    self.set_status(format!("Nota creada: {}", filename));
                }
                Err(e) => self.set_status(format!("Error creando nota: {}", e)),
            }
        }
    }

    pub fn create_dir_in_selected(&mut self, name: &str) {
        if let Some(vault) = &self.vault {
            let base = vault.tree.selected_entry()
                .map(|e| {
                    if e.is_dir {
                        e.path.clone()
                    } else {
                        e.path.parent().unwrap_or(&vault.root).to_path_buf()
                    }
                })
                .unwrap_or_else(|| vault.root.clone());
            let new_dir = base.join(name);
            match std::fs::create_dir_all(&new_dir) {
                Ok(_) => {
                    if let Some(v) = &mut self.vault {
                        v.refresh();
                    }
                    self.set_status(format!("Carpeta creada: {}", name));
                }
                Err(e) => self.set_status(format!("Error creando carpeta: {}", e)),
            }
        }
    }

    pub fn change_vault(&mut self, path_str: &str) {
        self.open_vault(path_str, false);
        self.set_status(format!("Vault: {}", path_str));
    }

    pub fn follow_link_at_cursor(&mut self) {
        if let Some(tab) = self.tabs.get(self.active_tab) {
            let content = tab.editor.lines().join("\n");
            let (row, col) = tab.editor.cursor();
            if let Some(link) = links::find_link_at_cursor(&content, row, col) {
                let vault_root = self.vault.as_ref().map(|v| v.root.clone());
                if let Some(root) = vault_root {
                    if let Some(target) = links::resolve_link(&root, &link) {
                        self.open_note(&target);
                    } else {
                        self.set_status(format!(
                            "'{}' no existe. Usa :new {} para crearla.",
                            link, link
                        ));
                    }
                }
            }
        }
    }

    pub fn delete_active_note(&mut self) {
        let result = self.tabs.get(self.active_tab).map(|tab| {
            let path = tab.note.path.clone();
            let name = tab.note.name().to_string();
            std::fs::remove_file(&path).map(|_| name)
        });
        match result {
            Some(Ok(name)) => {
                self.close_active_tab();
                if let Some(v) = &mut self.vault {
                    v.refresh();
                }
                self.set_status(format!("Eliminada: {}", name));
            }
            Some(Err(e)) => self.set_status(format!("Error al eliminar: {}", e)),
            None => self.set_status("Sin nota activa".into()),
        }
    }

    pub fn rename_active_note(&mut self, new_name: &str) {
        if new_name.is_empty() {
            return;
        }
        let new_name = if new_name.ends_with(".md") {
            new_name.to_string()
        } else {
            format!("{}.md", new_name)
        };

        let result = self.tabs.get_mut(self.active_tab).map(|tab| {
            let old_path = tab.note.path.clone();
            let new_path = old_path.parent()
                .unwrap_or_else(|| std::path::Path::new("."))
                .join(&new_name);
            if new_path.exists() {
                return Err(format!("Ya existe: {}", new_name));
            }
            // Guardar cambios pendientes antes de renombrar
            if tab.note.dirty {
                tab.sync_content();
                let _ = tab.note.save();
            }
            std::fs::rename(&old_path, &new_path)
                .map(|_| {
                    tab.note.path = new_path;
                    tab.note.dirty = false;
                    new_name.clone()
                })
                .map_err(|e| e.to_string())
        });

        match result {
            Some(Ok(name)) => {
                if let Some(v) = &mut self.vault {
                    v.refresh();
                }
                self.set_status(format!("Renombrada: {}", name));
            }
            Some(Err(e)) => self.set_status(format!("Error al renombrar: {}", e)),
            None => self.set_status("Sin nota activa".into()),
        }
    }

    pub fn fire_hook(&mut self, event: HookEvent) {
        let _results = self.extension_manager.fire_hook(&event);
        let notifs = self.extension_manager.drain_notifications();
        for n in notifs {
            self.set_status(n);
        }
    }

    fn set_status(&mut self, msg: String) {
        self.status_msg = Some(msg);
        self.status_msg_time = Some(Instant::now());
    }

    fn clear_stale_status(&mut self) {
        if let Some(t) = self.status_msg_time {
            if t.elapsed() > Duration::from_secs(3) {
                self.status_msg = None;
                self.status_msg_time = None;
            }
        }
    }

    pub fn run(mut self, terminal: &mut DefaultTerminal) -> std::io::Result<()> {
        loop {
            self.clear_stale_status();
            self.autosave_if_needed();
            self.refresh_vault_if_needed();
            terminal.draw(|frame| render(frame, &mut self))?;

            if self.should_quit {
                break;
            }

            if event::poll(Duration::from_millis(50))? {
                match event::read()? {
                    Event::Key(key) => self.handle_key(key),
                    Event::Mouse(mouse) => self.handle_mouse(mouse),
                    _ => {}
                }
            }
        }
        Ok(())
    }

    fn handle_mouse(&mut self, mouse: MouseEvent) {
        if !matches!(self.app_mode, AppMode::Normal | AppMode::Insert) {
            return;
        }
        if !matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
            return;
        }

        if let Some(hit) = self.preview_copy_hits.iter().find(|hit| {
            mouse.row == hit.y && mouse.column >= hit.x_start && mouse.column < hit.x_end
        }) {
            if hit.text.is_empty() {
                self.set_status("Bloque vacío: nada que copiar".into());
            } else {
                match copy_to_clipboard(&hit.text) {
                    Ok(_) => self.set_status(format!("Código copiado ({} chars)", hit.text.len())),
                    Err(e) => self.set_status(format!("Error clipboard: {}", e)),
                }
            }
        }
    }

    fn refresh_vault_if_needed(&mut self) {
        if self.last_vault_refresh.elapsed() >= Duration::from_secs(2) {
            self.last_vault_refresh = Instant::now();
            if let Some(v) = &mut self.vault {
                v.refresh();
            }
        }
    }

    fn autosave_if_needed(&mut self) {
        let should = self.last_edit_time
            .map(|t| t.elapsed() >= Duration::from_secs(30))
            .unwrap_or(false);
        if should {
            self.last_edit_time = None;
            self.save_all_dirty_silent();
        }
    }

    fn save_all_dirty_silent(&mut self) {
        for tab in &mut self.tabs {
            if tab.note.dirty {
                tab.sync_content();
                let _ = tab.note.save();
            }
        }
    }

    fn quit_save_all(&mut self) {
        self.save_all_dirty_silent();
        // Persist last opened vault so next launch opens it directly
        if let Some(vault) = &self.vault {
            let path = vault.root.to_string_lossy().to_string();
            self.config.settings.vault.add_recent(&path);
        }
        self.config.save();
        self.should_quit = true;
    }

    fn handle_key(&mut self, key: KeyEvent) {
        // Warning dialog captures all input
        if self.warning_dialog.is_some() {
            self.handle_warning_dialog(key);
            return;
        }
        // Global Ctrl+Q — save everything then quit
        if key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('q') {
            self.quit_save_all();
            return;
        }
        match self.app_mode {
            AppMode::FirstTimeSetup => self.handle_first_time_setup(key),
            AppMode::Home => self.handle_home(key),
            AppMode::FileBrowser => self.handle_file_browser(key),
            AppMode::NewVaultDialog => self.handle_new_vault_dialog(key),
            AppMode::Normal => self.handle_normal(key),
            AppMode::Insert => self.handle_insert(key),
            AppMode::Command => self.handle_command(key),
            AppMode::Settings => self.handle_settings(key),
        }
    }

    fn handle_first_time_setup(&mut self, key: KeyEvent) {
        self.handle_file_browser(key);
    }

    fn handle_new_vault_dialog(&mut self, key: KeyEvent) {
        let dialog = match self.new_vault_dialog.as_mut() {
            Some(d) => d,
            None => {
                self.app_mode = AppMode::Home;
                return;
            }
        };

        match key.code {
            KeyCode::Esc => {
                self.new_vault_dialog = None;
                self.app_mode = AppMode::Home;
            }
            KeyCode::Tab => {
                // Cambiar ruta solo esta vault → abre file browser
                self.file_browser = Some(FileBrowserState::new(FileBrowserPurpose::NewVaultOverridePath));
                self.app_mode = AppMode::FileBrowser;
            }
            KeyCode::Enter => {
                let name = dialog.name.trim().to_string();
                if name.is_empty() {
                    return;
                }
                let default_dir = self.config.settings.vault.default_vaults_dir
                    .clone()
                    .unwrap_or_default();
                let full_path = dialog.resolved_path(&default_dir);
                let path_str = full_path.to_string_lossy().to_string();
                self.new_vault_dialog = None;
                self.open_vault(&path_str, true);
            }
            KeyCode::Backspace => {
                dialog.name.pop();
            }
            KeyCode::Char(c) => {
                // Solo chars válidos para nombre de directorio
                if !"/\\:*?\"<>|".contains(c) {
                    dialog.name.push(c);
                }
            }
            _ => {}
        }
    }

    fn handle_home(&mut self, key: KeyEvent) {
        let recent_len = self.config.settings.vault.recent.len();
        match key.code {
            KeyCode::Char('q') | KeyCode::Char('Q') => {
                self.should_quit = true;
            }
            KeyCode::Char('n') => {
                // Nueva vault → dialog de nombre (usa default dir)
                self.new_vault_dialog = Some(NewVaultDialog::default());
                self.app_mode = AppMode::NewVaultDialog;
            }
            KeyCode::Char('o') => {
                self.file_browser = Some(FileBrowserState::new(FileBrowserPurpose::OpenVault));
                self.app_mode = AppMode::FileBrowser;
            }
            KeyCode::Char('s') => {
                // Cambiar directorio default
                self.file_browser = Some(FileBrowserState::new(FileBrowserPurpose::SetDefaultDir));
                self.app_mode = AppMode::FileBrowser;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if recent_len > 0 && self.home_state.selected + 1 < recent_len {
                    self.home_state.selected += 1;
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.home_state.selected > 0 {
                    self.home_state.selected -= 1;
                }
            }
            KeyCode::Enter => {
                if !self.config.settings.vault.recent.is_empty() {
                    let path = self.config.settings.vault.recent[self.home_state.selected].clone();
                    self.open_vault(&path, false);
                }
            }
            KeyCode::Char('d') => {
                let idx = self.home_state.selected;
                self.config.settings.vault.remove_recent(idx);
                self.config.save();
                if self.home_state.selected >= self.config.settings.vault.recent.len()
                    && self.home_state.selected > 0
                {
                    self.home_state.selected -= 1;
                }
            }
            _ => {}
        }
    }

    fn handle_file_browser(&mut self, key: KeyEvent) {
        let browser = match self.file_browser.as_mut() {
            Some(b) => b,
            None => {
                self.app_mode = AppMode::Home;
                return;
            }
        };

        match key.code {
            KeyCode::Esc => {
                let purpose = browser.purpose.clone();
                self.file_browser = None;
                match purpose {
                    FileBrowserPurpose::FirstTimeSetup | FileBrowserPurpose::SetDefaultDir => {
                        // Si cancela first-time setup sin elegir, igual va al home
                        self.app_mode = AppMode::Home;
                    }
                    FileBrowserPurpose::NewVaultOverridePath => {
                        // Vuelve al dialog de nueva vault
                        self.app_mode = AppMode::NewVaultDialog;
                    }
                    FileBrowserPurpose::OpenVault => {
                        self.app_mode = AppMode::Home;
                    }
                }
            }
            KeyCode::Char('j') | KeyCode::Down => browser.move_down(),
            KeyCode::Char('k') | KeyCode::Up => browser.move_up(),
            KeyCode::Enter => browser.enter_selected(),
            KeyCode::Char('h') | KeyCode::Left | KeyCode::Backspace => browser.go_up(),
            KeyCode::Char(' ') => {
                let path = browser.selected_path();
                let purpose = browser.purpose.clone();
                let path_str = path.to_string_lossy().to_string();
                self.file_browser = None;

                match purpose {
                    FileBrowserPurpose::FirstTimeSetup | FileBrowserPurpose::SetDefaultDir => {
                        self.config.settings.vault.default_vaults_dir = Some(path_str);
                        self.config.save();
                        self.set_status(format!(
                            "Directorio default: {}",
                            self.config.settings.vault.default_vaults_dir.as_deref().unwrap_or("")
                        ));
                        self.app_mode = AppMode::Home;
                    }
                    FileBrowserPurpose::NewVaultOverridePath => {
                        // Guarda override en el dialog y vuelve a él
                        if let Some(dialog) = self.new_vault_dialog.as_mut() {
                            dialog.override_path = Some(path);
                        }
                        self.app_mode = AppMode::NewVaultDialog;
                    }
                    FileBrowserPurpose::OpenVault => {
                        self.open_vault(&path_str, false);
                    }
                }
            }
            _ => {}
        }
    }

    fn handle_normal(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (KeyModifiers::CONTROL, KeyCode::Char('t')) => {
                self.app_mode = AppMode::Settings;
                self.settings_tab = SettingsTab::Extensions;
            }
            (KeyModifiers::CONTROL, KeyCode::Char('v')) => {
                self.view_mode = self.view_mode.next();
                self.config.settings.view_mode = self.view_mode.clone();
                self.config.save();
            }
            (KeyModifiers::NONE, KeyCode::Char('i')) => {
                if self.view_mode != ViewMode::Preview {
                    self.app_mode = AppMode::Insert;
                }
            }
            (KeyModifiers::NONE, KeyCode::Char('e')) => {
                self.app_mode = AppMode::Insert;
            }
            (KeyModifiers::NONE, KeyCode::Char(':')) => {
                self.app_mode = AppMode::Command;
                self.command_buffer.clear();
            }
            (KeyModifiers::NONE, KeyCode::Tab) => self.next_tab(),
            (KeyModifiers::SHIFT, KeyCode::BackTab) => self.prev_tab(),
            (KeyModifiers::NONE, KeyCode::Char('j')) | (KeyModifiers::NONE, KeyCode::Down) => {
                if let Some(v) = &mut self.vault {
                    v.tree.move_down();
                }
                self.sync_sidebar_scroll();
                self.sync_preview_scroll_to_cursor();
            }
            (KeyModifiers::NONE, KeyCode::Char('k')) | (KeyModifiers::NONE, KeyCode::Up) => {
                if let Some(v) = &mut self.vault {
                    v.tree.move_up();
                }
                self.sync_sidebar_scroll();
                self.sync_preview_scroll_to_cursor();
            }
            (KeyModifiers::NONE, KeyCode::Char('d')) => {
                if matches!(self.view_mode, ViewMode::Preview | ViewMode::Split) {
                    if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                        tab.scroll_preview = tab.scroll_preview.saturating_add(5);
                    }
                }
            }
            (KeyModifiers::NONE, KeyCode::Char('u')) => {
                if matches!(self.view_mode, ViewMode::Preview | ViewMode::Split) {
                    if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                        tab.scroll_preview = tab.scroll_preview.saturating_sub(5);
                    }
                }
            }
            (KeyModifiers::NONE, KeyCode::Enter) => {
                let entry_info = self.vault.as_ref()
                    .and_then(|v| v.tree.selected_entry())
                    .map(|e| (e.is_dir, e.path.clone()));
                match entry_info {
                    Some((true, _)) => {
                        if let Some(v) = &mut self.vault {
                            v.tree.toggle_dir();
                        }
                    }
                    Some((false, path)) => self.open_note(&path),
                    None => {}
                }
            }
            (KeyModifiers::NONE, KeyCode::Char('g')) => {
                self.follow_link_at_cursor();
            }
            (KeyModifiers::NONE, KeyCode::Char('r')) => {
                self.app_mode = AppMode::Command;
                self.command_buffer = "rename ".to_string();
            }
            (KeyModifiers::SHIFT, KeyCode::Char('D')) | (KeyModifiers::NONE, KeyCode::Char('D')) => {
                self.app_mode = AppMode::Command;
                self.command_buffer = "delete".to_string();
            }
            (KeyModifiers::CONTROL, KeyCode::Char('d')) => {
                self.app_mode = AppMode::Command;
                self.command_buffer = "delete".to_string();
            }
            (KeyModifiers::CONTROL, KeyCode::Char('w')) => self.close_active_tab(),
            (KeyModifiers::CONTROL, KeyCode::Char('s')) => self.save_active(),
            (KeyModifiers::CONTROL, KeyCode::Char('c'))
            | (KeyModifiers::CONTROL | KeyModifiers::SHIFT, KeyCode::Char('C')) => {
                let text = self.tabs.get(self.active_tab)
                    .map(|t| t.editor.lines().join("\n"))
                    .unwrap_or_default();
                if !text.is_empty() {
                    match copy_to_clipboard(&text) {
                        Ok(_) => self.set_status(format!("Nota copiada ({} chars)", text.len())),
                        Err(e) => self.set_status(format!("Error clipboard: {}", e)),
                    }
                }
            }
            (KeyModifiers::CONTROL, KeyCode::Char('h')) => {
                self.app_mode = AppMode::Home;
            }
            _ => {}
        }
    }

    fn handle_insert(&mut self, key: KeyEvent) {
        // Ctrl+S funciona desde Insert
        if key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('s') {
            self.save_active();
            return;
        }

        // Ctrl+C o Ctrl+Shift+C → copiar selección al clipboard del sistema
        let is_copy = (key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('c'))
            || (key.modifiers == (KeyModifiers::CONTROL | KeyModifiers::SHIFT)
                && key.code == KeyCode::Char('C'));
        if is_copy {
            if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                tab.editor.copy();
                let text = tab.editor.yank_text().to_string();
                if text.is_empty() {
                    self.set_status("Nada seleccionado (Shift+Flechas para seleccionar)".into());
                } else {
                    match copy_to_clipboard(&text) {
                        Ok(_) => self.set_status(format!("Copiado ({} chars)", text.len())),
                        Err(e) => self.set_status(format!("Error clipboard: {}", e)),
                    }
                }
            }
            return;
        }

        // Ctrl+Z → undo
        if key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('z') {
            if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                tab.editor.undo();
                tab.note.dirty = true;
                self.last_edit_time = Some(Instant::now());
            }
            return;
        }

        // Ctrl+Y → redo
        if key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('y') {
            if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                tab.editor.redo();
                tab.note.dirty = true;
                self.last_edit_time = Some(Instant::now());
            }
            return;
        }

        if key.code == KeyCode::Esc {
            // Trailing newline: si la última línea no está vacía, añadir línea en blanco
            if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                let needs_newline = tab.editor.lines()
                    .last()
                    .map(|l| !l.is_empty())
                    .unwrap_or(false);
                if needs_newline {
                    tab.editor.move_cursor(tui_textarea::CursorMove::Bottom);
                    tab.editor.move_cursor(tui_textarea::CursorMove::End);
                    tab.editor.insert_newline();
                    tab.note.dirty = true;
                }
            }

            // Autosave al salir de Insert
            let result = self.tabs.get_mut(self.active_tab).map(|tab| {
                tab.sync_content();
                let name = tab.note.name().to_string();
                if tab.note.dirty {
                    tab.note.save().map(|_| name)
                } else {
                    Ok(String::new())
                }
            });
            if let Some(Ok(name)) = result {
                if !name.is_empty() {
                    self.set_status(format!("Guardado: {}", name));
                }
            }
            self.last_edit_time = None;
            self.app_mode = AppMode::Normal;
            return;
        }

        // Auto-continuar listas al presionar Enter
        if key.modifiers == KeyModifiers::NONE && key.code == KeyCode::Enter {
            if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                let (row, _) = tab.editor.cursor();
                let current_line = tab.editor.lines().get(row).cloned().unwrap_or_default();
                if let Some(prefix) = detect_list_prefix(&current_line) {
                    let content_after = &current_line[prefix.len()..];
                    if content_after.trim().is_empty() {
                        // Línea solo tiene el prefijo → eliminar y dejar línea vacía
                        tab.editor.move_cursor(tui_textarea::CursorMove::End);
                        tab.editor.delete_line_by_head();
                        tab.editor.insert_newline();
                    } else {
                        tab.editor.insert_newline();
                        tab.editor.insert_str(&prefix);
                    }
                    tab.note.dirty = true;
                    self.last_edit_time = Some(Instant::now());
                    return;
                }
            }
        }

        // Ctrl+Backspace → borrar palabra anterior
        // Distintos terminales envían distintos códigos para Ctrl+Backspace
        let is_delete_word = key.modifiers == KeyModifiers::CONTROL && matches!(
            key.code,
            KeyCode::Backspace          // kitty, algunos otros
            | KeyCode::Char('h')        // \x08 BS — xterm, alacritty
            | KeyCode::Char('w')        // Ctrl+W clásico de terminal
            | KeyCode::Char('\x7f')     // DEL con CONTROL
        );
        if is_delete_word {
            if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                tab.editor.delete_word();
                tab.note.dirty = true;
                self.last_edit_time = Some(Instant::now());
            }
            return;
        }

        // Auto-cierre de pares de caracteres
        if key.modifiers == KeyModifiers::NONE {
            let pair = match key.code {
                KeyCode::Char('"') => Some(('"', '"')),
                KeyCode::Char('\'') => Some(('\'', '\'')),
                KeyCode::Char('`') => Some(('`', '`')),
                _ => None,
            };
            if let Some((open, close)) = pair {
                if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                    tab.editor.insert_char(open);
                    tab.editor.insert_char(close);
                    tab.editor.move_cursor(tui_textarea::CursorMove::Back);
                    tab.note.dirty = true;
                    self.last_edit_time = Some(Instant::now());
                    return;
                }
            }
        }

        if let Some(tab) = self.tabs.get_mut(self.active_tab) {
            tab.editor.input(key);
            tab.note.dirty = true;
            self.last_edit_time = Some(Instant::now());
            // Sincronizar scroll del preview con la posición del cursor
            if matches!(self.view_mode, ViewMode::Split | ViewMode::Preview) {
                let total = tab.editor.lines().len().max(1);
                let (cursor_row, _) = tab.editor.cursor();
                let ratio = cursor_row as f32 / total as f32;
                let target = (ratio * total as f32) as u16;
                tab.scroll_preview = target.saturating_sub(5);
            }
        }
    }

    fn handle_command(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.app_mode = AppMode::Normal;
                self.command_buffer.clear();
            }
            KeyCode::Enter => {
                let cmd = self.command_buffer.clone();
                self.command_buffer.clear();
                self.app_mode = AppMode::Normal;
                self.execute_command(&cmd);
            }
            KeyCode::Backspace => {
                self.command_buffer.pop();
            }
            KeyCode::Char(c) => {
                self.command_buffer.push(c);
            }
            _ => {}
        }
    }

    fn execute_command(&mut self, cmd: &str) {
        let parts: Vec<&str> = cmd.trim().splitn(2, ' ').collect();
        match parts.as_slice() {
            ["w"] | ["w", ""] => self.save_active(),
            ["q"] => {
                if self.tabs.get(self.active_tab).map(|t| t.note.dirty).unwrap_or(false) {
                    self.set_status(
                        "Cambios sin guardar. Usa :q! para forzar o :w primero.".into(),
                    );
                } else {
                    self.close_active_tab();
                }
            }
            ["q!"] => self.close_active_tab(),
            ["qa"] | ["qa!"] => {
                self.should_quit = true;
            }
            ["wq"] => {
                self.save_active();
                self.close_active_tab();
            }
            ["home"] => {
                self.app_mode = AppMode::Home;
            }
            ["rename", new_name] => {
                let new_name = new_name.trim().to_string();
                self.rename_active_note(&new_name);
            }
            ["mv", new_name] => {
                let new_name = new_name.trim().to_string();
                self.rename_active_note(&new_name);
            }
            ["delete"] | ["delete!"] | ["rm"] | ["rm!"] => {
                self.delete_active_note();
            }
            ["new", name] => {
                let name = name.trim().to_string();
                self.create_note(&name);
            }
            ["mkdir", name] => {
                let name = name.trim().to_string();
                self.create_dir_in_selected(&name);
            }
            ["vault", path] => {
                let path = path.trim().to_string();
                self.change_vault(&path);
            }
            ["export-tema", name] => {
                let name = name.trim().to_string();
                match self.config.export_theme(&name) {
                    Ok(path) => self.set_status(format!("Tema exportado: {}", path.display())),
                    Err(e) => self.set_status(format!("Error exportando: {}", e)),
                }
            }
            ["import-tema", name] => {
                let name = name.trim().to_string();
                match self.config.import_theme(&name) {
                    Ok(_) => {
                        self.theme_editor.selected_preset = 3;
                        self.set_status(format!("Tema importado: {}", name));
                    }
                    Err(e) => self.set_status(format!("Error importando: {}", e)),
                }
            }
            ["temas"] => {
                let list = crate::config::Config::list_exported_themes();
                if list.is_empty() {
                    self.set_status("Sin temas exportados.".into());
                } else {
                    self.set_status(format!("Temas: {}", list.join(", ")));
                }
            }
            ["help"] => {
                self.set_status(
                    ":w :q :qa :new <nom> :export-tema <nom> :import-tema <nom> :temas".into(),
                );
            }
            // Extension commands: :ext install <path>, :ext remove <name>, :ext list, :ext enable/disable <name>
            ["ext", rest] => {
                let rest = rest.trim();
                let parts: Vec<&str> = rest.splitn(2, ' ').collect();
                match parts.as_slice() {
                    ["list"] => {
                        let names: Vec<String> = self.extension_manager.extensions.iter()
                            .map(|e| format!("{}({})", e.manifest.name, if e.manifest.enabled { "on" } else { "off" }))
                            .collect();
                        if names.is_empty() {
                            self.set_status("Sin extensiones instaladas.".into());
                        } else {
                            self.set_status(format!("Extensiones: {}", names.join(", ")));
                        }
                    }
                    ["install", path] => {
                        let src = std::path::PathBuf::from(path.trim());
                        match crate::extensions::ExtensionManager::read_manifest_from(&src) {
                            Ok(manifest) => {
                                let wd = WarningDialog {
                                    ext_name: manifest.name.clone(),
                                    ext_version: manifest.version.clone(),
                                    ext_author: manifest.author.clone(),
                                    permissions: manifest.permissions.clone(),
                                    is_enable: false,
                                    ext_idx: None,
                                };
                                // Perform install then show warning (install disabled by default)
                                match self.extension_manager.install_from(&src) {
                                    Ok(name) => {
                                        self.warning_dialog = Some(wd);
                                        self.set_status(format!("'{}' instalada (desactivada). Activa en Configuración (Ctrl+T).", name));
                                    }
                                    Err(e) => self.set_status(format!("Error instalando: {}", e)),
                                }
                            }
                            Err(e) => self.set_status(format!("Error leyendo manifest: {}", e)),
                        }
                    }
                    ["remove", name] => {
                        let name = name.trim().to_string();
                        match self.extension_manager.remove(&name) {
                            Ok(_) => self.set_status(format!("Extensión '{}' eliminada.", name)),
                            Err(e) => self.set_status(format!("Error: {}", e)),
                        }
                    }
                    ["enable", name] => {
                        let name = name.trim();
                        if let Some(idx) = self.extension_manager.extensions.iter().position(|e| e.manifest.name == name) {
                            let wd = {
                                let e = &self.extension_manager.extensions[idx];
                                WarningDialog {
                                    ext_name: e.manifest.name.clone(),
                                    ext_version: e.manifest.version.clone(),
                                    ext_author: e.manifest.author.clone(),
                                    permissions: e.manifest.permissions.clone(),
                                    is_enable: true,
                                    ext_idx: Some(idx),
                                }
                            };
                            self.warning_dialog = Some(wd);
                        } else {
                            self.set_status(format!("Extensión '{}' no encontrada.", name));
                        }
                    }
                    ["disable", name] => {
                        let name = name.trim().to_string();
                        if let Some(idx) = self.extension_manager.extensions.iter().position(|e| e.manifest.name == name) {
                            self.extension_manager.disable(idx);
                            self.set_status(format!("Extensión '{}' desactivada.", name));
                        } else {
                            self.set_status(format!("Extensión '{}' no encontrada.", name));
                        }
                    }
                    _ => self.set_status("Uso: :ext list|install <ruta>|remove <nom>|enable <nom>|disable <nom>".into()),
                }
            }
            _ => {
                // Try extension commands before reporting unknown
                let parts: Vec<&str> = cmd.trim().splitn(2, ' ').collect();
                let (cmd_name, args_str) = match parts.as_slice() {
                    [name] => (*name, ""),
                    [name, rest] => (*name, *rest),
                    _ => {
                        self.set_status(format!("Comando desconocido: {}", cmd));
                        return;
                    }
                };
                let args: Vec<String> = args_str.split_whitespace().map(String::from).collect();
                if let Some(result) = self.extension_manager.dispatch_command(cmd_name, &args) {
                    self.set_status(result);
                } else {
                    self.set_status(format!("Comando desconocido: {}", cmd));
                }
            }
        }
    }

    fn handle_warning_dialog(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                if let Some(wd) = self.warning_dialog.take() {
                    if wd.is_enable {
                        if let Some(idx) = wd.ext_idx {
                            self.extension_manager.enable(idx);
                            self.set_status(format!("Extensión '{}' activada", wd.ext_name));
                        }
                    }
                    // install was already done before showing dialog; just reload
                    self.extension_manager.load_all();
                }
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                let name = self.warning_dialog.as_ref().map(|w| w.ext_name.clone()).unwrap_or_default();
                self.warning_dialog = None;
                if !name.is_empty() {
                    self.set_status(format!("Cancelado: '{}'", name));
                }
            }
            _ => {}
        }
    }

    fn handle_settings(&mut self, key: KeyEvent) {

        // If on Themes tab, delegate to theme editor handler
        if self.settings_tab == SettingsTab::Themes {
            match key.code {
                KeyCode::Esc => { self.app_mode = AppMode::Normal; }
                KeyCode::Tab => { self.settings_tab = SettingsTab::Extensions; }
                KeyCode::Char('1') => { self.settings_tab = SettingsTab::Extensions; }
                _ => self.handle_theme_editor(key),
            }
            return;
        }

        // Extensions tab
        let ext_count = self.extension_manager.extensions.len();
        match key.code {
            KeyCode::Esc => { self.app_mode = AppMode::Normal; }
            KeyCode::Tab => { self.settings_tab = SettingsTab::Themes; }
            KeyCode::Char('2') => { self.settings_tab = SettingsTab::Themes; }
            KeyCode::Char('j') | KeyCode::Down => {
                if ext_count > 0 && self.ext_selected + 1 < ext_count {
                    self.ext_selected += 1;
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.ext_selected > 0 {
                    self.ext_selected -= 1;
                }
            }
            KeyCode::Char(' ') | KeyCode::Enter => {
                if let Some(entry) = self.extension_manager.extensions.get(self.ext_selected) {
                    let wd = WarningDialog {
                        ext_name: entry.manifest.name.clone(),
                        ext_version: entry.manifest.version.clone(),
                        ext_author: entry.manifest.author.clone(),
                        permissions: entry.manifest.permissions.clone(),
                        is_enable: true,
                        ext_idx: Some(self.ext_selected),
                    };
                    if entry.manifest.enabled {
                        // Disable immediately without dialog
                        let idx = self.ext_selected;
                        let name = entry.manifest.name.clone();
                        self.extension_manager.disable(idx);
                        self.set_status(format!("Extensión '{}' desactivada", name));
                    } else {
                        self.warning_dialog = Some(wd);
                    }
                }
            }
            KeyCode::Delete => {
                if let Some(entry) = self.extension_manager.extensions.get(self.ext_selected) {
                    let name = entry.manifest.name.clone();
                    match self.extension_manager.remove(&name) {
                        Ok(_) => {
                            if self.ext_selected > 0 { self.ext_selected -= 1; }
                            self.set_status(format!("Extensión '{}' eliminada", name));
                        }
                        Err(e) => self.set_status(format!("Error eliminando: {}", e)),
                    }
                }
            }
            _ => {}
        }
    }

    fn handle_theme_editor(&mut self, key: KeyEvent) {
        use crate::ui::theme_editor::ThemeEditorFocus;

        // ── Export name input ────────────────────────────────────────────────
        if self.theme_editor.exporting {
            match key.code {
                KeyCode::Enter => {
                    let name = self.theme_editor.export_input.trim().to_string();
                    self.theme_editor.exporting = false;
                    self.theme_editor.export_input.clear();
                    if !name.is_empty() {
                        match self.config.export_theme(&name) {
                            Ok(_) => self.set_status(format!("Tema exportado: {}", name)),
                            Err(e) => self.set_status(format!("Error exportando: {}", e)),
                        }
                    }
                }
                KeyCode::Esc => {
                    self.theme_editor.exporting = false;
                    self.theme_editor.export_input.clear();
                }
                KeyCode::Backspace => { self.theme_editor.export_input.pop(); }
                KeyCode::Char(c) => { self.theme_editor.export_input.push(c); }
                _ => {}
            }
            return;
        }

        // ── Import name input ────────────────────────────────────────────────
        if self.theme_editor.importing {
            match key.code {
                KeyCode::Enter => {
                    let name = self.theme_editor.import_input.trim().to_string();
                    self.theme_editor.importing = false;
                    self.theme_editor.import_input.clear();
                    if !name.is_empty() {
                        match self.config.import_theme(&name) {
                            Ok(_) => {
                                self.theme_editor.selected_preset = 3;
                                self.set_status(format!("Tema importado: {}", name));
                            }
                            Err(e) => self.set_status(format!("Error importando: {}", e)),
                        }
                    }
                }
                KeyCode::Esc => {
                    self.theme_editor.importing = false;
                    self.theme_editor.import_input.clear();
                }
                KeyCode::Backspace => { self.theme_editor.import_input.pop(); }
                KeyCode::Char(c) => { self.theme_editor.import_input.push(c); }
                _ => {}
            }
            return;
        }

        // ── Editing a hex color value ────────────────────────────────────────
        if self.theme_editor.editing {
            match key.code {
                KeyCode::Enter => {
                    let val = self.theme_editor.input_buffer.clone();
                    let idx = self.theme_editor.selected_field;
                    self.config.theme.set_field_by_index(idx, val.clone());
                    self.config.custom_theme.set_field_by_index(idx, val);
                    self.config.save();
                    self.theme_editor.editing = false;
                }
                KeyCode::Esc => { self.theme_editor.editing = false; }
                KeyCode::Backspace => { self.theme_editor.input_buffer.pop(); }
                KeyCode::Char(c) => { self.theme_editor.input_buffer.push(c); }
                _ => {}
            }
            return;
        }

        let total_presets = Theme::preset_names().len() + self.config.user_themes.len();

        // ── Preset selector focus ────────────────────────────────────────────
        if self.theme_editor.focus == ThemeEditorFocus::Presets {
            match key.code {
                KeyCode::Esc => { self.app_mode = AppMode::Normal; }
                KeyCode::Left | KeyCode::Char('h') => {
                    if self.theme_editor.selected_preset > 0 {
                        self.theme_editor.selected_preset -= 1;
                    }
                }
                KeyCode::Right | KeyCode::Char('l') => {
                    if self.theme_editor.selected_preset + 1 < total_presets {
                        self.theme_editor.selected_preset += 1;
                    }
                }
                KeyCode::Enter => {
                    let idx = self.theme_editor.selected_preset;
                    self.config.apply_preset(idx);
                    self.config.active_preset = idx;
                    self.config.save();
                    let name = if idx < Theme::preset_names().len() {
                        Theme::preset_names()[idx].to_string()
                    } else {
                        self.config.user_themes.get(idx - Theme::preset_names().len())
                            .map(|(n, _)| n.clone())
                            .unwrap_or_default()
                    };
                    self.set_status(format!("Tema: {}", name));
                }
                KeyCode::Char('e') => {
                    self.theme_editor.exporting = true;
                    self.theme_editor.export_input.clear();
                }
                KeyCode::Char('i') => {
                    self.theme_editor.importing = true;
                    self.theme_editor.import_input.clear();
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.theme_editor.focus = ThemeEditorFocus::Fields;
                }
                KeyCode::Tab => { self.theme_editor.focus = ThemeEditorFocus::Fields; }
                _ => {}
            }
            return;
        }

        // ── Fields focus ─────────────────────────────────────────────────────
        match key.code {
            KeyCode::Esc => { self.app_mode = AppMode::Normal; }
            KeyCode::Tab | KeyCode::Up if self.theme_editor.selected_field == 0 => {
                self.theme_editor.focus = ThemeEditorFocus::Presets;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.theme_editor.selected_field > 0 {
                    self.theme_editor.selected_field -= 1;
                } else {
                    self.theme_editor.focus = ThemeEditorFocus::Presets;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.theme_editor.selected_field + 1 < 11 {
                    self.theme_editor.selected_field += 1;
                }
            }
            KeyCode::Char('e') => {
                self.theme_editor.exporting = true;
                self.theme_editor.export_input.clear();
            }
            KeyCode::Char('i') => {
                self.theme_editor.importing = true;
                self.theme_editor.import_input.clear();
            }
            KeyCode::Enter => {
                if self.theme_editor.selected_preset != 3 {
                    self.config.switch_to_custom_copying_current();
                    self.theme_editor.selected_preset = 3;
                    self.config.save();
                    self.set_status("Cambiado a Custom".into());
                }
                let idx = self.theme_editor.selected_field;
                self.theme_editor.input_buffer = self.config.theme.get_field_by_index(idx);
                self.theme_editor.editing = true;
            }
            _ => {}
        }
    }

    fn sync_sidebar_scroll(&mut self) {
        let selected = self.vault.as_ref().map(|v| v.tree.selected).unwrap_or(0);
        let visible = 20usize;
        if selected < self.sidebar_scroll {
            self.sidebar_scroll = selected;
        } else if selected >= self.sidebar_scroll + visible {
            self.sidebar_scroll = selected + 1 - visible;
        }
    }

    fn sync_preview_scroll_to_cursor(&mut self) {
        if !matches!(self.view_mode, ViewMode::Split | ViewMode::Preview) {
            return;
        }
        if let Some(tab) = self.tabs.get_mut(self.active_tab) {
            let total = tab.editor.lines().len().max(1);
            let (cursor_row, _) = tab.editor.cursor();
            let ratio = cursor_row as f32 / total as f32;
            let target = (ratio * total as f32) as u16;
            tab.scroll_preview = target.saturating_sub(5);
        }
    }
}

fn copy_to_clipboard(text: &str) -> Result<(), &'static str> {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let has_wayland = std::env::var("WAYLAND_DISPLAY").is_ok();
    let has_x11 = std::env::var("DISPLAY").is_ok();

    // wl-copy (Wayland) — solo si WAYLAND_DISPLAY está definido
    if has_wayland {
        if let Ok(mut child) = Command::new("wl-copy")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(text.as_bytes());
            }
            return Ok(());
        }
    }

    // xclip (X11) — solo si DISPLAY está definido
    if has_x11 {
        if let Ok(mut child) = Command::new("xclip")
            .args(["-selection", "clipboard"])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(text.as_bytes());
            }
            return Ok(());
        }

        // xsel (X11 alternativo)
        if let Ok(mut child) = Command::new("xsel")
            .args(["--clipboard", "--input"])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(text.as_bytes());
            }
            return Ok(());
        }
    }

    Err("Sin servidor gráfico disponible (WAYLAND_DISPLAY / DISPLAY no definidos)")
}

fn detect_list_prefix(line: &str) -> Option<String> {
    let indent_len = line.len() - line.trim_start().len();
    let indent = &line[..indent_len];
    let trimmed = &line[indent_len..];
    for marker in &["* ", "- ", "+ ", "> "] {
        if trimmed.starts_with(marker) {
            return Some(format!("{}{}", indent, marker));
        }
    }
    // Ordered lists: "1. ", "2. " etc — incrementa el número
    let digits: String = trimmed.chars().take_while(|c| c.is_ascii_digit()).collect();
    if !digits.is_empty() {
        let rest = &trimmed[digits.len()..];
        if rest.starts_with(". ") {
            let n: u64 = digits.parse().unwrap_or(1);
            return Some(format!("{}{}. ", indent, n + 1));
        }
    }
    None
}
