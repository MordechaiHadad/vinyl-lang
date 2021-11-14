pub enum TreeSitter {
    VariableDeclaration = 26,
    FunctionDeclaration = 27,

    PrimitiveType = 3,
    Identifier = 16,
    EqualSign = 1,

    IntegerLiteal = 8,
    FloatingPointLiteral = 9,
    StringLiteral = 31,
    CharLiteral = 32,
    BoolLiteral = 33,

    SemiColon = 2,
}

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

#[derive(Debug)]
pub struct Expression {
    pub id: usize,
    pub span: Span,
    pub kind: Box<ExpressionKind>
}

#[derive(Debug)]
pub enum ExpressionKind {
    Binary(BinaryOperator, Box<Expression>, Box<Expression>),
    Literal(Literal),
}

#[derive(Debug)]
pub struct Literal {
    pub kind: LiteralKind,
    pub value: Span,
    pub id: usize,
}

#[derive(Debug)]
pub enum LiteralKind {
    String,
    Char,
    Int,
    Bool,
    Float
}

#[derive(Debug)]
pub struct BinaryOperator {
    pub kind: BinaryOperatorKind,
}

#[derive(Debug)]
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