use miette::Diagnostic;
use thiserror::Error;

#[derive(Debug, Error, Diagnostic)]
pub enum CraneliftError {
    /// A code generation or runtime failure with contextual text.
    #[error("{0}")]
    #[diagnostic(code(codegen::cranelift_error))]
    Msg(String),
}
