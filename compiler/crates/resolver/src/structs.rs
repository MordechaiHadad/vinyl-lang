use std::path::{Path, PathBuf};

use crate::{ResolveDiagnostic, traits::FileSystem};

#[derive(Debug)]
pub struct DiskFileSystem;

impl FileSystem for DiskFileSystem {
    fn file_exists(&self, path: &Path) -> bool {
        path.is_file()
    }

    fn collect_vn_files(&self, dir: &Path) -> Result<Vec<PathBuf>, ResolveDiagnostic> {
        let mut files = Vec::new();
        let walker = ignore::WalkBuilder::new(dir)
            .parents(true)
            .git_ignore(true)
            .git_global(true)
            .require_git(false)
            .build();
        for result in walker {
            let entry = result.map_err(|e| ResolveDiagnostic::Io(std::io::Error::other(e)))?;
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "vn") {
                files.push(path.to_path_buf());
            }
        }
        files.sort();
        Ok(files)
    }
}
