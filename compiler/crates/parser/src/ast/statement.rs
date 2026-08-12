use miette::SourceSpan;

use crate::ast::{expression::Expression, item::ImportDef, operator::AssignOp, types::Type};

/// A statement in a lowered function body.
#[derive(Debug, Clone)]
pub enum Statement {
    /// A local binding declaration.
    Let {
        span: SourceSpan,
        name: String,
        mutable: bool,
        type_: Option<Type>,
        value: Expression,
    },
    /// An expression whose value is discarded.
    Expression(Expression),
    /// An explicit return.
    Return(Option<Expression>, SourceSpan),
    /// The value-producing final expression in a block.
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
    /// A function-local import.
    Import(ImportDef),
}

impl Statement {
    /// Returns the source span covering the statement.
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
            Statement::Import(import) => import.span,
        }
    }
}

#[derive(Debug, Clone)]
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
