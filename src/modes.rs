#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    /// Primera vez: elige directorio default para vaults
    FirstTimeSetup,
    Home,
    FileBrowser,
    /// Dialog para crear nueva vault (solo nombre)
    NewVaultDialog,
    Normal,
    Insert,
    Command,
    /// Ventana de configuración (Extensiones + Temas)
    Settings,
}

/// Tab activo dentro de la ventana Settings
#[derive(Debug, Clone, PartialEq)]
pub enum SettingsTab {
    Extensions,
    Themes,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, Default)]
pub enum ViewMode {
    #[default]
    Editor,
    Split,
    Preview,
}

impl ViewMode {
    pub fn next(&self) -> Self {
        match self {
            ViewMode::Editor => ViewMode::Split,
            ViewMode::Split => ViewMode::Preview,
            ViewMode::Preview => ViewMode::Editor,
        }
    }

    pub fn label(&self) -> &str {
        match self {
            ViewMode::Editor => "EDITOR",
            ViewMode::Split => "SPLIT",
            ViewMode::Preview => "PREVIEW",
        }
    }
}

impl AppMode {
    pub fn label(&self) -> &str {
        match self {
            AppMode::FirstTimeSetup => "SETUP",
            AppMode::Home => "HOME",
            AppMode::FileBrowser => "BROWSER",
            AppMode::NewVaultDialog => "NUEVA VAULT",
            AppMode::Normal => "NORMAL",
            AppMode::Insert => "INSERT",
            AppMode::Command => "COMMAND",
            AppMode::Settings => "SETTINGS",
        }
    }
}
