use std::collections::HashMap;

use miette::{NamedSource, SourceSpan};
use vinyl_parser::ast::item::{EnumVariantData, FunctionDef, Item};

use crate::error::{CompileWarning, TypeError};
use crate::hir::{
    HirEnum, HirEnumVariant, HirEnumVariantData, HirField, HirItem, HirItemKind, HirStruct,
    HirTupleStruct, Type,
};

pub mod expression;
pub mod literal;
pub mod resolve;
pub mod scope;
pub mod statement;
pub mod unify;

use scope::ScopeState;
use unify::SubstitutionState;

#[derive(Debug, Clone)]
struct TypeScheme {
    type_: Type,
    mutable: bool,
}

pub(super) struct SourceContext {
    source: String,
    source_name: String,
}

impl SourceContext {
    fn new(source: &str, source_name: &str) -> Self {
        SourceContext {
            source: source.to_string(),
            source_name: source_name.to_string(),
        }
    }

    pub(super) fn error(&self, span: SourceSpan, message: String) -> TypeError {
        TypeError {
            message,
            source: NamedSource::new(&self.source_name, self.source.to_string()),
            span,
        }
    }

    pub(super) fn warn(&self, span: SourceSpan, message: String) -> CompileWarning {
        CompileWarning {
            message,
            source: NamedSource::new(&self.source_name, self.source.to_string()),
            span,
        }
    }
}

struct InferState {
    source: SourceContext,
    scope: ScopeState,
    types: HashMap<String, HirItemKind>,
    subs: SubstitutionState,
    current_return_type: Option<Type>,
    loop_depth: usize,
    errors: Vec<TypeError>,
    warnings: Vec<CompileWarning>,
}

impl InferState {
    fn new(source: &str, source_name: &str) -> Self {
        InferState {
            source: SourceContext::new(source, source_name),
            scope: ScopeState::new(),
            types: HashMap::new(),
            subs: SubstitutionState::new(),
            current_return_type: None,
            loop_depth: 0,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }
}

pub fn typeck(
    items: &[Item],
    source: &str,
    source_name: &str,
    warnings: &mut Vec<CompileWarning>,
) -> Result<Vec<HirItem>, Vec<TypeError>> {
    let mut state = InferState::new(source, source_name);

    let signatures: HashMap<&str, &FunctionDef> = items
        .iter()
        .filter_map(|item| {
            if let Item::Function(f) = item {
                Some((f.name.as_str(), f))
            } else {
                None
            }
        })
        .collect();

    let mut hir_items: Vec<HirItem> = Vec::new();

    for item in items {
        let hir_item = match item {
            Item::Struct(s) => Some(HirItem {
                kind: HirItemKind::Struct(HirStruct {
                    name: s.name.clone(),
                    repr_c: s.attrs.iter().any(|a| a.name == "repr_c"),
                    fields: s
                        .fields
                        .iter()
                        .map(|f| HirField {
                            name: f.name.clone(),
                            type_: f.type_.clone(),
                        })
                        .collect(),
                }),
            }),
            Item::TupleStruct(t) => Some(HirItem {
                kind: HirItemKind::TupleStruct(HirTupleStruct {
                    name: t.name.clone(),
                    types: t.types.clone(),
                }),
            }),
            Item::Enum(e) => Some(HirItem {
                kind: HirItemKind::Enum(HirEnum {
                    name: e.name.clone(),
                    variants: e
                        .variants
                        .iter()
                        .map(|v| HirEnumVariant {
                            name: v.name.clone(),
                            data: v.data.as_ref().map(|d| match d {
                                EnumVariantData::Tuple(types) => {
                                    HirEnumVariantData::Tuple(types.clone())
                                }
                                EnumVariantData::Struct(fields) => HirEnumVariantData::Struct(
                                    fields
                                        .iter()
                                        .map(|f| HirField {
                                            name: f.name.clone(),
                                            type_: f.type_.clone(),
                                        })
                                        .collect(),
                                ),
                            }),
                        })
                        .collect(),
                }),
            }),
            Item::Function(_) => None,
        };
        if let Some(hir) = hir_item {
            let name = match &hir.kind {
                HirItemKind::Struct(s) => s.name.clone(),
                HirItemKind::TupleStruct(t) => t.name.clone(),
                HirItemKind::Enum(e) => e.name.clone(),
                _ => unreachable!(),
            };
            state.types.insert(name, hir.kind.clone());
            hir_items.push(hir);
        }
    }

    for item in items {
        if let Item::Function(f) = item {
            match state.infer_function(f, &signatures) {
                Ok(hir) => hir_items.push(HirItem {
                    kind: HirItemKind::Function(hir),
                }),
                Err(e) => state.errors.push(e),
            }
        }
    }

    warnings.append(&mut state.warnings);

    if state.errors.is_empty() {
        Ok(hir_items)
    } else {
        Err(state.errors)
    }
}
