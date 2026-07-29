use miette::Diagnostic;
use thiserror::Error;

#[derive(Debug, Error, Diagnostic)]
pub enum FormatError {
    #[error(transparent)]
    #[diagnostic(transparent)]
    Resolve(#[from] vinyl_resolver::ResolveDiagnostic),

    #[error(transparent)]
    #[diagnostic(transparent)]
    Parse(Box<vinyl_parser::ParserDiagnostic>),

    #[error("io error: {0}")]
    #[diagnostic(code(formatter::io_error))]
    Io(#[from] std::io::Error),
}

impl From<vinyl_parser::ParserDiagnostic> for FormatError {
    fn from(e: vinyl_parser::ParserDiagnostic) -> Self {
        FormatError::Parse(Box::new(e))
    }
}
