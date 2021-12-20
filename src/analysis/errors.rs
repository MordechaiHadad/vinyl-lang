use lasso::Spur;
use crate::parser::ast::{Literal, Span, Type};

pub enum Error {
    NullReferenceError(NullReferenceError),
    TypeMismatchError(TypeMismatchError)
}

pub struct NullReferenceError {
    pub value: Spur,
    pub span: Span,
}

pub struct TypeMismatchError {
    pub expected_type: Type,
    pub found_type: Literal,
    pub span: Span,
}