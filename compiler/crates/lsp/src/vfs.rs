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
        if let Some(s) = self.files.get(path) {
            return Some(s.clone());
        }
        // Try all normalized forms of the path
        let mut candidates = vec![normalize_vfs_path(path)];
        if let Ok(canon) = path.canonicalize() {
            candidates.push(normalize_vfs_path(&canon));
        }
        // Also try stripping \\?\ prefix on Windows
        if let Some(s) = path.to_str()
            && let Some(stripped) = s.strip_prefix("\\\\?\\")
        {
            candidates.push(PathBuf::from(stripped));
        }
        for candidate in &candidates {
            if let Some(s) = self.files.get(candidate) {
                return Some(s.clone());
            }
        }
        std::fs::read_to_string(path).ok()
    }
}

fn normalize_vfs_path(path: &Path) -> PathBuf {
    if let Some(s) = path.to_str()
        && let Some(stripped) = s.strip_prefix("\\\\?\\")
    {
        return PathBuf::from(stripped);
    }
    path.to_path_buf()
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
