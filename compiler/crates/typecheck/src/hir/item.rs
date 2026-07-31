use miette::SourceSpan;

use crate::hir::statement::HirStatement;
use crate::hir::types::Type;

#[derive(Debug, Clone)]
pub struct HirItem {
    pub span: SourceSpan,
    pub kind: HirItemKind,
}

#[derive(Debug, Clone)]
pub enum HirItemKind {
    Function(HirFunction),
    Struct(HirStruct),
    TupleStruct(HirTupleStruct),
    Enum(HirEnum),
}

#[derive(Debug, Clone)]
pub struct HirFunction {
    pub span: SourceSpan,
    pub name: String,
    pub public: bool,
    pub params: Vec<HirParam>,
    pub return_type: Type,
    pub body: Vec<HirStatement>,
}

#[derive(Debug, Clone)]
pub struct HirStruct {
    pub span: SourceSpan,
    pub name: String,
    pub public: bool,
    pub repr_c: bool,
    pub fields: Vec<HirField>,
}

#[derive(Debug, Clone)]
pub struct HirTupleStruct {
    pub span: SourceSpan,
    pub name: String,
    pub public: bool,
    pub types: Vec<Type>,
}

#[derive(Debug, Clone)]
pub struct HirEnum {
    pub span: SourceSpan,
    pub name: String,
    pub public: bool,
    pub variants: Vec<HirEnumVariant>,
}

#[derive(Debug, Clone)]
pub struct HirField {
    pub name: String,
    pub type_: Type,
}

#[derive(Debug, Clone)]
pub struct HirEnumVariant {
    pub name: String,
    pub data: Option<HirEnumVariantData>,
}

#[derive(Debug, Clone)]
pub enum HirEnumVariantData {
    Tuple(Vec<Type>),
    Struct(Vec<HirField>),
}

#[derive(Debug, Clone)]
pub struct HirParam {
    pub span: SourceSpan,
    pub name: String,
    pub mutable: bool,
    pub type_: Type,
}
