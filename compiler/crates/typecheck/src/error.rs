use miette::{Diagnostic, NamedSource, SourceSpan};
use std::error::Error;
use std::fmt;

#[derive(Debug, Diagnostic)]
#[diagnostic()]
pub struct TypeError {
    pub message: String,
    #[source_code]
    pub source: NamedSource<String>,
    #[label]
    pub span: SourceSpan,
}

impl fmt::Display for TypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl Error for TypeError {}

#[derive(Debug, Diagnostic)]
#[diagnostic(severity(Warning))]
pub struct CompileWarning {
    pub message: String,
    #[source_code]
    pub source: NamedSource<String>,
    #[label]
    pub span: SourceSpan,
}

impl fmt::Display for CompileWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl Error for CompileWarning {}
