pub mod settings;
pub mod theme;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub use settings::Settings;
pub use theme::Theme;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(flatten)]
    pub settings: Settings,
    /// Colores activos (lo que se renderiza)
    pub theme: Theme,
    /// Colores del preset Custom — separados para no mezclarlos con los presets fijos
    #[serde(default)]
    pub custom_theme: Theme,
    /// Índice del preset activo (0=Default 1=Matrix 2=SDRX 3=Custom 4+=user themes)
    #[serde(default)]
    pub active_preset: usize,
    /// Temas exportados por el usuario — cargados al inicio, no se serializan
    #[serde(skip)]
    pub user_themes: Vec<(String, Theme)>,
}

impl Config {
    pub fn config_path() -> PathBuf {
        Settings::config_path()
    }

    pub fn themes_dir() -> PathBuf {
        let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("~/.config"));
        base.join("sdrx-mimic").join("themes")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        let mut cfg: Self = if !path.exists() {
            Self::default()
        } else {
            let content = std::fs::read_to_string(&path).unwrap_or_default();
            toml::from_str(&content).unwrap_or_default()
        };
        cfg.reload_user_themes();
        cfg
    }

    pub fn reload_user_themes(&mut self) {
        let dir = Self::themes_dir();
        let mut themes: Vec<(String, Theme)> = std::fs::read_dir(&dir)
            .into_iter()
            .flatten()
            .filter_map(|e| e.ok())
            .filter_map(|e| {
                let p = e.path();
                if p.extension().map(|x| x == "toml").unwrap_or(false) {
                    let name = p.file_stem()?.to_str()?.to_string();
                    let content = std::fs::read_to_string(&p).ok()?;
                    let theme: Theme = toml::from_str(&content).ok()?;
                    Some((name, theme))
                } else {
                    None
                }
            })
            .collect();
        themes.sort_by(|a, b| a.0.cmp(&b.0));
        self.user_themes = themes;
    }

    pub fn save(&self) {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(content) = toml::to_string_pretty(self) {
            let _ = std::fs::write(path, content);
        }
    }

    pub fn apply_preset(&mut self, idx: usize) {
        self.active_preset = idx;
        if idx < 3 {
            self.theme = Theme::presets()[idx].1.clone();
        } else if idx == 3 {
            self.theme = self.custom_theme.clone();
        } else if let Some((_, t)) = self.user_themes.get(idx - 4) {
            self.theme = t.clone();
        }
    }

    pub fn switch_to_custom_copying_current(&mut self) {
        if self.active_preset != 3 {
            self.custom_theme = self.theme.clone();
            self.active_preset = 3;
        }
    }

    pub fn export_theme(&mut self, name: &str) -> std::io::Result<PathBuf> {
        let dir = Self::themes_dir();
        std::fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{}.toml", name));
        let content = toml::to_string_pretty(&self.custom_theme)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        std::fs::write(&path, content)?;
        self.reload_user_themes();
        Ok(path)
    }

    pub fn import_theme(&mut self, name: &str) -> std::io::Result<()> {
        let path = Self::themes_dir().join(format!("{}.toml", name));
        let content = std::fs::read_to_string(&path)?;
        let theme: Theme = toml::from_str(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        self.custom_theme = theme.clone();
        self.theme = theme;
        self.active_preset = 3;
        self.save();
        Ok(())
    }

    pub fn list_exported_themes() -> Vec<String> {
        let dir = Self::themes_dir();
        std::fs::read_dir(&dir)
            .into_iter()
            .flatten()
            .filter_map(|e| e.ok())
            .filter_map(|e| {
                let p = e.path();
                if p.extension().map(|x| x == "toml").unwrap_or(false) {
                    p.file_stem().and_then(|s| s.to_str()).map(|s| s.to_string())
                } else {
                    None
                }
            })
            .collect()
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            settings: Settings::default(),
            theme: Theme::default(),
            custom_theme: Theme::default(),
            active_preset: 0,
            user_themes: Vec::new(),
        }
    }
}
