use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VaultConfig {
    pub path: Option<String>,
    pub last_open_notes: Vec<String>,
    #[serde(default)]
    pub recent: Vec<String>,
    /// Directorio donde se crean las nuevas vaults por defecto
    pub default_vaults_dir: Option<String>,
}

impl VaultConfig {
    pub fn add_recent(&mut self, path: &str) {
        self.recent.retain(|p| p != path);
        self.recent.insert(0, path.to_string());
        self.recent.truncate(20);
        self.path = Some(path.to_string());
    }

    pub fn remove_recent(&mut self, idx: usize) {
        if idx < self.recent.len() {
            self.recent.remove(idx);
        }
    }

    pub fn has_default_dir(&self) -> bool {
        self.default_vaults_dir.is_some()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Settings {
    pub vault: VaultConfig,
    #[serde(default)]
    pub view_mode: crate::modes::ViewMode,
}

impl Settings {
    pub fn config_path() -> PathBuf {
        let base = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"));
        base.join("sdrx-mimic").join("config.toml")
    }
}
