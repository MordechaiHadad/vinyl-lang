use miette::SourceSpan;
use std::fmt;

#[derive(Debug)]
pub enum Item {
    Function(FunctionDef),
    Struct(StructDef),
    TupleStruct(TupleStructDef),
    Enum(EnumDef),
}

impl Item {
    pub fn span(&self) -> SourceSpan {
        match self {
            Item::Function(f) => f.span,
            Item::Struct(s) => s.span,
            Item::TupleStruct(t) => t.span,
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
    pub attrs: Vec<Attr>,
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
pub struct TupleStructDef {
    pub span: SourceSpan,
    pub attrs: Vec<Attr>,
    pub name: String,
    pub types: Vec<Type>,
}

#[derive(Debug)]
pub struct EnumDef {
    pub span: SourceSpan,
    pub attrs: Vec<Attr>,
    pub name: String,
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
    Tuple(Vec<Type>),
    Var(usize),
}

impl Type {
    pub fn as_primitive(&self) -> Option<&Primitive> {
        match self {
            Type::Primitive(p) => Some(p),
            _ => None,
        }
    }

    pub fn is_int(&self) -> bool {
        self.as_primitive().is_some_and(|p| p.is_int())
    }

    pub fn is_uint(&self) -> bool {
        self.as_primitive().is_some_and(|p| p.is_uint())
    }

    pub fn is_signed(&self) -> bool {
        self.is_int()
    }

    pub fn is_unsigned(&self) -> bool {
        self.is_uint()
    }

    pub fn is_float(&self) -> bool {
        self.as_primitive().is_some_and(|p| p.is_float())
    }

    pub fn is_numeric(&self) -> bool {
        self.as_primitive().is_some_and(|p| p.is_numeric())
    }
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
            Type::Tuple(elements) => {
                write!(f, "(")?;
                for (i, t) in elements.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    fmt::Display::fmt(t, f)?;
                }
                if elements.len() == 1 {
                    write!(f, ",")?;
                }
                write!(f, ")")
            }
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

impl Primitive {
    pub fn is_int(&self) -> bool {
        matches!(
            self,
            Primitive::Int8
                | Primitive::Int16
                | Primitive::Int32
                | Primitive::Int64
                | Primitive::Int128
                | Primitive::ISize
        )
    }

    pub fn is_uint(&self) -> bool {
        matches!(
            self,
            Primitive::UInt8
                | Primitive::UInt16
                | Primitive::UInt32
                | Primitive::UInt64
                | Primitive::UInt128
                | Primitive::USize
        )
    }

    pub fn is_signed(&self) -> bool {
        self.is_int()
    }

    pub fn is_unsigned(&self) -> bool {
        self.is_uint()
    }

    pub fn is_float(&self) -> bool {
        matches!(self, Primitive::Float32 | Primitive::Float64)
    }

    pub fn is_numeric(&self) -> bool {
        self.is_int() || self.is_uint() || self.is_float()
    }
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
    Value(Expr, SourceSpan),
    If {
        span: SourceSpan,
        condition: Expr,
        then_block: Vec<Stmt>,
        else_if: Vec<(Expr, Vec<Stmt>)>,
        else_block: Option<Vec<Stmt>>,
    },
    While {
        span: SourceSpan,
        condition: Expr,
        body: Vec<Stmt>,
    },
    Loop {
        span: SourceSpan,
        body: Vec<Stmt>,
    },
    Break(SourceSpan),
    Continue(SourceSpan),
    Assign {
        span: SourceSpan,
        target: AssignTarget,
        op: AssignOp,
        value: Box<Expr>,
    },
}

impl Stmt {
    pub fn span(&self) -> SourceSpan {
        match self {
            Stmt::Let { span, .. } => *span,
            Stmt::Expr(e) => e.span(),
            Stmt::Return(_, span) => *span,
            Stmt::Value(_, span) => *span,
            Stmt::If { span, .. } => *span,
            Stmt::While { span, .. } => *span,
            Stmt::Loop { span, .. } => *span,
            Stmt::Break(span) => *span,
            Stmt::Continue(span) => *span,
            Stmt::Assign { span, .. } => *span,
        }
    }
}

#[derive(Debug)]
pub enum AssignTarget {
    Ident(String, SourceSpan),
    Index {
        span: SourceSpan,
        array: Box<Expr>,
        index: Box<Expr>,
    },
    Field {
        span: SourceSpan,
        object: Box<Expr>,
        name: String,
    },
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

#[derive(Debug)]
pub enum Expr {
    Int(i128, SourceSpan),
    Float(f64, SourceSpan),
    String(String, SourceSpan),
    Char(char, SourceSpan),
    Bool(bool, SourceSpan),
    Unit(SourceSpan),
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
    EnumVariant {
        span: SourceSpan,
        type_name: String,
        variant_name: String,
        args: Vec<Expr>,
    },
    Paren(Box<Expr>, SourceSpan),
    Ref {
        span: SourceSpan,
        operand: Box<Expr>,
    },
    If {
        span: SourceSpan,
        condition: Box<Expr>,
        then_block: Vec<Stmt>,
        else_if: Vec<(Expr, Vec<Stmt>)>,
        else_block: Option<Vec<Stmt>>,
    },
}

impl Expr {
    pub fn span(&self) -> SourceSpan {
        match self {
            Expr::Int(_, s) => *s,
            Expr::Float(_, s) => *s,
            Expr::String(_, s) => *s,
            Expr::Char(_, s) => *s,
            Expr::Bool(_, s) => *s,
            Expr::Unit(s) => *s,
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
            Expr::Ref { span, .. } => *span,
            Expr::If { span, .. } => *span,
            Expr::EnumVariant { span, .. } => *span,
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

#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOp {
    Neg,
    Not,
    Ref,
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
    EnumVariant {
        span: SourceSpan,
        name: String,
        patterns: Vec<Pattern>,
    },
}

impl Pattern {
    pub fn span(&self) -> SourceSpan {
        match self {
            Pattern::Wildcard(s) => *s,
            Pattern::Ident(_, s) => *s,
            Pattern::Literal(_, s) => *s,
            Pattern::Struct { span, .. } => *span,
            Pattern::Tuple(_, s) => *s,
            Pattern::EnumVariant { span, .. } => *span,
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
