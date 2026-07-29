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
