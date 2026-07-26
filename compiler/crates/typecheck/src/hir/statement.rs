use miette::SourceSpan;

use crate::hir::expression::HirExpression;
use crate::hir::operator::AssignOp;
use crate::hir::types::Type;

#[derive(Debug, Clone)]
pub struct HirStatement {
    pub kind: HirStatementKind,
}

#[derive(Debug, Clone)]
pub enum HirStatementKind {
    Let {
        span: SourceSpan,
        name: String,
        mutable: bool,
        type_: Type,
        value: HirExpression,
    },
    Expr(HirExpression, SourceSpan),
    Return(Option<HirExpression>, SourceSpan),
    Value(HirExpression, SourceSpan),
    Loop {
        span: SourceSpan,
        body: Vec<HirStatement>,
    },
    Break(SourceSpan),
    Continue(SourceSpan),
    Assign {
        span: SourceSpan,
        target: HirAssignTarget,
        op: AssignOp,
        value: HirExpression,
    },
}

#[derive(Debug, Clone)]
pub enum HirAssignTarget {
    Ident(String, SourceSpan),
    Index {
        span: SourceSpan,
        array: Box<HirExpression>,
        index: Box<HirExpression>,
    },
    Field {
        span: SourceSpan,
        object: Box<HirExpression>,
        name: String,
    },
    Deref(Box<HirExpression>, SourceSpan),
}
