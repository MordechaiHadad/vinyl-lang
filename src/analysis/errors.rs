use crate::parser::ast::{Expression, Span, Type};
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
    pub expected_type: Type,
    pub found_expression: Expression,
    pub span: Span,
}
