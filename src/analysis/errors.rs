use crate::parser::ast::{Expression, Span, Type, Variable};
use lasso::Spur;

#[derive(Debug)]
pub enum Error {
    NullReferenceError(NullReferenceError),
    TypeMismatchError(TypeMismatchError),
}

#[derive(Debug)]
pub struct NullReferenceError {
    pub value: Spur,
    pub span: Span,
}

#[derive(Debug)]
pub struct TypeMismatchError {
    pub variable: Variable,
}
