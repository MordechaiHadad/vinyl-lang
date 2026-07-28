use std::path::PathBuf;

use miette::Diagnostic;
use thiserror::Error;

#[derive(Debug, Error, Diagnostic)]
pub enum ResolveDiagnostic {
    #[error("module `{import_path:?}` not found, searched: {searched:?}")]
    #[diagnostic(code(resolver::not_found))]
    NotFound {
        import_path: Vec<String>,
        searched: Vec<PathBuf>,
    },

    #[error("io error: {0}")]
    #[diagnostic(code(resolver::io_error))]
    Io(#[from] std::io::Error),
}
