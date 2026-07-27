use miette::{Diagnostic, NamedSource, SourceSpan};
use thiserror::Error;

vinyl_diagnostics::diagnostic_codes! {
    "type",
    pub enum TypeDiagnosticKind {
        Message,
        Mismatch,
    }
}

#[derive(Debug, Error)]
pub enum TypeErrorKind {
    #[error("{0}")]
    Message(String),
    #[error("type mismatch: expected `{expected}`, found `{found}`")]
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

#[derive(Debug, Error, Diagnostic)]
#[error("{kind}")]
#[diagnostic()]
pub struct TypeError {
    #[diagnostic(skip)]
    pub kind: TypeErrorKind,
    #[source_code]
    pub source_code: NamedSource<String>,
    #[label]
    pub span: SourceSpan,
}

#[derive(Debug, Error, Diagnostic)]
#[error("{message}")]
#[diagnostic(severity(Warning))]
pub struct CompileWarning {
    pub message: String,
    #[source_code]
    pub source_code: NamedSource<String>,
    #[label]
    pub span: SourceSpan,
}


#[derive(Debug, Error, Diagnostic)]
#[error("{kind}")]
pub struct TypecheckDiagnostic {
    #[diagnostic(transparent)]
    pub kind: TypecheckDiagnosticKind,

    #[source_code]
    pub source_code: NamedSource<String>,

    #[label]
    pub span: SourceSpan,
}

#[derive(Debug, Error, Diagnostic)]
pub enum TypecheckDiagnosticKind {
    #[error("{0}")]
    #[diagnostic(code(typeck::message))]
    Message(String),

    #[error("type mismatch: expected `{expected}`, found `{found}`")]
    #[diagnostic(code(typeck::mismatch))]
    Mismatch {
        expected: crate::hir::Type,
        found: crate::hir::Type,
    },

    #[error("{message}")]
    #[diagnostic(code(typeck::warning), severity(warning))]
    Warning { message: String },
}
