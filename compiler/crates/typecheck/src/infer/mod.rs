use std::collections::HashMap;

use miette::{NamedSource, SourceSpan};
use vinyl_parser::ast::item::{EnumVariantData, FunctionDef, Item};

use crate::error::{CompileWarning, TypeError, TypeErrorKind};
use crate::hir::{
    HirEnum, HirEnumVariant, HirEnumVariantData, HirField, HirItem, HirItemKind, HirStruct,
    HirTupleStruct, Type,
};
use crate::module::ModuleTable;

use crate::index::builder::IndexBuilder;
pub use crate::index::{Definition, DefinitionKind, HirExprRef, TypeckResult};

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
            kind: TypeErrorKind::Message(message),
            source_code: NamedSource::new(&self.source_name, self.source.to_string()),
            span,
        }
    }

    pub(super) fn type_mismatch(&self, span: SourceSpan, expected: Type, found: Type) -> TypeError {
        TypeError {
            kind: TypeErrorKind::Mismatch { expected, found },
            source_code: NamedSource::new(&self.source_name, self.source.to_string()),
            span,
        }
    }

    pub(super) fn warn(&self, span: SourceSpan, message: String) -> CompileWarning {
        CompileWarning {
            message,
            source_code: NamedSource::new(&self.source_name, self.source.to_string()),
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
    module_table: ModuleTable,
}

impl InferState {
    fn new(source: &str, source_name: &str, module_table: &ModuleTable) -> Self {
        InferState {
            source: SourceContext::new(source, source_name),
            scope: ScopeState::new(),
            types: HashMap::new(),
            subs: SubstitutionState::new(),
            current_return_type: None,
            loop_depth: 0,
            errors: Vec::new(),
            warnings: Vec::new(),
            module_table: module_table.clone(),
        }
    }
}

pub fn typeck(
    items: &[Item],
    source: &str,
    source_name: &str,
    warnings: &mut Vec<CompileWarning>,
) -> Result<Vec<HirItem>, Vec<TypeError>> {
    typeck_with_modules(items, source, source_name, warnings, &ModuleTable::new())
}

pub fn typeck_with_modules(
    items: &[Item],
    source: &str,
    source_name: &str,
    warnings: &mut Vec<CompileWarning>,
    module_table: &ModuleTable,
) -> Result<Vec<HirItem>, Vec<TypeError>> {
    let mut state = InferState::new(source, source_name, module_table);

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
                span: s.span,
                kind: HirItemKind::Struct(HirStruct {
                    span: s.span,
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
                span: t.span,
                kind: HirItemKind::TupleStruct(HirTupleStruct {
                    span: t.span,
                    name: t.name.clone(),
                    types: t.types.clone(),
                }),
            }),
            Item::Enum(e) => Some(HirItem {
                span: e.span,
                kind: HirItemKind::Enum(HirEnum {
                    span: e.span,
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
            Item::Import(_) => None,
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
                    span: f.span,
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

pub fn typeck_with_index(
    items: &[Item],
    source: &str,
    source_name: &str,
    warnings: &mut Vec<CompileWarning>,
    module_table: &ModuleTable,
) -> Result<TypeckResult, Vec<TypeError>> {
    let hir_items = typeck_with_modules(items, source, source_name, warnings, module_table)?;
    let index = IndexBuilder::default().build(&hir_items);
    Ok(TypeckResult {
        items: hir_items,
        expr_at_pos: index.expr_at_pos,
        definitions: index.definitions,
        references: index.references,
        unused: index.unused,
    })
}
