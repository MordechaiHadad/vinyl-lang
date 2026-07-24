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
        name: String,
        mutable: bool,
        type_: Type,
        value: HirExpression,
    },
    Expr(HirExpression),
    Return(Option<HirExpression>),
    Value(HirExpression),
    Loop {
        body: Vec<HirStatement>,
    },
    Break,
    Continue,
    Assign {
        target: HirAssignTarget,
        op: AssignOp,
        value: HirExpression,
    },
}

#[derive(Debug, Clone)]
pub enum HirAssignTarget {
    Ident(String),
    Index {
        array: Box<HirExpression>,
        index: Box<HirExpression>,
    },
    Field {
        object: Box<HirExpression>,
        name: String,
    },
    Deref(Box<HirExpression>),
}
