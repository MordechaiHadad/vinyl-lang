use miette::Diagnostic;
use std::error::Error;
use std::fmt;
use tracing::instrument;
use vinyl_parser::lower::LowerError;
use vinyl_typecheck::TypeError;

#[derive(Debug, Diagnostic)]
#[diagnostic()]
pub enum CompileError {
    Parse(#[diagnostic(transparent)] vinyl_parser::ParseError),
    Lower(#[diagnostic(transparent)] LowerError),
    TypeError(#[diagnostic(transparent)] TypeError),
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompileError::Parse(e) => fmt::Display::fmt(e, f),
            CompileError::Lower(e) => fmt::Display::fmt(e, f),
            CompileError::TypeError(e) => fmt::Display::fmt(e, f),
        }
    }
}

impl Error for CompileError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            CompileError::Parse(e) => Some(e),
            CompileError::Lower(e) => Some(e),
            CompileError::TypeError(e) => Some(e),
        }
    }
}

impl From<vinyl_parser::ParseError> for CompileError {
    fn from(e: vinyl_parser::ParseError) -> Self {
        CompileError::Parse(e)
    }
}

impl From<LowerError> for CompileError {
    fn from(e: LowerError) -> Self {
        CompileError::Lower(e)
    }
}

impl From<TypeError> for CompileError {
    fn from(e: TypeError) -> Self {
        CompileError::TypeError(e)
    }
}

#[instrument(skip_all)]
pub fn compile(
    source: &str,
    source_name: &str,
) -> Result<Vec<vinyl_typecheck::hir::HirItem>, Vec<CompileError>> {
    let tree = match vinyl_parser::parse(source) {
        Ok(t) => t,
        Err(errors) => return Err(errors.into_iter().map(CompileError::Parse).collect()),
    };

    let items = vinyl_parser::lower::lower(&tree, source, source_name).map_err(|errors| {
        errors
            .into_iter()
            .map(CompileError::Lower)
            .collect::<Vec<_>>()
    })?;

    vinyl_typecheck::typeck(&items, source, source_name).map_err(|errors| {
        errors
            .into_iter()
            .map(CompileError::TypeError)
            .collect::<Vec<_>>()
    })
}
