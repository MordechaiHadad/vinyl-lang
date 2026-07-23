use core::fmt;
use std::error::Error;

use miette::{Diagnostic, NamedSource, SourceSpan};

#[derive(Debug, Diagnostic)]
#[diagnostic()]
pub struct LowerError {
    pub message: String,
    #[source_code]
    pub source: NamedSource<String>,
    #[label]
    pub span: SourceSpan,
}

impl fmt::Display for LowerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl Error for LowerError {}
