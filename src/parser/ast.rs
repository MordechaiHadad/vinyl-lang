use lasso::Spur;

pub enum TreeSitter {
    VariableDeclaration = 42,
    FunctionDeclaration = 45,

    PrimitiveType = 3,
    Identifier = 6,

    Literal = 50,
    IntegerLiteral = 26,
    FloatingPointLiteral = 27,
    Reference = 51,
    StringLiteral = 52,
    CharLiteral = 53,
    BoolLiteral = 54,

    BinaryExpression = 48,

    EqualSign = 1,
    SemiColon = 2,

    And = 7,
    Or = 8,
    BitAnd = 9,
    BitOr = 10,
    BitXor = 11,
    Equal = 12,
    NotEqual = 13,
    LessThan = 14,
    LessThanOrEqual = 15,
    GreaterThan = 16,
    GreaterThanOrEqual = 17,
    ShiftLeft = 18,
    ShiftRight = 19,
    PlusSign = 20,
    MinusSign = 21,
    Multiply = 22,
    Divide = 23,
    Modulus = 24,

    Parameters = 55,
    Parameter = 56,
    Block = 57,
    LeftParen = 34,
    RightParen = 36,
    LeftCurly = 37,
    RightCurly = 38,
}

#[derive(Debug)]
pub enum AST {
    Function(Function),
    Variable(Variable),
}

#[derive(Debug)]
pub struct Function {
    pub return_type: Type,
    pub name: Spur,
    pub parameters: Option<Parameters>,
    pub body: Option<Block>,
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
    Void,
    Var,
}

#[derive(Debug)]
pub struct Block {
    pub statements: Vec<Statement>,
    pub span: Span,
    pub id: usize,
}

#[derive(Debug)]
pub struct Parameters {
    pub parameters: Vec<Parameter>,
    pub span: Span,
    pub id: usize,
}

#[derive(Debug)]
pub struct Parameter {
    pub param_type: Type,
    pub name: Spur,
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
    pub name: Spur,
    pub span: Span,
    pub id: usize,
}

#[derive(Debug)]
pub struct Expression {
    pub id: usize,
    pub span: Span,
    pub kind: Box<ExpressionKind>,
}

#[derive(Debug)]
pub enum ExpressionKind {
    Binary(BinaryOperator, Box<Expression>, Box<Expression>),
    Literal(Literal),
    Reference,
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
    Float,
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

#[derive(Debug, Clone, Copy)]
pub struct Span(pub usize, pub usize);