use miette::Diagnostic;
use std::error::Error;
use std::fmt;
use vinyl_parser::lower::LowerError;

#[derive(Debug, Diagnostic)]
#[diagnostic()]
pub enum CompileError {
    Parse(#[diagnostic(transparent)] vinyl_parser::ParseError),
    Lower(#[diagnostic(transparent)] LowerError),
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompileError::Parse(e) => fmt::Display::fmt(e, f),
            CompileError::Lower(e) => fmt::Display::fmt(e, f),
        }
    }
}

impl Error for CompileError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            CompileError::Parse(e) => Some(e),
            CompileError::Lower(e) => Some(e),
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

pub fn compile(source: &str) -> Result<Vec<vinyl_parser::ast::Item>, Vec<CompileError>> {
    let tree = match vinyl_parser::parse(source) {
        Ok(t) => t,
        Err(errors) => return Err(errors.into_iter().map(CompileError::Parse).collect()),
    };

    vinyl_parser::lower::lower(&tree, source).map_err(|errors| errors.into_iter().map(CompileError::Lower).collect())
}
