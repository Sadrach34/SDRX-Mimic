use std::collections::HashSet;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub path: PathBuf,
    pub name: String,
    pub depth: usize,
    pub is_dir: bool,
}

#[derive(Debug, Clone, Default)]
pub struct FileTree {
    pub entries: Vec<FileEntry>,
    pub selected: usize,
    pub vault_root: PathBuf,
    pub collapsed: HashSet<PathBuf>,
}

impl FileTree {
    pub fn load(vault_root: &Path) -> Self {
        let entries = WalkDir::new(vault_root)
            .sort_by_file_name()
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| {
                let p = e.path();
                // skip hidden dirs like .obsidian, .git
                !p.components().any(|c| {
                    c.as_os_str().to_str().map(|s| s.starts_with('.')).unwrap_or(false)
                })
            })
            .filter(|e| {
                e.file_type().is_dir()
                    || e.path().extension().map(|x| x == "md").unwrap_or(false)
            })
            .skip(1) // skip root itself
            .map(|e| {
                let depth = e.depth().saturating_sub(1);
                let name = e.file_name().to_string_lossy().to_string();
                FileEntry {
                    path: e.path().to_path_buf(),
                    name,
                    depth,
                    is_dir: e.file_type().is_dir(),
                }
            })
            .collect();

        Self {
            entries,
            selected: 0,
            vault_root: vault_root.to_path_buf(),
            collapsed: HashSet::new(),
        }
    }

    pub fn selected_entry(&self) -> Option<&FileEntry> {
        self.entries.get(self.selected)
    }

    pub fn visible_indices(&self) -> Vec<usize> {
        let mut vis = Vec::new();
        let mut skip_depth: Option<usize> = None;
        for (i, entry) in self.entries.iter().enumerate() {
            if let Some(d) = skip_depth {
                if entry.depth > d {
                    continue;
                } else {
                    skip_depth = None;
                }
            }
            vis.push(i);
            if entry.is_dir && self.collapsed.contains(&entry.path) {
                skip_depth = Some(entry.depth);
            }
        }
        vis
    }

    pub fn toggle_dir(&mut self) {
        if let Some(e) = self.entries.get(self.selected) {
            if e.is_dir {
                if !self.collapsed.remove(&e.path) {
                    self.collapsed.insert(e.path.clone());
                }
            }
        }
    }

    pub fn move_down(&mut self) {
        let vis = self.visible_indices();
        if let Some(pos) = vis.iter().position(|&i| i == self.selected) {
            if pos + 1 < vis.len() {
                self.selected = vis[pos + 1];
            }
        }
    }

    pub fn move_up(&mut self) {
        let vis = self.visible_indices();
        if let Some(pos) = vis.iter().position(|&i| i == self.selected) {
            if pos > 0 {
                self.selected = vis[pos - 1];
            }
        }
    }

    pub fn refresh(&mut self) {
        let root = self.vault_root.clone();
        let prev_selected = self.selected;
        let prev_collapsed = std::mem::take(&mut self.collapsed);
        *self = Self::load(&root);
        self.collapsed = prev_collapsed;
        self.selected = prev_selected.min(self.entries.len().saturating_sub(1));
    }
}
