use miette::SourceSpan;

use crate::ast::{operator::{BinaryOp, UnaryOp}, pattern::Pattern, statement::Statement};

#[derive(Debug)]
pub enum Expression {
    Int(i128, SourceSpan),
    Float(f64, SourceSpan),
    String(String, SourceSpan),
    Char(char, SourceSpan),
    Bool(bool, SourceSpan),
    Unit(SourceSpan),
    Ident(String, SourceSpan),
    Binary {
        span: SourceSpan,
        left: Box<Expression>,
        op: BinaryOp,
        right: Box<Expression>,
    },
    Unary {
        span: SourceSpan,
        op: UnaryOp,
        operand: Box<Expression>,
    },
    Call {
        span: SourceSpan,
        function: Box<Expression>,
        args: Vec<Expression>,
    },
    Block(Vec<Statement>, SourceSpan),
    Match {
        span: SourceSpan,
        value: Box<Expression>,
        arms: Vec<MatchArm>,
    },
    Field {
        span: SourceSpan,
        object: Box<Expression>,
        name: String,
    },
    Index {
        span: SourceSpan,
        array: Box<Expression>,
        index: Box<Expression>,
    },
    Tuple(Vec<Expression>, SourceSpan),
    Array(Vec<Expression>, SourceSpan),
    EnumVariant {
        span: SourceSpan,
        type_name: String,
        variant_name: String,
        args: Vec<Expression>,
    },
    Paren(Box<Expression>, SourceSpan),
    Ref {
        span: SourceSpan,
        operand: Box<Expression>,
    },
    If {
        span: SourceSpan,
        condition: Box<Expression>,
        then_block: Vec<Statement>,
        else_if: Vec<(Expression, Vec<Statement>)>,
        else_block: Option<Vec<Statement>>,
    },
}

impl Expression {
    pub fn span(&self) -> SourceSpan {
        match self {
            Expression::Int(_, s) => *s,
            Expression::Float(_, s) => *s,
            Expression::String(_, s) => *s,
            Expression::Char(_, s) => *s,
            Expression::Bool(_, s) => *s,
            Expression::Unit(s) => *s,
            Expression::Ident(_, s) => *s,
            Expression::Binary { span, .. } => *span,
            Expression::Unary { span, .. } => *span,
            Expression::Call { span, .. } => *span,
            Expression::Block(_, s) => *s,
            Expression::Match { span, .. } => *span,
            Expression::Field { span, .. } => *span,
            Expression::Index { span, .. } => *span,
            Expression::Tuple(_, s) => *s,
            Expression::Array(_, s) => *s,
            Expression::Paren(_, s) => *s,
            Expression::Ref { span, .. } => *span,
            Expression::If { span, .. } => *span,
            Expression::EnumVariant { span, .. } => *span,
        }
    }
}

#[derive(Debug)]
pub struct MatchArm {
    pub span: SourceSpan,
    pub pattern: Pattern,
    pub body: Box<Expression>,
}
