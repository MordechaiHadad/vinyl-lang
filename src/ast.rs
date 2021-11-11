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
    Float128,
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
    Variable(Variable)
}

#[derive(Debug)]
pub struct Variable {
    pub is_initialized: bool,
    pub var_type: Type,
    pub name: Span,
    pub id: usize,
}

#[derive(Debug)]
pub struct Span(pub usize, pub usize);
