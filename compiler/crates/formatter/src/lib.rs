//! A formatter for the vinyl language, built on the tree-sitter syntax tree.
//!
//! The crate exposes three levels of entry points:
//! - [`format_source`] / [`format_source_with_config`] format a source string.
//! - [`format_path`] formats a single file in place.
//! - [`format_project`] formats every file in a project or script.

pub mod error;
mod tree;

pub use error::FormatError;
use std::path::{Path, PathBuf};

/// Configuration for the formatter.
pub struct FormatterConfig {
    /// Number of spaces used for one level of indentation.
    pub indent_width: usize,
}

impl Default for FormatterConfig {
    fn default() -> Self {
        FormatterConfig { indent_width: 4 }
    }
}

/// Formats a source string using the default configuration.
pub fn format_source(source: &str) -> Result<String, FormatError> {
    format_source_with_config(source, &FormatterConfig::default())
}

/// Formats a source string with a custom configuration.
pub fn format_source_with_config(
    source: &str,
    config: &FormatterConfig,
) -> Result<String, FormatError> {
    tree::format_source_with_config(source, config)
}

/// Formats a byte range of a source string.
///
/// Range formatting is not yet implemented; the whole source is formatted and
/// the range is ignored. The LSP and CLI format whole documents.
// todo: support formatting a range of the source code, currently just formats the whole source
pub fn format_range(
    source: &str,
    config: &FormatterConfig,
    _range: std::ops::Range<usize>,
) -> Result<String, FormatError> {
    tree::format_range(source, config)
}

/// Formats a single file in place, writing only when the output differs.
pub fn format_path(path: &Path) -> Result<(), Vec<FormatError>> {
    let source = std::fs::read_to_string(path).map_err(|e| vec![FormatError::Io(e)])?;
    let formatted = format_source(&source).map_err(|e| vec![e])?;
    if formatted != source {
        std::fs::write(path, &formatted).map_err(|e| vec![FormatError::Io(e)])?;
    }
    Ok(())
}

/// Recursively collects every `*.vn` file under `dir`.
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

/// Formats every file in a project or script, accumulating errors across files.
///
/// In manifest mode the module list comes from the resolver; in script mode
/// every `*.vn` file under the root is formatted.
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
