use crate::parser::ast::Span;

#[derive(Debug)]
pub enum ParserError {
    NonSupportedPrimitives(NonSupportedPrimitives),
}

#[derive(Debug)]
pub struct NonSupportedPrimitives {
    pub error_message: &'static str,
    pub span: Span,
}
