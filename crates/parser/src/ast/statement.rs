use miette::SourceSpan;

use crate::ast::{expression::Expression, operator::AssignOp, types::Type};

#[derive(Debug)]
pub enum Statement {
    Let {
        span: SourceSpan,
        name: String,
        mutable: bool,
        type_: Option<Type>,
        value: Expression,
    },
    Expression(Expression),
    Return(Option<Expression>, SourceSpan),
    Value(Expression, SourceSpan),
    If {
        span: SourceSpan,
        condition: Expression,
        then_block: Vec<Statement>,
        else_if: Vec<(Expression, Vec<Statement>)>,
        else_block: Option<Vec<Statement>>,
    },
    While {
        span: SourceSpan,
        condition: Expression,
        body: Vec<Statement>,
    },
    Loop {
        span: SourceSpan,
        body: Vec<Statement>,
    },
    Break(SourceSpan),
    Continue(SourceSpan),
    Assign {
        span: SourceSpan,
        target: AssignTarget,
        op: AssignOp,
        value: Box<Expression>,
    },
}

impl Statement {
    pub fn span(&self) -> SourceSpan {
        match self {
            Statement::Let { span, .. } => *span,
            Statement::Expression(e) => e.span(),
            Statement::Return(_, span) => *span,
            Statement::Value(_, span) => *span,
            Statement::If { span, .. } => *span,
            Statement::While { span, .. } => *span,
            Statement::Loop { span, .. } => *span,
            Statement::Break(span) => *span,
            Statement::Continue(span) => *span,
            Statement::Assign { span, .. } => *span,
        }
    }
}

#[derive(Debug)]
pub enum AssignTarget {
    Ident(String, SourceSpan),
    Index {
        span: SourceSpan,
        array: Box<Expression>,
        index: Box<Expression>,
    },
    Field {
        span: SourceSpan,
        object: Box<Expression>,
        name: String,
    },
}
