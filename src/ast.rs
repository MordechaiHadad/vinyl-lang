pub enum AST {
    Function(Function),
    Type(Type),
}

pub struct Function {
    pub fun_type: Type,
    pub name: Span,
    pub block: Option<Box<Block>>,
}

pub enum Type {
    Primitive(PrimitiveType),
}

pub enum PrimitiveType {
    I8,
    I16,
    I32,
    I64
}

pub struct Block {}

pub struct Span {
    start: usize,
    end: usize,
}
