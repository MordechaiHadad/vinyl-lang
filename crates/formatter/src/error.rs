use std::fmt;

#[derive(Debug)]
pub enum FormatError {
    Resolve(vinyl_resolver::ResolveError),
    Parse(Box<vinyl_parser::ParseError>),
    Io(std::io::Error),
}

impl fmt::Display for FormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FormatError::Resolve(e) => write!(f, "{e}"),
            FormatError::Parse(e) => write!(f, "{e}"),
            FormatError::Io(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for FormatError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FormatError::Resolve(e) => Some(e),
            FormatError::Parse(e) => Some(e),
            FormatError::Io(e) => Some(e),
        }
    }
}

impl From<vinyl_resolver::ResolveError> for FormatError {
    fn from(e: vinyl_resolver::ResolveError) -> Self {
        FormatError::Resolve(e)
    }
}

impl From<vinyl_parser::ParseError> for FormatError {
    fn from(e: vinyl_parser::ParseError) -> Self {
        FormatError::Parse(Box::new(e))
    }
}

impl From<std::io::Error> for FormatError {
    fn from(e: std::io::Error) -> Self {
        FormatError::Io(e)
    }
}
