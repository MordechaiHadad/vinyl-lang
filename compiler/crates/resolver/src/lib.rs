use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub mod error;
pub mod module_graph;
pub mod resolver;
pub mod structs;
pub mod traits;
use crate::resolver::{ImportPrefix, ModuleInfo};
pub use error::ResolveDiagnostic;

/// Strips the Windows `\\?\` verbatim prefix so that canonicalized paths
/// (`std::fs::canonicalize` always adds it on Windows) compare equal to the
/// plain paths produced by `read_dir`/`std::path::absolute`. Filesystem
/// operations re-add the prefix internally, so plain paths are safe to store.
fn strip_verbatim_prefix(path: &Path) -> PathBuf {
    if let Some(s) = path.to_str()
        && let Some(stripped) = s.strip_prefix(r"\\?\")
    {
        return PathBuf::from(stripped);
    }
    path.to_path_buf()
}

fn find_manifest_dir(start: &Path) -> Option<PathBuf> {
    let mut dir = Some(start.to_path_buf());
    while let Some(ref current) = dir {
        if current.join("vinyl.toml").is_file() {
            return Some(current.clone());
        }
        dir = current.parent().map(|p| p.to_path_buf());
    }
    None
}

fn compute_target_path(prefix: &ImportPrefix, path: &[&str], from: &Path) -> PathBuf {
    let mut base = strip_verbatim_prefix(from.parent().unwrap_or(Path::new("")));

    if let ImportPrefix::Parent(n) = prefix {
        for _ in 0..*n {
            base.push("..");
        }
    }

    for segment in path {
        base.push(segment);
    }
    base.set_extension("vn");
    base
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                result.pop();
            }
            other => {
                result.push(other.as_os_str());
            }
        }
    }
    result
}

fn path_from_relative(relative: &Path) -> Vec<String> {
    relative
        .iter()
        .map(|s| {
            let s = s.to_string_lossy().to_string();
            if let Some(stem) = s.rsplit_once('.') {
                stem.0.to_string()
            } else {
                s
            }
        })
        .collect()
}

fn add_module_path(
    file_path: &Path,
    source_root: &Path,
    modules: &mut HashMap<Vec<String>, ModuleInfo>,
) {
    if file_path.extension().is_none_or(|e| e != "vn") {
        return;
    }

    let file_stem = file_path.file_stem().unwrap().to_string_lossy().to_string();
    let relative = file_path.strip_prefix(source_root).unwrap_or(file_path);
    let mut parts = path_from_relative(relative);

    if parts.len() >= 2 && parts.last() == parts.get(parts.len() - 2) {
        parts.pop();
    }

    let parent_dir_name = file_path
        .parent()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let is_dir_module = file_stem == parent_dir_name;
    let import_name = parts.last().cloned().unwrap_or(file_stem);

    let info = ModuleInfo {
        path: parts.clone(),
        file_path: file_path.to_path_buf(),
        import_name,
    };

    if is_dir_module {
        modules.entry(parts).or_insert(info);
    } else {
        modules.insert(parts, info);
    }
}
