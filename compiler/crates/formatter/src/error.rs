use miette::Diagnostic;
use thiserror::Error;

/// Errors that can occur while formatting vinyl source code.
#[derive(Debug, Error, Diagnostic)]
pub enum FormatError {
    /// The project root could not be resolved into a module tree.
    #[error(transparent)]
    #[diagnostic(transparent)]
    Resolve(#[from] vinyl_resolver::error::ResolveDiagnostic),

    /// The source failed to parse.
    #[error(transparent)]
    #[diagnostic(transparent)]
    Parse(Box<vinyl_parser::ParserDiagnostic>),

    /// Reading or writing a source file failed.
    #[error("io error: {0}")]
    #[diagnostic(code(formatter::io_error))]
    Io(#[from] std::io::Error),
}

impl From<vinyl_parser::ParserDiagnostic> for FormatError {
    fn from(e: vinyl_parser::ParserDiagnostic) -> Self {
        FormatError::Parse(Box::new(e))
    }
}
