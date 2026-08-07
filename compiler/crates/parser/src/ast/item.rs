use miette::SourceSpan;

use crate::ast::{expression::Expression, statement::Statement, types::Type};

#[derive(Debug, Clone)]
pub enum Item {
    Function(FunctionDef),
    Struct(StructDef),
    TupleStruct(TupleDef),
    Enum(EnumDef),
    TypeAlias(TypeAliasDef),
    Import(ImportDef),
}

impl Item {
    pub fn span(&self) -> SourceSpan {
        match self {
            Item::Function(f) => f.span,
            Item::Struct(s) => s.span,
            Item::TupleStruct(t) => t.span,
            Item::Enum(e) => e.span,
            Item::TypeAlias(a) => a.span,
            Item::Import(i) => i.span,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FunctionDef {
    pub span: SourceSpan,
    pub public: bool,
    pub attrs: Vec<Attribute>,
    pub name: String,
    pub params: Vec<FunctionParam>,
    pub return_type: Option<Type>,
    pub body: Vec<Statement>,
}

#[derive(Debug, Clone)]
pub struct FunctionParam {
    pub span: SourceSpan,
    pub name: String,
    pub mutable: bool,
    pub type_: Type,
}

#[derive(Debug, Clone)]
pub struct StructDef {
    pub span: SourceSpan,
    pub public: bool,
    pub attrs: Vec<Attribute>,
    pub name: String,
    pub fields: Vec<StructField>,
}

#[derive(Debug, Clone)]
pub struct StructField {
    pub span: SourceSpan,
    pub public: bool,
    pub name: String,
    pub type_: Type,
}

#[derive(Debug, Clone)]
pub struct TupleDef {
    pub span: SourceSpan,
    pub public: bool,
    pub attrs: Vec<Attribute>,
    pub name: String,
    pub types: Vec<Type>,
}

#[derive(Debug, Clone)]
pub struct EnumDef {
    pub span: SourceSpan,
    pub public: bool,
    pub attrs: Vec<Attribute>,
    pub name: String,
    pub variants: Vec<EnumVariant>,
}

#[derive(Debug, Clone)]
pub struct EnumVariant {
    pub span: SourceSpan,
    pub name: String,
    pub data: Option<EnumVariantData>,
}

#[derive(Debug, Clone)]
pub struct TypeAliasDef {
    pub span: SourceSpan,
    pub public: bool,
    pub attrs: Vec<Attribute>,
    pub name: String,
    pub type_: Type,
}

#[derive(Debug, Clone)]
pub enum EnumVariantData {
    Tuple(Vec<Type>),
    Struct(Vec<StructField>),
}

#[derive(Debug, Clone)]
pub struct ImportDef {
    pub span: SourceSpan,
    pub prefix: Vec<String>,
    pub path: Vec<String>,
    pub symbols: Vec<String>,
    pub wildcard: bool,
}

#[derive(Debug, Clone)]
pub struct Attribute {
    pub span: SourceSpan,
    pub name: String,
    pub args: Vec<Expression>,
}
