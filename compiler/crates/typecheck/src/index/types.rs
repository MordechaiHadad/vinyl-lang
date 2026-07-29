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
}

#[derive(Debug, Default)]
pub struct HirIndex {
    pub expr_at_pos: BTreeMap<usize, HirExprRef>,
    pub definitions: HashMap<String, Vec<Definition>>,
    pub references: BTreeMap<usize, Definition>,
    pub unused: Vec<Definition>,
}
