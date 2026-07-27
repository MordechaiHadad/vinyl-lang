use miette::{Diagnostic, NamedSource, SourceSpan};
use std::error::Error;
use std::fmt;

vinyl_diagnostics::diagnostic_codes! {
    "type",
    pub enum TypeDiagnosticKind {
        Message,
        Mismatch,
    }
}

#[derive(Debug)]
pub enum TypeErrorKind {
    Message(String),
    Mismatch {
        expected: crate::hir::Type,
        found: crate::hir::Type,
    },
}

impl TypeErrorKind {
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Message(_) => TypeDiagnosticKind::Message.code().variant,
            Self::Mismatch { .. } => TypeDiagnosticKind::Mismatch.code().variant,
        }
    }
}

impl TypeError {
    pub fn diagnostic_code(&self) -> &'static str {
        self.kind.code()
    }
}

impl fmt::Display for TypeErrorKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code())
    }
}

#[derive(Debug, Diagnostic)]
#[diagnostic()]
pub struct TypeError {
    #[diagnostic(skip)]
    pub kind: TypeErrorKind,
    #[source_code]
    pub source: NamedSource<String>,
    #[label]
    pub span: SourceSpan,
}

impl fmt::Display for TypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            TypeErrorKind::Message(message) => f.write_str(message),
            TypeErrorKind::Mismatch { expected, found } => {
                write!(f, "type mismatch: expected `{expected}`, found `{found}`")
            }
        }
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
