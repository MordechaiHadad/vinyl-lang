use miette::SourceSpan;
use vinyl_parser::ast::operator::{BinaryOp, UnaryOp};

use crate::hir::statement::HirStatement;
use crate::hir::types::Type;

#[derive(Debug, Clone)]
pub struct HirExpression {
    pub kind: HirExpressionKind,
    pub type_: Type,
}

#[derive(Debug, Clone)]
pub enum HirExpressionKind {
    Int(i128, SourceSpan),
    Float(f64, SourceSpan),
    String(String),
    Bool(bool),
    Unit,
    Char(char),
    Ident(String),
    Binary {
        left: Box<HirExpression>,
        op: BinaryOp,
        right: Box<HirExpression>,
    },
    Unary {
        op: UnaryOp,
        operand: Box<HirExpression>,
    },
    Call {
        function: Box<HirExpression>,
        args: Vec<HirExpression>,
    },
    Block(Vec<HirStatement>),
    Index {
        span: SourceSpan,
        array: Box<HirExpression>,
        index: Box<HirExpression>,
    },
    Array(Vec<HirExpression>),
    If {
        condition: Box<HirExpression>,
        then_block: Vec<HirStatement>,
        else_if: Vec<(HirExpression, Vec<HirStatement>)>,
        else_block: Option<Vec<HirStatement>>,
    },
    Ref(Box<HirExpression>),
    EnumVariant {
        type_name: String,
        variant_index: usize,
        payload: Vec<HirExpression>,
    },
    Tuple(Vec<HirExpression>, SourceSpan),
    FieldAccess {
        span: SourceSpan,
        object: Box<HirExpression>,
        name: String,
    },
}
