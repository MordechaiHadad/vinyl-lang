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
    String(String, SourceSpan),
    Bool(bool, SourceSpan),
    Unit(SourceSpan),
    Char(char, SourceSpan),
    Ident(String, SourceSpan),
    Binary {
        span: SourceSpan,
        left: Box<HirExpression>,
        op: BinaryOp,
        right: Box<HirExpression>,
    },
    Unary {
        span: SourceSpan,
        op: UnaryOp,
        operand: Box<HirExpression>,
    },
    Call {
        span: SourceSpan,
        function: Box<HirExpression>,
        args: Vec<HirExpression>,
    },
    Block(Vec<HirStatement>, SourceSpan),
    Index {
        span: SourceSpan,
        array: Box<HirExpression>,
        index: Box<HirExpression>,
    },
    Array(Vec<HirExpression>, SourceSpan),
    If {
        span: SourceSpan,
        condition: Box<HirExpression>,
        then_block: Vec<HirStatement>,
        else_if: Vec<(HirExpression, Vec<HirStatement>)>,
        else_block: Option<Vec<HirStatement>>,
    },
    Ref(Box<HirExpression>, SourceSpan),
    EnumVariant {
        span: SourceSpan,
        type_name: String,
        variant_index: usize,
        payload: Vec<HirExpression>,
    },
    Tuple(Vec<HirExpression>, SourceSpan),
    Struct {
        span: SourceSpan,
        type_name: String,
        fields: Vec<(String, HirExpression)>,
    },
    FieldAccess {
        span: SourceSpan,
        object: Box<HirExpression>,
        name: String,
    },
}
