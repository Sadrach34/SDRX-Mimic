use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Note {
    pub path: PathBuf,
    pub content: String,
    pub dirty: bool,
}

impl Note {
    pub fn load(path: &Path) -> std::io::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Ok(Self {
            path: path.to_path_buf(),
            content,
            dirty: false,
        })
    }

    pub fn save(&mut self) -> std::io::Result<()> {
        std::fs::write(&self.path, &self.content)?;
        self.dirty = false;
        Ok(())
    }

    pub fn name(&self) -> &str {
        self.path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("untitled.md")
    }

    pub fn stem(&self) -> &str {
        self.path
            .file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or("untitled")
    }

    pub fn create(path: &Path) -> std::io::Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let title = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Nueva nota");
        let content = format!("# {}\n\n", title);
        std::fs::write(path, &content)?;
        Ok(Self {
            path: path.to_path_buf(),
            content,
            dirty: false,
        })
    }
}
