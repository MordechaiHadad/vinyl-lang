use std::path::PathBuf;

use miette::Diagnostic;
use thiserror::Error;

#[derive(Debug, Error, Diagnostic)]
pub enum ResolveDiagnostic {
    #[error("module not found: `{import_path:?}`, searched: {searched:?}")]
    #[diagnostic(code(resolver::not_found))]
    NotFound {
        import_path: Vec<String>,
        searched: Vec<PathBuf>,
    },

    #[error("io error: {0}")]
    #[diagnostic(code(resolver::io_error))]
    Io(#[from] std::io::Error),

    #[error("manifest mode requires a `src/` directory, not found at `{root}`")]
    #[diagnostic(code(resolver::missing_src_dir))]
    MissingSrcDir { root: PathBuf },

    #[error("prefix `{prefix}` is not allowed in {mode} mode")]
    #[diagnostic(code(resolver::invalid_prefix))]
    InvalidPrefix { prefix: String, mode: String },

    #[error("import `{import_path:?}` resolves above the project root")]
    #[diagnostic(code(resolver::above_root))]
    AboveRoot { import_path: Vec<String> },
}
