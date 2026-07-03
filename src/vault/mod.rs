pub mod links;
pub mod note;
pub mod tree;

pub use note::Note;
pub use tree::FileTree;

use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Vault {
    pub root: PathBuf,
    pub tree: FileTree,
}

impl Vault {
    pub fn open(root: &Path) -> std::io::Result<Self> {
        std::fs::create_dir_all(root)?;
        let tree = FileTree::load(root);
        Ok(Self {
            root: root.to_path_buf(),
            tree,
        })
    }

    pub fn refresh(&mut self) {
        self.tree.refresh();
    }
}
