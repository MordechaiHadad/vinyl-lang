use crate::error::ResolveDiagnostic;
use std::fmt::Debug;
use std::path::{Path, PathBuf};

pub trait FileSystem: Debug + Send + Sync {
    fn file_exists(&self, path: &Path) -> bool;
    fn collect_vn_files(&self, dir: &Path) -> Result<Vec<PathBuf>, ResolveDiagnostic>;
}
