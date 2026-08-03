use std::path::Path;

use line_index::LineIndex;
use tower_lsp::lsp_types::*;
use vinyl_typecheck::hir::{HirExpressionKind, HirItemKind, Type};
use vinyl_typecheck::{Definition, DefinitionKind, SourceSpan};

use crate::backend::state::{Analysis, Backend};
use crate::position::span_range;
use crate::text::name_range;

#[derive(Debug)]
pub(crate) enum SymbolRef {
    Ident { name: String, span: SourceSpan },
    Type { name: String },
    Field { name: String, object_type: Type },
    Variant { type_name: String, name: String },
    Module { name: String },
}

fn span_contains(span: (usize, usize), offset: usize) -> bool {
    offset >= span.0 && offset < span.1
}

fn type_name(type_: &Type) -> Option<&str> {
    match type_ {
        Type::Named(name) => Some(name),
        Type::Generic { name, .. } => Some(name),
        _ => None,
    }
}

/// Splits a `Type::X` or `module::item` span into (colon_offset, first, second).
fn split_segments(source: &str, span: SourceSpan) -> Option<(usize, String, String)> {
    let start = span.offset();
    let end = (start + span.len()).min(source.len());
    let text = source.get(start..end)?;
    let colon = text.find("::")?;
    let first = text[..colon].to_string();
    let after = &text[colon + 2..];
    let second = after
        .split(|character: char| !character.is_alphanumeric() && character != '_')
        .next()
        .unwrap_or("")
        .to_string();
    Some((colon, first, second))
}

pub(crate) fn resolve_symbol(analysis: &Analysis, offset: usize) -> Option<SymbolRef> {
    let source = &analysis.source;

    if let Some(symbol) = resolve_type_position(analysis, offset) {
        return Some(symbol);
    }
    if let Some(symbol) = resolve_field_access(analysis, offset) {
        return Some(symbol);
    }
    for definitions in analysis.result.definitions.values() {
        if let Some(definition) = definitions.iter().find(|definition| {
            !definition.name.contains("::")
                && span_contains(
                    name_range(
                        source,
                        (
                            definition.span.offset(),
                            definition.span.offset() + definition.span.len(),
                        ),
                        &definition.name,
                    ),
                    offset,
                )
        }) {
            return Some(SymbolRef::Ident {
                name: definition.name.clone(),
                span: definition.span,
            });
        }
    }
    let innermost = analysis
        .result
        .expr_at_pos
        .values()
        .filter(|expression| {
            span_contains(
                (
                    expression.span.offset(),
                    expression.span.offset() + expression.span.len(),
                ),
                offset,
            )
        })
        .min_by_key(|expression| expression.span.len());
    if let Some(expression) = innermost {
        return match &expression.kind {
            HirExpressionKind::Ident(name, span) => {
                if let Some((colon, module, item)) = split_segments(source, *span) {
                    let module_end = span.offset() + colon;
                    if offset < module_end {
                        Some(SymbolRef::Module { name: module })
                    } else {
                        Some(SymbolRef::Ident {
                            name: format!("{module}::{item}"),
                            span: *span,
                        })
                    }
                } else {
                    Some(SymbolRef::Ident {
                        name: name.clone(),
                        span: *span,
                    })
                }
            }
            HirExpressionKind::EnumVariant { span, .. } => {
                if let Some((colon, type_part, variant)) = split_segments(source, *span) {
                    let type_end = span.offset() + colon;
                    if offset >= type_end {
                        Some(SymbolRef::Variant {
                            type_name: type_part,
                            name: variant,
                        })
                    } else {
                        Some(SymbolRef::Type { name: type_part })
                    }
                } else {
                    None
                }
            }
            HirExpressionKind::Struct {
                span, type_name, ..
            } => {
                let type_end = span.offset() + type_name.len();
                if offset < type_end {
                    Some(SymbolRef::Type {
                        name: type_name.clone(),
                    })
                } else {
                    None
                }
            }
            _ => None,
        };
    }
    let (_, definition) = analysis
        .result
        .references
        .range(..=offset)
        .next_back()
        .filter(|(reference_offset, definition)| {
            !definition.name.contains("::")
                && offset >= **reference_offset
                && offset < **reference_offset + definition.name.len()
        })?;
    Some(SymbolRef::Ident {
        name: definition.name.clone(),
        span: definition.span,
    })
}

fn resolve_type_position(analysis: &Analysis, offset: usize) -> Option<SymbolRef> {
    for (type_offset, name) in &analysis.result.type_positions {
        let span = (*type_offset, type_offset + name.len());
        if span_contains(span, offset) {
            return Some(SymbolRef::Type { name: name.clone() });
        }
    }
    None
}

fn resolve_field_access(analysis: &Analysis, offset: usize) -> Option<SymbolRef> {
    for access in analysis.result.field_accesses.values() {
        let name_start = access.span.offset() + access.span.len() - access.name.len();
        if offset >= name_start
            && span_contains(
                (
                    access.span.offset(),
                    access.span.offset() + access.span.len(),
                ),
                offset,
            )
        {
            return Some(SymbolRef::Field {
                name: access.name.clone(),
                object_type: access.object_type.clone(),
            });
        }
    }
    None
}

pub(crate) fn target_definition(analysis: &Analysis, target: &SymbolRef) -> Option<Definition> {
    match target {
        SymbolRef::Ident { name, span } => analysis
            .result
            .references
            .get(&span.offset())
            .cloned()
            .or_else(|| {
                analysis
                    .result
                    .definitions
                    .get(name)?
                    .iter()
                    .find(|definition| definition.span == *span)
                    .cloned()
            }),
        SymbolRef::Type { name, .. } => find_type_definition(analysis, name),
        SymbolRef::Field {
            object_type, name, ..
        } => {
            let type_name = type_name(object_type)?;
            analysis
                .result
                .items
                .iter()
                .find_map(|item| match &item.kind {
                    HirItemKind::Struct(structure) if structure.name == type_name => structure
                        .fields
                        .iter()
                        .find(|field| field.name == *name)
                        .map(|field| Definition {
                            id: 0,
                            name: name.clone(),
                            kind: DefinitionKind::Variable,
                            span: field.span,
                            scope_depth: 1,
                            type_name: Some(field.type_.to_string()),
                        }),
                    _ => None,
                })
        }
        SymbolRef::Variant {
            type_name, name, ..
        } => analysis
            .result
            .items
            .iter()
            .find_map(|item| match &item.kind {
                HirItemKind::Enum(enumeration) if enumeration.name == *type_name => enumeration
                    .variants
                    .iter()
                    .find(|variant| variant.name == *name)
                    .map(|variant| Definition {
                        id: 0,
                        name: name.clone(),
                        kind: DefinitionKind::Enum,
                        span: variant.span,
                        scope_depth: 1,
                        type_name: None,
                    }),
                _ => None,
            }),
        SymbolRef::Module { name } => Some(Definition {
            id: 0,
            name: name.clone(),
            kind: DefinitionKind::Struct,
            span: SourceSpan::from(0..0),
            scope_depth: 1,
            type_name: None,
        }),
    }
}

fn find_type_definition(analysis: &Analysis, name: &str) -> Option<Definition> {
    analysis
        .result
        .definitions
        .get(name)?
        .iter()
        .find(|definition| {
            matches!(
                definition.kind,
                DefinitionKind::Struct | DefinitionKind::Enum | DefinitionKind::TupleStruct
            )
        })
        .cloned()
}

impl Backend {
    pub(crate) async fn definition_location(
        &self,
        target: &SymbolRef,
        analysis: &Analysis,
    ) -> Option<Location> {
        match target {
            SymbolRef::Ident { name, .. } => {
                if let Some((module, item)) = name.split_once("::") {
                    self.module_item_location(module, item).await
                } else {
                    let definition = target_definition(analysis, target)?;
                    self.location_for_definition(&definition).await
                }
            }
            SymbolRef::Type { name, .. } => {
                let definition = find_type_definition(analysis, name)?;
                self.location_for_definition(&definition).await
            }
            SymbolRef::Field {
                object_type, name, ..
            } => {
                let type_name = type_name(object_type)?;
                self.field_location(analysis, type_name, name).await
            }
            SymbolRef::Variant {
                type_name, name, ..
            } => self.variant_location(analysis, type_name, name).await,
            SymbolRef::Module { name, .. } => {
                let path = self.state.read().await.modules.get(name)?.clone();
                Some(Location::new(
                    Url::from_file_path(&path).ok()?,
                    Range::new(Position::new(0, 0), Position::new(0, 0)),
                ))
            }
        }
    }

    async fn module_item_location(&self, module: &str, item: &str) -> Option<Location> {
        let module_path = self.state.read().await.modules.get(module)?.clone();
        let publics = self.state.read().await.publics.clone();
        let (name, public) = publics.iter().find(|(name, public)| {
            *name == item && crate::backend::workspace::same_file(&public.path, &module_path)
        })?;
        self.location_for_symbol(&public.path, public.span, name)
            .await
    }

    async fn location_for_definition(&self, definition: &Definition) -> Option<Location> {
        let publics = self.state.read().await.publics.clone();
        for (name, public) in &publics {
            if public.span == definition.span {
                return self
                    .location_for_symbol(&public.path, public.span, name)
                    .await;
            }
        }
        for candidate in self.analyses().await {
            let found = candidate
                .result
                .definitions
                .values()
                .flatten()
                .any(|candidate_definition| candidate_definition.span == definition.span);
            if found {
                let (start, end) = name_range(
                    &candidate.source,
                    (
                        definition.span.offset(),
                        definition.span.offset() + definition.span.len(),
                    ),
                    &definition.name,
                );
                return Some(Location::new(
                    Url::from_file_path(&candidate.path).ok()?,
                    span_range(&candidate.line_index, start, end - start),
                ));
            }
        }
        None
    }

    async fn field_location(
        &self,
        analysis: &Analysis,
        type_name: &str,
        name: &str,
    ) -> Option<Location> {
        let others = self.analyses().await;
        let mut candidates = vec![analysis];
        for candidate in &others {
            if candidate.path != analysis.path {
                candidates.push(candidate);
            }
        }
        if let Some(candidate) = candidates.into_iter().next() {
            let field = candidate
                .result
                .items
                .iter()
                .find_map(|item| match &item.kind {
                    HirItemKind::Struct(structure) if structure.name == type_name => {
                        structure.fields.iter().find(|field| field.name == name)
                    }
                    _ => None,
                })?;
            let (start, end) = name_range(
                &candidate.source,
                (field.span.offset(), field.span.offset() + field.span.len()),
                name,
            );
            Some(Location::new(
                Url::from_file_path(&candidate.path).ok()?,
                span_range(&candidate.line_index, start, end - start),
            ))
        } else {
            None
        }
    }

    async fn variant_location(
        &self,
        analysis: &Analysis,
        type_name: &str,
        name: &str,
    ) -> Option<Location> {
        let others = self.analyses().await;
        let mut candidates = vec![analysis];
        for candidate in &others {
            if candidate.path != analysis.path {
                candidates.push(candidate);
            }
        }
        if let Some(candidate) = candidates.into_iter().next() {
            let variant = candidate
                .result
                .items
                .iter()
                .find_map(|item| match &item.kind {
                    HirItemKind::Enum(enumeration) if enumeration.name == type_name => enumeration
                        .variants
                        .iter()
                        .find(|variant| variant.name == name),
                    _ => None,
                })?;
            let (start, end) = name_range(
                &candidate.source,
                (
                    variant.span.offset(),
                    variant.span.offset() + variant.span.len(),
                ),
                name,
            );
            Some(Location::new(
                Url::from_file_path(&candidate.path).ok()?,
                span_range(&candidate.line_index, start, end - start),
            ))
        } else {
            None
        }
    }

    pub(crate) async fn location_for_symbol(
        &self,
        path: &Path,
        span: SourceSpan,
        name: &str,
    ) -> Option<Location> {
        let uri = Url::from_file_path(path).ok()?;
        if let Some(analysis) = self.analysis(&uri).await {
            let (start, end) = name_range(
                &analysis.source,
                (span.offset(), span.offset() + span.len()),
                name,
            );
            return Some(Location::new(
                uri,
                span_range(&analysis.line_index, start, end - start),
            ));
        }
        let source = self.state.read().await.vfs.source(path)?.to_string();
        let line_index = LineIndex::new(&source);
        let (start, end) = name_range(&source, (span.offset(), span.offset() + span.len()), name);
        Some(Location::new(
            uri,
            span_range(&line_index, start, end - start),
        ))
    }
}
