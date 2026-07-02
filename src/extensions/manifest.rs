use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Lua,
    Rhai,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub language: Language,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub permissions: Vec<String>,
}

impl Manifest {
    pub fn has_permission(&self, perm: &str) -> bool {
        self.permissions.iter().any(|p| p == perm)
    }

    /// Permisos considerados peligrosos — requieren advertencia extra en UI
    pub fn has_dangerous_permissions(&self) -> bool {
        self.permissions.iter().any(|p| p == "fs.write" || p == "process.run")
    }
}
