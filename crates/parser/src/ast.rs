use miette::SourceSpan;
use std::fmt;

#[derive(Debug)]
pub enum Item {
    Function(FunctionDef),
    Struct(StructDef),
    Enum(EnumDef),
}

impl Item {
    pub fn span(&self) -> SourceSpan {
        match self {
            Item::Function(f) => f.span,
            Item::Struct(s) => s.span,
            Item::Enum(e) => e.span,
        }
    }
}

#[derive(Debug)]
pub struct FunctionDef {
    pub span: SourceSpan,
    pub attrs: Vec<Attr>,
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<Type>,
    pub body: Vec<Stmt>,
}

#[derive(Debug)]
pub struct Attr {
    pub span: SourceSpan,
    pub name: String,
    pub args: Vec<Expr>,
}

#[derive(Debug)]
pub struct Param {
    pub span: SourceSpan,
    pub name: String,
    pub mutable: bool,
    pub type_: Type,
}

#[derive(Debug)]
pub struct StructDef {
    pub span: SourceSpan,
    pub name: String,
    pub fields: Vec<Field>,
}

#[derive(Debug)]
pub struct Field {
    pub span: SourceSpan,
    pub name: String,
    pub type_: Type,
}

#[derive(Debug)]
pub struct EnumDef {
    pub span: SourceSpan,
    pub variants: Vec<EnumVariant>,
}

#[derive(Debug)]
pub struct EnumVariant {
    pub span: SourceSpan,
    pub name: String,
    pub data: Option<EnumVariantData>,
}

#[derive(Debug)]
pub enum EnumVariantData {
    Tuple(Vec<Type>),
    Struct(Vec<Field>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Primitive(Primitive),
    Named(String),
    Generic { name: String, args: Vec<Type> },
    Ref(Box<Type>),
    Array { element: Box<Type>, size: usize },
    Var(usize),
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Primitive(p) => fmt::Display::fmt(p, f),
            Type::Named(name) => write!(f, "{name}"),
            Type::Generic { name, args } => {
                write!(f, "{name}<")?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    fmt::Display::fmt(arg, f)?;
                }
                write!(f, ">")
            }
            Type::Ref(inner) => write!(f, "&{inner}"),
            Type::Array { element, size } => write!(f, "[{element}; {size}]"),
            Type::Var(id) => write!(f, "_{id}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Primitive {
    Int8,
    Int16,
    Int32,
    Int64,
    Int128,
    ISize,
    UInt8,
    UInt16,
    UInt32,
    UInt64,
    UInt128,
    USize,
    Float32,
    Float64,
    Bool,
    Char,
    String,
    Unit,
}

impl fmt::Display for Primitive {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Primitive::Int8 => write!(f, "int8"),
            Primitive::Int16 => write!(f, "int16"),
            Primitive::Int32 => write!(f, "int32"),
            Primitive::Int64 => write!(f, "int64"),
            Primitive::Int128 => write!(f, "int128"),
            Primitive::ISize => write!(f, "isize"),
            Primitive::UInt8 => write!(f, "uint8"),
            Primitive::UInt16 => write!(f, "uint16"),
            Primitive::UInt32 => write!(f, "uint32"),
            Primitive::UInt64 => write!(f, "uint64"),
            Primitive::UInt128 => write!(f, "uint128"),
            Primitive::USize => write!(f, "usize"),
            Primitive::Float32 => write!(f, "float32"),
            Primitive::Float64 => write!(f, "float64"),
            Primitive::Bool => write!(f, "bool"),
            Primitive::Char => write!(f, "char"),
            Primitive::String => write!(f, "string"),
            Primitive::Unit => write!(f, "unit"),
        }
    }
}

#[derive(Debug)]
pub enum Stmt {
    Let {
        span: SourceSpan,
        name: String,
        mutable: bool,
        type_: Option<Type>,
        value: Expr,
    },
    Expr(Expr),
    Return(Option<Expr>, SourceSpan),
    If {
        span: SourceSpan,
        condition: Expr,
        then_block: Vec<Stmt>,
        else_if: Vec<(Expr, Vec<Stmt>)>,
        else_block: Option<Vec<Stmt>>,
    },
}

impl Stmt {
    pub fn span(&self) -> SourceSpan {
        match self {
            Stmt::Let { span, .. } => *span,
            Stmt::Expr(e) => e.span(),
            Stmt::Return(_, span) => *span,
            Stmt::If { span, .. } => *span,
        }
    }
}

#[derive(Debug)]
pub enum Expr {
    Int(i128, SourceSpan),
    Float(f64, SourceSpan),
    String(String, SourceSpan),
    Char(char, SourceSpan),
    Bool(bool, SourceSpan),
    Ident(String, SourceSpan),
    Binary {
        span: SourceSpan,
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
    },
    Unary {
        span: SourceSpan,
        op: UnaryOp,
        operand: Box<Expr>,
    },
    Call {
        span: SourceSpan,
        function: Box<Expr>,
        args: Vec<Expr>,
    },
    Block(Vec<Stmt>, SourceSpan),
    Match {
        span: SourceSpan,
        value: Box<Expr>,
        arms: Vec<MatchArm>,
    },
    Field {
        span: SourceSpan,
        object: Box<Expr>,
        name: String,
    },
    Index {
        span: SourceSpan,
        array: Box<Expr>,
        index: Box<Expr>,
    },
    Tuple(Vec<Expr>, SourceSpan),
    Array(Vec<Expr>, SourceSpan),
    Paren(Box<Expr>, SourceSpan),
}

impl Expr {
    pub fn span(&self) -> SourceSpan {
        match self {
            Expr::Int(_, s) => *s,
            Expr::Float(_, s) => *s,
            Expr::String(_, s) => *s,
            Expr::Char(_, s) => *s,
            Expr::Bool(_, s) => *s,
            Expr::Ident(_, s) => *s,
            Expr::Binary { span, .. } => *span,
            Expr::Unary { span, .. } => *span,
            Expr::Call { span, .. } => *span,
            Expr::Block(_, s) => *s,
            Expr::Match { span, .. } => *span,
            Expr::Field { span, .. } => *span,
            Expr::Index { span, .. } => *span,
            Expr::Tuple(_, s) => *s,
            Expr::Array(_, s) => *s,
            Expr::Paren(_, s) => *s,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Pow,
    FloorDiv,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    And,
    Or,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    Range,
    RangeInclusive,
}

#[derive(Debug)]
pub enum UnaryOp {
    Neg,
    Not,
}

#[derive(Debug)]
pub struct MatchArm {
    pub span: SourceSpan,
    pub pattern: Pattern,
    pub body: Box<Expr>,
}

#[derive(Debug)]
pub enum Pattern {
    Wildcard(SourceSpan),
    Ident(String, SourceSpan),
    Literal(LiteralPattern, SourceSpan),
    Struct {
        span: SourceSpan,
        name: String,
        fields: Vec<(String, Pattern)>,
    },
    Tuple(Vec<Pattern>, SourceSpan),
}

impl Pattern {
    pub fn span(&self) -> SourceSpan {
        match self {
            Pattern::Wildcard(s) => *s,
            Pattern::Ident(_, s) => *s,
            Pattern::Literal(_, s) => *s,
            Pattern::Struct { span, .. } => *span,
            Pattern::Tuple(_, s) => *s,
        }
    }
}

#[derive(Debug)]
pub enum LiteralPattern {
    Int(i128),
    Bool(bool),
    Char(char),
    String(String),
}
