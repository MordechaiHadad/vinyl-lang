use lasso::Spur;
use num_derive::FromPrimitive;
use std::convert::TryFrom;
use std::fmt;
use std::fmt::Formatter;

#[derive(Clone, Copy, FromPrimitive)]
pub enum TreesitterNodeID {
    EqualSign = 1,
    SemiColon = 2,
    MutabilitySpecifier = 3,
    Comment = 4,
    PrimitiveType = 5,
    LeftBracket = 6,
    RightBracket = 7,
    And = 9,
    Or = 10,
    BitAnd = 11,
    BitOr = 12,
    BitXor = 13,
    Equal = 14,
    NotEqual = 15,
    LessThan = 16,
    LessThanOrEqual = 17,
    GreaterThan = 18,
    GreaterThanOrEqual = 19,
    ShiftLeft = 20,
    ShiftRight = 21,
    PlusSign = 22,
    MinusSign = 23,
    Multiply = 24,
    Divide = 25,
    Modulus = 26,
    New = 27,
    IntegerLiteral = 28,
    RealLiteral = 29,
    True = 34,
    False = 35,
    LeftParen = 36,
    RightParen = 38,
    LeftCurly = 39,
    RightCurly = 40,
    SourceFile = 41,
    VariableDeclaration = 44,
    FunctionDeclaration = 45,
    ArrayType = 47,
    Identifier = 48,
    BinaryExpression = 50,
    ArrayCreationExpression = 51,
    Literal = 52,
    Reference = 53,
    StringLiteral = 54,
    CharLiteral = 55,
    BoolLiteral = 56,
    Parameters = 57,
    Parameter = 58,
    Block = 59,
}

#[derive(Debug, Clone)]
pub struct AST {
    pub namespaces: Vec<Namespace>,
}

#[derive(Debug, Clone)]
pub struct Namespace {
    pub name: Spur,
    pub statements: Vec<Statement>,
    pub span: Span,
    pub id: usize,
}

#[derive(Debug, Clone)]
pub struct Function {
    pub return_type: Type,
    pub name: Spur,
    pub parameters: Option<Parameters>,
    pub body: Option<Block>,
    pub span: Span,
    pub id: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Type {
    Primitive(PrimitiveType),
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let result = match self {
            Type::Primitive(primitive) => match primitive {
                PrimitiveType::I8 => "int8",
                PrimitiveType::I16 => "int16",
                PrimitiveType::I32 => "int32",
                PrimitiveType::I64 => "int64",
                PrimitiveType::I128 => "int128",
                PrimitiveType::U8 => "uint8",
                PrimitiveType::U16 => "uint16",
                PrimitiveType::U32 => "uint32",
                PrimitiveType::U64 => "uint64",
                PrimitiveType::U128 => "uint128",
                PrimitiveType::Char => "char",
                PrimitiveType::Bool => "bool",
                PrimitiveType::Float32 => "float32",
                PrimitiveType::Float64 => "float64",
                PrimitiveType::String => "string",
                PrimitiveType::Void => "void",
                PrimitiveType::Var => "var",
            },
        };

        write!(f, "{}", result)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
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
    String,
    Void,
    Var,
}

#[derive(Debug, Clone)]
pub struct Block {
    pub statements: Vec<Statement>,
    pub span: Span,
    pub id: usize,
}

#[derive(Debug, Clone)]
pub struct Parameters {
    pub parameters: Vec<Parameter>,
    pub span: Span,
    pub id: usize,
}

#[derive(Debug, Clone)]
pub struct Parameter {
    pub param_type: Type,
    pub name: Spur,
    pub span: Span,
    pub id: usize,
}

#[derive(Debug, Clone)]
pub struct Statement {
    pub kind: StatementKind,
    pub span: Span,
    pub id: usize,
}

#[derive(Debug, Clone)]
pub enum StatementKind {
    Variable(Variable),
    Function(Function),
    Expression(Expression),
}

#[derive(Debug, Clone)]
pub struct Variable {
    pub mutability: Mutability,
    pub var_type: Type,
    pub expression: Option<Expression>,
    pub name: Spur,
    pub span: Span,
    pub id: usize,
}

#[derive(Debug, Clone)]
pub struct Expression {
    pub id: usize,
    pub span: Span,
    pub kind: Box<ExpressionKind>,
}

#[derive(Debug, Clone)]
pub enum ExpressionKind {
    Binary(BinaryOperator, Box<Expression>, Box<Expression>),
    Literal(Literal),
    Reference(Mutability, Identifier),
}

#[derive(Debug, Clone)]
pub struct Identifier {
    pub symbol: Spur,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum Mutability {
    Mutable,
    Immutable,
}

#[derive(Debug, Clone, Copy)]
pub struct Literal {
    pub kind: LiteralKind,
    pub value: Spur,
    pub span: Span,
    pub id: usize,
}

#[derive(Debug, Clone, Copy)]
pub enum LiteralKind {
    String,
    Char,
    Int,
    Bool,
    Float,
}

#[derive(Debug, Clone)]
pub struct BinaryOperator {
    pub kind: BinaryOperatorKind,
}

#[derive(Debug, Clone)]
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
pub struct Span {
    pub range: (usize, usize),
    pub file_id: &'static str,
}

impl ariadne::Span for Span {
    type SourceId = &'static str;

    fn source(&self) -> &&'static str {
        &self.file_id
    }

    fn start(&self) -> usize {
        self.range.0
    }
    fn end(&self) -> usize {
        self.range.1
    }
}
