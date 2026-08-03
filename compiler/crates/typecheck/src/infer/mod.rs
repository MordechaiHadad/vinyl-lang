use std::collections::{BTreeMap, HashMap};

use miette::{NamedSource, SourceSpan};
use vinyl_parser::ast::expression::Expression;
use vinyl_parser::ast::item::{EnumVariantData, FunctionDef, Item};
use vinyl_parser::ast::statement::Statement;

use crate::error::{TypeDiagnostic, TypeDiagnosticKind};
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

    pub(super) fn error(&self, span: SourceSpan, kind: TypeDiagnosticKind) -> TypeDiagnostic {
        TypeDiagnostic {
            kind,
            source_code: NamedSource::new(&self.source_name, self.source.to_string()),
            span,
        }
    }

    pub(super) fn type_mismatch(
        &self,
        span: SourceSpan,
        expected: Type,
        found: Type,
    ) -> TypeDiagnostic {
        TypeDiagnostic {
            kind: TypeDiagnosticKind::Mismatch { expected, found },
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
    errors: Vec<TypeDiagnostic>,
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
            module_table: module_table.clone(),
        }
    }
}

pub fn typeck(
    items: &[Item],
    source: &str,
    source_name: &str,
) -> Result<(Vec<HirItem>, Vec<TypeDiagnostic>), Vec<TypeDiagnostic>> {
    typeck_with_modules(items, source, source_name, &ModuleTable::new())
}

pub fn typeck_with_modules(
    items: &[Item],
    source: &str,
    source_name: &str,
    module_table: &ModuleTable,
) -> Result<(Vec<HirItem>, Vec<TypeDiagnostic>), Vec<TypeDiagnostic>> {
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
                    public: s.public,
                    repr_c: s.attrs.iter().any(|a| a.name == "repr_c"),
                    fields: s
                        .fields
                        .iter()
                        .map(|f| HirField {
                            span: f.span,
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
                    public: t.public,
                    types: t.types.clone(),
                }),
            }),
            Item::Enum(e) => Some(HirItem {
                span: e.span,
                kind: HirItemKind::Enum(HirEnum {
                    span: e.span,
                    name: e.name.clone(),
                    public: e.public,
                    variants: e
                        .variants
                        .iter()
                        .map(|v| HirEnumVariant {
                            span: v.span,
                            name: v.name.clone(),
                            data: v.data.as_ref().map(|d| match d {
                                EnumVariantData::Tuple(types) => {
                                    HirEnumVariantData::Tuple(types.clone())
                                }
                                EnumVariantData::Struct(fields) => HirEnumVariantData::Struct(
                                    fields
                                        .iter()
                                        .map(|f| HirField {
                                            span: f.span,
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
                Err(e) => state.errors.push(*e),
            }
        }
    }

    let all_diagnostics = std::mem::take(&mut state.errors);
    let mut fatal_errors = Vec::new();
    let mut warnings = Vec::new();
    for diagnostic in all_diagnostics {
        if matches!(diagnostic.kind, TypeDiagnosticKind::UnreachableStatement) {
            warnings.push(diagnostic);
        } else {
            fatal_errors.push(diagnostic);
        }
    }
    if fatal_errors.is_empty() {
        Ok((hir_items, warnings))
    } else {
        Err(fatal_errors)
    }
}

pub fn typeck_with_index(
    items: &[Item],
    source: &str,
    source_name: &str,
    module_table: &ModuleTable,
) -> Result<(TypeckResult, Vec<TypeDiagnostic>), Vec<TypeDiagnostic>> {
    let (hir_items, warnings) = typeck_with_modules(items, source, source_name, module_table)?;
    let index = IndexBuilder::default().build(&hir_items);
    Ok((
        TypeckResult {
            items: hir_items,
            expr_at_pos: index.expr_at_pos,
            definitions: index.definitions,
            references: index.references,
            unused: index.unused,
            type_positions: collect_type_positions(items, source),
            field_accesses: index.field_accesses,
        },
        warnings,
    ))
}

fn collect_type_positions(items: &[Item], source: &str) -> BTreeMap<usize, String> {
    let mut positions = BTreeMap::new();
    for item in items {
        match item {
            Item::Function(f) => {
                if let Some(offset) = return_type_offset(source, f.span) {
                    positions.insert(
                        offset,
                        f.return_type
                            .as_ref()
                            .map(ToString::to_string)
                            .unwrap_or_default(),
                    );
                }
                for param in &f.params {
                    if let Some(offset) = type_after_colon(source, param.span) {
                        positions.insert(offset, param.type_.to_string());
                    }
                }
                for stmt in &f.body {
                    collect_statement_type_positions(stmt, source, &mut positions);
                }
            }
            Item::Struct(s) => {
                for field in &s.fields {
                    if let Some(offset) = type_after_colon(source, field.span) {
                        positions.insert(offset, field.type_.to_string());
                    }
                }
            }
            Item::TupleStruct(t) => {
                collect_parenthesized_types(source, t.span, &t.types, &mut positions);
            }
            Item::Enum(e) => {
                for variant in &e.variants {
                    if let Some(data) = &variant.data {
                        match data {
                            EnumVariantData::Tuple(types) => {
                                collect_parenthesized_types(
                                    source,
                                    variant.span,
                                    types,
                                    &mut positions,
                                );
                            }
                            EnumVariantData::Struct(fields) => {
                                for field in fields {
                                    if let Some(offset) = type_after_colon(source, field.span) {
                                        positions.insert(offset, field.type_.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Item::Import(_) => {}
        }
    }
    positions
}

fn collect_statement_type_positions(
    stmt: &Statement,
    source: &str,
    positions: &mut BTreeMap<usize, String>,
) {
    match stmt {
        Statement::Let {
            span,
            type_: Some(type_),
            ..
        } => {
            if let Some(offset) = let_type_offset(source, *span) {
                positions.insert(offset, type_.to_string());
            }
        }
        Statement::Expression(Expression::Block(stmts, _)) => {
            for s in stmts {
                collect_statement_type_positions(s, source, positions);
            }
        }
        Statement::If {
            then_block,
            else_if,
            else_block,
            ..
        } => {
            for s in then_block {
                collect_statement_type_positions(s, source, positions);
            }
            for (_, block) in else_if {
                for s in block {
                    collect_statement_type_positions(s, source, positions);
                }
            }
            if let Some(block) = else_block {
                for s in block {
                    collect_statement_type_positions(s, source, positions);
                }
            }
        }
        Statement::While { body, .. } | Statement::Loop { body, .. } => {
            for s in body {
                collect_statement_type_positions(s, source, positions);
            }
        }
        _ => {}
    }
}

fn type_after_colon(source: &str, span: SourceSpan) -> Option<usize> {
    let text = source.get(span.offset()..span.offset() + span.len())?;
    let colon = text.find(':')?;
    let after = &text[colon + 1..];
    let start = after.find(|c: char| !c.is_whitespace())?;
    Some(span.offset() + colon + 1 + start)
}

fn let_type_offset(source: &str, span: SourceSpan) -> Option<usize> {
    let text = source.get(span.offset()..span.offset() + span.len())?;
    let before_eq = text.split('=').next()?;
    let colon = before_eq.find(':')?;
    let after = &before_eq[colon + 1..];
    let start = after.find(|c: char| !c.is_whitespace())?;
    Some(span.offset() + colon + 1 + start)
}

fn return_type_offset(source: &str, span: SourceSpan) -> Option<usize> {
    let text = source.get(span.offset()..span.offset() + span.len())?;
    let paren = text.find(')')?;
    let after_paren = &text[paren + 1..];
    let colon = after_paren.find(':')?;
    let after = &after_paren[colon + 1..];
    let start = after.find(|c: char| !c.is_whitespace())?;
    Some(span.offset() + paren + 1 + colon + 1 + start)
}

fn collect_parenthesized_types(
    source: &str,
    span: SourceSpan,
    types: &[Type],
    positions: &mut BTreeMap<usize, String>,
) {
    let Some(text) = source.get(span.offset()..span.offset() + span.len()) else {
        return;
    };
    let Some(paren) = text.find('(') else {
        return;
    };
    let mut cursor = paren + 1;
    for type_ in types {
        let rest = &text[cursor..];
        let Some(start) = rest.find(|c: char| !c.is_whitespace()) else {
            return;
        };
        cursor += start;
        let name = type_.to_string();
        positions.insert(span.offset() + cursor, name.clone());
        cursor += name.len();
        let rest = &text[cursor..];
        if let Some(comma) = rest.find(',') {
            cursor += comma + 1;
        }
    }
}
