use miette::SourceSpan;
use vinyl_parser::ast::BinaryOp;

#[derive(Debug, Clone)]
pub struct HirItem {
    pub kind: HirItemKind,
}

#[derive(Debug, Clone)]
pub enum HirItemKind {
    Function(HirFunction),
}

#[derive(Debug, Clone)]
pub struct HirFunction {
    pub name: String,
    pub params: Vec<HirParam>,
    pub return_type: Type,
    pub body: Vec<HirStmt>,
}

#[derive(Debug, Clone)]
pub struct HirParam {
    pub name: String,
    pub mutable: bool,
    pub type_: Type,
}

#[derive(Debug, Clone)]
pub struct HirStmt {
    pub kind: HirStmtKind,
}

#[derive(Debug, Clone)]
pub enum HirStmtKind {
    Let {
        name: String,
        mutable: bool,
        type_: Type,
        value: HirExpr,
    },
    Expr(HirExpr),
    Return(Option<HirExpr>),
    If {
        condition: HirExpr,
        then_block: Vec<HirStmt>,
        else_if: Vec<(HirExpr, Vec<HirStmt>)>,
        else_block: Option<Vec<HirStmt>>,
    },
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
    Char(char),
    Ident(String),
    Binary {
        left: Box<HirExpr>,
        op: BinaryOp,
        right: Box<HirExpr>,
    },
    Call {
        function: Box<HirExpr>,
        args: Vec<HirExpr>,
    },
    Block(Vec<HirStmt>),
    Index {
        array: Box<HirExpr>,
        index: Box<HirExpr>,
    },
    Array(Vec<HirExpr>),
}

pub type Type = vinyl_parser::ast::Type;
