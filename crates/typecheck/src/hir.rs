use miette::SourceSpan;
use vinyl_parser::ast::operator::{BinaryOp, UnaryOp};

#[derive(Debug, Clone)]
pub struct HirItem {
    pub kind: HirItemKind,
}

#[derive(Debug, Clone)]
pub enum HirItemKind {
    Function(HirFunction),
    Struct(HirStruct),
    TupleStruct(HirTupleStruct),
    Enum(HirEnum),
}

#[derive(Debug, Clone)]
pub struct HirFunction {
    pub name: String,
    pub params: Vec<HirParam>,
    pub return_type: Type,
    pub body: Vec<HirStatement>,
}

#[derive(Debug, Clone)]
pub struct HirStruct {
    pub name: String,
    pub repr_c: bool,
    pub fields: Vec<HirField>,
}

#[derive(Debug, Clone)]
pub struct HirTupleStruct {
    pub name: String,
    pub types: Vec<Type>,
}

#[derive(Debug, Clone)]
pub struct HirEnum {
    pub name: String,
    pub variants: Vec<HirEnumVariant>,
}

#[derive(Debug, Clone)]
pub struct HirField {
    pub name: String,
    pub type_: Type,
}

#[derive(Debug, Clone)]
pub struct HirEnumVariant {
    pub name: String,
    pub data: Option<HirEnumVariantData>,
}

#[derive(Debug, Clone)]
pub enum HirEnumVariantData {
    Tuple(Vec<Type>),
    Struct(Vec<HirField>),
}

#[derive(Debug, Clone)]
pub struct HirParam {
    pub name: String,
    pub mutable: bool,
    pub type_: Type,
}

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
        value: HirExpr,
    },
    Expr(HirExpr),
    Return(Option<HirExpr>),
    Value(HirExpr),
    Loop {
        body: Vec<HirStatement>,
    },
    Break,
    Continue,
    Assign {
        target: HirAssignTarget,
        op: AssignOp,
        value: HirExpr,
    },
}

#[derive(Debug, Clone)]
pub enum HirAssignTarget {
    Ident(String),
    Index {
        array: Box<HirExpr>,
        index: Box<HirExpr>,
    },
    Field {
        object: Box<HirExpr>,
        name: String,
    },
    Deref(Box<HirExpr>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum AssignOp {
    Eq,
    AddEq,
    SubEq,
    MulEq,
    DivEq,
    RemEq,
    BitAndEq,
    BitOrEq,
    BitXorEq,
    ShlEq,
    ShrEq,
}

#[derive(Debug, Clone)]
pub struct HirExpr {
    pub kind: HirExprKind,
    pub type_: Type,
}

#[derive(Debug, Clone)]
pub enum HirExprKind {
    Int(i128, SourceSpan),
    Float(f64, SourceSpan),
    String(String),
    Bool(bool),
    Unit,
    Char(char),
    Ident(String),
    Binary {
        left: Box<HirExpr>,
        op: BinaryOp,
        right: Box<HirExpr>,
    },
    Unary {
        op: UnaryOp,
        operand: Box<HirExpr>,
    },
    Call {
        function: Box<HirExpr>,
        args: Vec<HirExpr>,
    },
    Block(Vec<HirStatement>),
    Index {
        span: SourceSpan,
        array: Box<HirExpr>,
        index: Box<HirExpr>,
    },
    Array(Vec<HirExpr>),
    If {
        condition: Box<HirExpr>,
        then_block: Vec<HirStatement>,
        else_if: Vec<(HirExpr, Vec<HirStatement>)>,
        else_block: Option<Vec<HirStatement>>,
    },
    Ref(Box<HirExpr>),
    EnumVariant {
        type_name: String,
        variant_index: usize,
        payload: Vec<HirExpr>,
    },
    Tuple(Vec<HirExpr>, SourceSpan),
    FieldAccess {
        span: SourceSpan,
        object: Box<HirExpr>,
        name: String,
    },
}

pub type Type = vinyl_parser::ast::types::Type;
