#[derive(Debug)]
pub enum Item {
    Function(FunctionDef),
    Struct(StructDef),
    Enum(EnumDef),
}

#[derive(Debug)]
pub struct FunctionDef {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<Type>,
    pub body: Vec<Stmt>,
}

#[derive(Debug)]
pub struct Param {
    pub name: String,
    pub mutable: bool,
    pub type_: Type,
}

#[derive(Debug)]
pub struct StructDef {
    pub name: String,
    pub fields: Vec<Field>,
}

#[derive(Debug)]
pub struct Field {
    pub name: String,
    pub type_: Type,
}

#[derive(Debug)]
pub enum EnumDef {
    Variants(Vec<EnumVariant>),
}

#[derive(Debug)]
pub struct EnumVariant {
    pub name: String,
    pub data: Option<EnumVariantData>,
}

#[derive(Debug)]
pub enum EnumVariantData {
    Tuple(Vec<Type>),
    Struct(Vec<Field>),
}

#[derive(Debug)]
pub enum Type {
    Primitive(Primitive),
    Named(String),
    Generic { name: String, args: Vec<Type> },
    Ref(Box<Type>),
    Array { element: Box<Type>, size: usize },
}

#[derive(Debug)]
pub enum Primitive {
    Int8, Int16, Int32, Int64, Int128,
    UInt8, UInt16, UInt32, UInt64, UInt128,
    Float32, Float64,
    Bool,
    Char,
    String,
    Unit,
}

#[derive(Debug)]
pub enum Stmt {
    Let { name: String, mutable: bool, type_: Option<Type>, value: Expr },
    Expr(Expr),
    Return(Option<Expr>),
    If { condition: Expr, then_block: Vec<Stmt>, else_if: Vec<(Expr, Vec<Stmt>)>, else_block: Option<Vec<Stmt>> },
}

#[derive(Debug)]
pub enum Expr {
    Int(i128),
    Float(f64),
    String(String),
    Char(char),
    Bool(bool),
    Ident(String),
    Binary { left: Box<Expr>, op: BinaryOp, right: Box<Expr> },
    Unary { op: UnaryOp, operand: Box<Expr> },
    Call { function: Box<Expr>, args: Vec<Expr> },
    Block(Vec<Stmt>),
    Match { value: Box<Expr>, arms: Vec<MatchArm> },
    Field { object: Box<Expr>, name: String },
    Index { array: Box<Expr>, index: Box<Expr> },
    Tuple(Vec<Expr>),
    Array(Vec<Expr>),
    Paren(Box<Expr>),
}

#[derive(Debug)]
pub enum BinaryOp {
    Add, Sub, Mul, Div, Rem,
    Pow, FloorDiv,
    Eq, Ne, Lt, Gt, Le, Ge,
    And, Or,
    BitAnd, BitOr, BitXor, Shl, Shr,
    Range, RangeInclusive,
}

#[derive(Debug)]
pub enum UnaryOp {
    Neg, Not,
}

#[derive(Debug)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub body: Box<Expr>,
}

#[derive(Debug)]
pub enum Pattern {
    Wildcard,
    Ident(String),
    Literal(LiteralPattern),
    Struct { name: String, fields: Vec<(String, Pattern)> },
    Tuple(Vec<Pattern>),
}

#[derive(Debug)]
pub enum LiteralPattern {
    Int(i128),
    Bool(bool),
    Char(char),
    String(String),
}
