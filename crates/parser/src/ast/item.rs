use miette::SourceSpan;

use crate::ast::{expression::Expression, statement::Statement, types::Type};

#[derive(Debug)]
pub enum Item {
    Function(FunctionDef),
    Struct(StructDef),
    TupleStruct(TupleDef),
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
    pub attrs: Vec<Attribute>,
    pub name: String,
    pub params: Vec<FunctionParam>,
    pub return_type: Option<Type>,
    pub body: Vec<Statement>,
}

#[derive(Debug)]
pub struct FunctionParam {
    pub span: SourceSpan,
    pub name: String,
    pub mutable: bool,
    pub type_: Type,
}

#[derive(Debug)]
pub struct StructDef {
    pub span: SourceSpan,
    pub attrs: Vec<Attribute>,
    pub name: String,
    pub fields: Vec<StructField>,
}

#[derive(Debug)]
pub struct StructField {
    pub span: SourceSpan,
    pub name: String,
    pub type_: Type,
}

#[derive(Debug)]
pub struct TupleDef {
    pub span: SourceSpan,
    pub attrs: Vec<Attribute>,
    pub name: String,
    pub types: Vec<Type>,
}

#[derive(Debug)]
pub struct EnumDef {
    pub span: SourceSpan,
    pub attrs: Vec<Attribute>,
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
    Struct(Vec<StructField>),
}

#[derive(Debug)]
pub struct Attribute {
    pub span: SourceSpan,
    pub name: String,
    pub args: Vec<Expression>,
}
