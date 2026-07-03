pub mod hooks;
pub mod lua_runtime;
pub mod manifest;
pub mod rhai_runtime;
pub mod runtime;

use std::path::{Path, PathBuf};

use manifest::{Language, Manifest};
use hooks::{HookEvent, HookResult};
use runtime::ExtRuntime;

pub struct ExtensionEntry {
    pub manifest: Manifest,
    pub dir: PathBuf,
    runtime: Option<Box<dyn ExtRuntime>>,
}

impl ExtensionEntry {
    fn load_runtime(&mut self, vault_root: Option<&Path>) {
        if !self.manifest.enabled {
            self.runtime = None;
            return;
        }
        let script_name = match self.manifest.language {
            Language::Lua => "main.lua",
            Language::Rhai => "main.rhai",
        };
        let script_path = self.dir.join(script_name);
        if !script_path.exists() {
            return;
        }
        let result: Result<Box<dyn ExtRuntime>, String> = match self.manifest.language {
            Language::Lua => lua_runtime::LuaRuntime::new(&self.manifest, &script_path, vault_root)
                .map(|r| Box::new(r) as Box<dyn ExtRuntime>),
            Language::Rhai => rhai_runtime::RhaiRuntime::new(&self.manifest, &script_path, vault_root)
                .map(|r| Box::new(r) as Box<dyn ExtRuntime>),
        };
        match result {
            Ok(rt) => self.runtime = Some(rt),
            Err(e) => eprintln!("[mimic] Extension '{}' failed to load: {}", self.manifest.name, e),
        }
    }
}

pub struct ExtensionManager {
    pub extensions: Vec<ExtensionEntry>,
    extensions_dir: PathBuf,
    vault_root: Option<PathBuf>,
}

impl ExtensionManager {
    pub fn new() -> Self {
        let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("~/.config"));
        let extensions_dir = base.join("sdrx-mimic").join("extensions");
        let _ = std::fs::create_dir_all(&extensions_dir);
        Self {
            extensions: Vec::new(),
            extensions_dir,
            vault_root: None,
        }
    }

    /// Path to the per-vault extensions directory: `<vault_root>/.mimic/extensions/`
    pub fn vault_extensions_dir(vault_root: &Path) -> PathBuf {
        vault_root.join(".mimic").join("extensions")
    }

    /// Extension manager scoped to a single vault
    pub fn new_for_vault(vault_root: &Path) -> Self {
        let extensions_dir = Self::vault_extensions_dir(vault_root);
        let _ = std::fs::create_dir_all(&extensions_dir);
        Self {
            extensions: Vec::new(),
            extensions_dir,
            vault_root: Some(vault_root.to_path_buf()),
        }
    }

    /// Scan extensions dir and load manifests (does not start runtimes)
    pub fn load_all(&mut self) {
        self.extensions.clear();
        let dir = self.extensions_dir.clone();
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => return,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let manifest_path = path.join("manifest.toml");
            if !manifest_path.exists() {
                continue;
            }
            let content = match std::fs::read_to_string(&manifest_path) {
                Ok(c) => c,
                Err(_) => continue,
            };
            let manifest: Manifest = match toml::from_str(&content) {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("[mimic] Bad manifest at {:?}: {}", manifest_path, e);
                    continue;
                }
            };

            // Persist enabled state from saved manifest
            let mut entry = ExtensionEntry {
                manifest: manifest.clone(),
                dir: path.clone(),
                runtime: None,
            };
            entry.load_runtime(self.vault_root.as_deref());
            self.extensions.push(entry);
        }
    }

    pub fn enable(&mut self, idx: usize) {
        let vault_root = self.vault_root.clone();
        if let Some(entry) = self.extensions.get_mut(idx) {
            entry.manifest.enabled = true;
            entry.load_runtime(vault_root.as_deref());
            self.save_manifest(idx);
        }
    }

    pub fn disable(&mut self, idx: usize) {
        if let Some(entry) = self.extensions.get_mut(idx) {
            entry.manifest.enabled = false;
            entry.runtime = None;
            self.save_manifest(idx);
        }
    }

    fn save_manifest(&self, idx: usize) {
        if let Some(entry) = self.extensions.get(idx) {
            let manifest_path = entry.dir.join("manifest.toml");
            if let Ok(content) = toml::to_string_pretty(&entry.manifest) {
                let _ = std::fs::write(manifest_path, content);
            }
        }
    }

    /// Install an extension from a source directory (copy to extensions dir)
    /// Returns the manifest on success so the caller can show a warning dialog
    pub fn read_manifest_from(source: &PathBuf) -> Result<Manifest, String> {
        let manifest_path = source.join("manifest.toml");
        if !manifest_path.exists() {
            return Err(format!("No manifest.toml found in {:?}", source));
        }
        let content = std::fs::read_to_string(&manifest_path)
            .map_err(|e| e.to_string())?;
        toml::from_str(&content).map_err(|e| e.to_string())
    }

    pub fn install_from(&mut self, source: &PathBuf) -> Result<String, String> {
        let manifest = Self::read_manifest_from(source)?;
        let name = manifest.name.clone();
        let dest = self.extensions_dir.join(&name);

        // Copy directory
        copy_dir_all(source, &dest).map_err(|e| e.to_string())?;

        // Force disabled on first install — user must explicitly enable
        let manifest_path = dest.join("manifest.toml");
        let content = std::fs::read_to_string(&manifest_path).map_err(|e| e.to_string())?;
        let mut m: Manifest = toml::from_str(&content).map_err(|e| e.to_string())?;
        m.enabled = false;
        let new_content = toml::to_string_pretty(&m).map_err(|e| e.to_string())?;
        std::fs::write(manifest_path, new_content).map_err(|e| e.to_string())?;

        self.load_all();
        Ok(name)
    }

    pub fn remove(&mut self, name: &str) -> Result<(), String> {
        let dest = self.extensions_dir.join(name);
        if !dest.exists() {
            return Err(format!("Extension '{}' not found", name));
        }
        std::fs::remove_dir_all(&dest).map_err(|e| e.to_string())?;
        self.extensions.retain(|e| e.manifest.name != name);
        Ok(())
    }

    pub fn fire_hook(&mut self, event: &HookEvent) -> Vec<HookResult> {
        let perm = event.required_permission();
        let mut results = Vec::new();
        for entry in &mut self.extensions {
            if !entry.manifest.enabled {
                continue;
            }
            if !entry.manifest.has_permission(perm) {
                continue;
            }
            if let Some(rt) = &mut entry.runtime {
                results.push(rt.call_hook(event));
            }
        }
        results
    }

    /// Try to dispatch a command to any enabled extension.
    /// Returns the first non-None result.
    pub fn dispatch_command(&mut self, name: &str, args: &[String]) -> Option<String> {
        for entry in &mut self.extensions {
            if !entry.manifest.enabled {
                continue;
            }
            if !entry.manifest.has_permission("commands") {
                continue;
            }
            if let Some(rt) = &mut entry.runtime {
                if let Some(result) = rt.dispatch_command(name, args) {
                    return Some(result);
                }
            }
        }
        None
    }

    /// Drain all pending notifications from all active runtimes
    pub fn drain_notifications(&mut self) -> Vec<String> {
        let mut all = Vec::new();
        for entry in &mut self.extensions {
            if let Some(rt) = &mut entry.runtime {
                all.extend(rt.drain_notifications());
            }
        }
        all
    }

}

impl Default for ExtensionManager {
    fn default() -> Self {
        Self::new()
    }
}

fn copy_dir_all(src: &PathBuf, dst: &PathBuf) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dest_path = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dest_path)?;
        } else {
            std::fs::copy(entry.path(), dest_path)?;
        }
    }
    Ok(())
}
