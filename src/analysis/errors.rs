use lasso::Spur;
use crate::parser::ast::Span;

pub enum Error {
    NullReferenceError(NullReferenceError)
}

pub struct NullReferenceError {
    pub value: Spur,
    pub span: Span,
}