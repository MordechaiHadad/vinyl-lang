use miette::Diagnostic;
use thiserror::Error;

#[derive(Debug, Error, Diagnostic)]
pub enum CompilerDiagnostic {
    #[error(transparent)]
    #[diagnostic(transparent)]
    Parse(#[from] vinyl_parser::ParserDiagnostic),

    #[error(transparent)]
    #[diagnostic(transparent)]
    Type(#[from] vinyl_typecheck::TypeDiagnostic),

    #[error(transparent)]
    #[diagnostic(transparent)]
    Resolve(#[from] vinyl_resolver::ResolveDiagnostic),
}
