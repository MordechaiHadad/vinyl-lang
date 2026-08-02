use std::collections::{BTreeMap, HashMap};

use miette::SourceSpan;

use crate::hir::{HirExpressionKind, HirItem, Type};

#[derive(Debug, Clone)]
pub struct HirExprRef {
    pub span: SourceSpan,
    pub kind: HirExpressionKind,
    pub type_: Type,
}

#[derive(Debug, Clone)]
pub struct FieldAccessRef {
    pub span: SourceSpan,
    pub object_type: Type,
    pub name: String,
}
#[derive(Debug, Clone)]
pub struct Definition {
    pub id: usize,
    pub name: String,
    pub kind: DefinitionKind,
    pub span: SourceSpan,
    pub scope_depth: usize,
    pub type_name: Option<String>,
}

#[derive(Debug, Clone)]
pub enum DefinitionKind {
    Function,
    Struct,
    Enum,
    TupleStruct,
    Variable,
    Parameter,
}

#[derive(Debug, Clone)]
pub struct TypeckResult {
    pub items: Vec<HirItem>,
    pub expr_at_pos: BTreeMap<usize, HirExprRef>,
    pub definitions: HashMap<String, Vec<Definition>>,
    pub references: BTreeMap<usize, Definition>,
    pub unused: Vec<Definition>,
    pub type_positions: BTreeMap<usize, String>,
    pub field_accesses: BTreeMap<usize, FieldAccessRef>,
}

#[derive(Debug, Default)]
pub struct HirIndex {
    pub expr_at_pos: BTreeMap<usize, HirExprRef>,
    pub definitions: HashMap<String, Vec<Definition>>,
    pub references: BTreeMap<usize, Definition>,
    pub unused: Vec<Definition>,
    pub field_accesses: BTreeMap<usize, FieldAccessRef>,
}
