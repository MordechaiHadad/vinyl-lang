use vinyl_parser::ast::operator::AssignOp as ParserAssignOp;

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

impl AssignOp {
    pub fn from_parser(op: &ParserAssignOp) -> Self {
        match op {
            ParserAssignOp::Eq => AssignOp::Eq,
            ParserAssignOp::AddEq => AssignOp::AddEq,
            ParserAssignOp::SubEq => AssignOp::SubEq,
            ParserAssignOp::MulEq => AssignOp::MulEq,
            ParserAssignOp::DivEq => AssignOp::DivEq,
            ParserAssignOp::RemEq => AssignOp::RemEq,
            ParserAssignOp::BitAndEq => AssignOp::BitAndEq,
            ParserAssignOp::BitOrEq => AssignOp::BitOrEq,
            ParserAssignOp::BitXorEq => AssignOp::BitXorEq,
            ParserAssignOp::ShlEq => AssignOp::ShlEq,
            ParserAssignOp::ShrEq => AssignOp::ShrEq,
        }
    }
}
