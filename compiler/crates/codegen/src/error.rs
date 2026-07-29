use miette::Diagnostic;
use thiserror::Error;

#[derive(Debug, Error, Diagnostic)]
pub enum CraneliftError {
    #[error("{0}")]
    #[diagnostic(code(codegen::cranelift_error))]
    Msg(String),
}
