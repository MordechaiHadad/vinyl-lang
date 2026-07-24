use crate::hir::statement::HirStatement;
use crate::hir::types::Type;

#[derive(Debug, Clone)]
pub struct HirItem {
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
    pub name: String,
    pub params: Vec<HirParam>,
    pub return_type: Type,
    pub body: Vec<HirStatement>,
}

#[derive(Debug, Clone)]
pub struct HirStruct {
    pub name: String,
    pub repr_c: bool,
    pub fields: Vec<HirField>,
}

#[derive(Debug, Clone)]
pub struct HirTupleStruct {
    pub name: String,
    pub types: Vec<Type>,
}

#[derive(Debug, Clone)]
pub struct HirEnum {
    pub name: String,
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
    pub name: String,
    pub mutable: bool,
    pub type_: Type,
}
