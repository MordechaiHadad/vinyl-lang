use std::{collections::HashMap, path::{Path, PathBuf}};

#[derive(Default, Clone)]
pub struct Vfs {
    files: HashMap<PathBuf, String>,
}

impl Vfs {
    pub fn set(&mut self, path: PathBuf, source: String) {
        self.files.insert(path, source);
    }

    pub fn remove(&mut self, path: &Path) {
        self.files.remove(path);
    }

    pub fn source(&self, path: &Path) -> Option<String> {
        self.files
            .get(path)
            .cloned()
            .or_else(|| std::fs::read_to_string(path).ok())
    }
}
