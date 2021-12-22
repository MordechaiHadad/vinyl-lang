use crate::parser::ast::Span;

pub enum ParserError {
    NonSupportedPrimitives(NonSupportedPrimitives)
}

pub struct NonSupportedPrimitives {
    pub error_message: &'static str,
    //pub span: Span
}