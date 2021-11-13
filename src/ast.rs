#[derive(Debug)]
pub enum AST {
    Function(Function),
    Variable(Variable)
}

#[derive(Debug)]
pub struct Function {
    pub fn_type: Type,
    pub name: Span,
    pub block: Option<Block>,
    pub span: Span,
    pub id: usize,
}

#[derive(Debug)]
pub enum Type {
    Primitive(PrimitiveType),
}

#[derive(Debug)]
pub enum PrimitiveType {
    I8,
    I16,
    I32,
    I64,
    I128,
    U8,
    U16,
    U32,
    U64,
    U128,
    Bool,
    Char,
    Float32,
    Float64,
}

#[derive(Debug)]
pub struct Block {
    pub statements: Vec<Statement>,
    pub span: Span,
    pub id: usize,
}

#[derive(Debug)]
pub struct Statement {
    pub kind: StatementKind,
    pub span: Span,
    pub id: usize,
}

#[derive(Debug)]
pub enum StatementKind {
    Variable(Variable),
    Expression(Expression),
}

#[derive(Debug)]
pub struct Variable {
    pub var_type: Type,
    pub expression: Option<Expression>,
    pub name: Span,
    pub span: Span,
    pub id: usize,
}

pub struct Expression {
    pub id: usize,
    pub span: Span,
    pub kind: ExpressionKind
}

pub enum ExpressionKind {
    Binary(BinaryOperator, Expression, Expression),
    Litera(Literal),
}

pub struct Literal {
    pub kind: LiteralKind,
    pub span: Span,
    pub id: usize,
}

pub enum LiteralKind {
    String,
    Char,
    Int,
    Bool,
    Float
}

pub struct BinaryOperator {
    pub kind: BinaryOperatorKind,
}

pub enum BinaryOperatorKind {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulus,
    And,
    Or,
    BitXor,
    BitAnd,
    BitOr,
    ShiftLeft,
    ShiftRight,
    Equal,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
}

#[derive(Debug)]
pub struct Span(pub usize, pub usize);
