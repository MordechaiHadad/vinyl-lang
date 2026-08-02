pub mod error;
mod tree;

pub use error::FormatError;

use std::path::{Path, PathBuf};

pub struct FormatterConfig {
    pub indent_width: usize,
}

impl Default for FormatterConfig {
    fn default() -> Self {
        FormatterConfig { indent_width: 4 }
    }
}

pub fn format_source(source: &str) -> Result<String, FormatError> {
    tree::format_source(source)
}

pub fn format_source_with_config(
    source: &str,
    config: &FormatterConfig,
) -> Result<String, FormatError> {
    tree::format_source_with_config(source, config)
}

pub fn format_range(
    source: &str,
    config: &FormatterConfig,
    range: std::ops::Range<usize>,
) -> Result<String, FormatError> {
    tree::format_range(source, config, range)
}

pub fn format_path(path: &Path) -> Result<(), Vec<FormatError>> {
    let source = std::fs::read_to_string(path).map_err(|e| vec![FormatError::Io(e)])?;
    let formatted = format_source(&source).map_err(|e| vec![e])?;
    if formatted != source {
        std::fs::write(path, &formatted).map_err(|e| vec![FormatError::Io(e)])?;
    }
    Ok(())
}

fn collect_vn_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(collect_vn_files(&path));
            } else if path.extension().is_some_and(|e| e == "vn") {
                files.push(path);
            }
        }
    }
    files
}

pub fn format_project(source_root: &Path) -> Result<(), Vec<FormatError>> {
    let source_root = source_root
        .canonicalize()
        .map_err(|e| vec![FormatError::Io(e)])?;
    let resolver = vinyl_resolver::resolver::Resolver::detect(&source_root)
        .map_err(|e| vec![FormatError::Resolve(e)])?;
    let files: Vec<PathBuf> = match resolver.mode() {
        vinyl_resolver::resolver::ResolverMode::Manifest => resolver
            .all_modules()
            .values()
            .map(|info| info.file_path.clone())
            .collect(),
        vinyl_resolver::resolver::ResolverMode::Script => collect_vn_files(resolver.root()),
    };
    let mut errors = Vec::new();
    for path in &files {
        if let Err(e) = format_path(path) {
            errors.extend(e);
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}
