use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use vinyl_resolver::{DiskFileSystem, FileSystem, ResolveDiagnostic};

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

    pub fn files(&self) -> &HashMap<PathBuf, String> {
        &self.files
    }

    pub fn source(&self, path: &Path) -> Option<String> {
        self.files
            .get(path)
            .cloned()
            .or_else(|| std::fs::read_to_string(path).ok())
    }
}

#[derive(Debug)]
pub struct LspFileSystem {
    vfs: HashMap<PathBuf, String>,
}

impl LspFileSystem {
    pub fn new(vfs: HashMap<PathBuf, String>) -> Self {
        LspFileSystem { vfs }
    }
}

impl FileSystem for LspFileSystem {
    fn file_exists(&self, path: &Path) -> bool {
        self.vfs.contains_key(path) || path.is_file()
    }

    fn collect_vn_files(&self, dir: &Path) -> Result<Vec<PathBuf>, ResolveDiagnostic> {
        let disk_fs = DiskFileSystem;
        let mut files = disk_fs.collect_vn_files(dir)?;
        for vfs_path in self.vfs.keys() {
            if vfs_path.starts_with(dir)
                && vfs_path.extension().is_some_and(|ext| ext == "vn")
                && !files.contains(vfs_path)
            {
                files.push(vfs_path.clone());
            }
        }
        files.sort();
        Ok(files)
    }
}
