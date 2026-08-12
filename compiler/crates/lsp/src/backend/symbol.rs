use std::path::Path;

use line_index::LineIndex;
use tower_lsp::lsp_types::*;
use vinyl_typecheck::hir::{HirExpressionKind, HirItemKind, HirPatternKind, Type};
use vinyl_typecheck::{Definition, DefinitionKind, SourceSpan};

use crate::backend::state::{Analysis, Backend};
use crate::position::span_range;
use crate::text::{name_range, name_span};

#[derive(Debug)]
pub(crate) enum SymbolRef {
    Ident { name: String, span: SourceSpan },
    Type { name: String },
    Field { name: String, object_type: Type },
    Variant { type_name: String, name: String },
    Module { name: String },
}

#[derive(Clone, Copy)]
enum MemberKind {
    Field,
    Variant,
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

/// Splits a qualified span into its final item and module prefix.
fn split_segments(source: &str, span: SourceSpan) -> Option<(usize, String, String)> {
    let start = span.offset();
    let end = (start + span.len()).min(source.len());
    let text = source.get(start..end)?;
    let item_start = text.rfind("::")? + 2;
    let item = text[item_start..]
        .split(|character: char| !character.is_alphanumeric() && character != '_')
        .next()
        .unwrap_or("");
    Some((
        item_start - 2,
        text[..item_start - 2].to_string(),
        item.to_string(),
    ))
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
                && name_span(
                    source,
                    (
                        definition.span.offset(),
                        definition.span.offset() + definition.span.len(),
                    ),
                    &definition.name,
                )
                .is_some_and(|(start, end)| offset >= start && offset < end)
        }) {
            return Some(SymbolRef::Ident {
                name: definition.name.clone(),
                span: definition.span,
            });
        }
    }
    if let Some(symbol) = resolve_pattern(analysis, offset) {
        return Some(symbol);
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
            HirExpressionKind::EnumVariant { span, type_name, .. } => {
                variant_or_type_symbol(source, *span, type_name, offset)
            }
            HirExpressionKind::Struct { span, type_name, .. } => {
                struct_type_symbol(source, *span, type_name, offset)
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

fn resolve_pattern(analysis: &Analysis, offset: usize) -> Option<SymbolRef> {
    let source = &analysis.source;
    let innermost = analysis
        .result
        .patterns_at_pos
        .values()
        .filter(|pattern| {
            let span = pattern.span();
            span_contains(
                (span.offset(), span.offset() + span.len()),
                offset,
            )
        })
        .min_by_key(|pattern| pattern.span().len())?;
    match &innermost.kind {
        HirPatternKind::EnumVariant { span, type_name, .. } => {
            variant_or_type_symbol(source, *span, type_name, offset)
        }
        HirPatternKind::Struct { span, type_name, .. } => {
            struct_type_symbol(source, *span, type_name, offset)
        }
        HirPatternKind::Tuple { .. }
        | HirPatternKind::Ident { .. }
        | HirPatternKind::Literal { .. }
        | HirPatternKind::Wildcard(_) => None,
    }
}

/// Maps a cursor offset inside an enum variant path to either the enclosing
/// type or the variant, depending on which half of the path the cursor is in.
/// Shared by expression and pattern resolution so both stay in sync.
fn variant_or_type_symbol(
    source: &str,
    span: SourceSpan,
    type_name: &str,
    offset: usize,
) -> Option<SymbolRef> {
    let (colon, _, variant) = split_segments(source, span)?;
    let variant_start = span.offset() + colon + 2;
    if offset >= variant_start {
        Some(SymbolRef::Variant {
            type_name: type_name.to_string(),
            name: variant,
        })
    } else {
        Some(SymbolRef::Type {
            name: type_name.to_string(),
        })
    }
}

/// Maps a cursor offset inside a struct literal or pattern to the type, as
/// long as the cursor is before the opening brace. Shared by expression and
/// pattern resolution.
fn struct_type_symbol(
    source: &str,
    span: SourceSpan,
    type_name: &str,
    offset: usize,
) -> Option<SymbolRef> {
    let source_text = source.get(span.offset()..)?;
    let type_end = source_text
        .find('{')
        .map(|index| span.offset() + index)
        .unwrap_or_else(|| span.offset() + type_name.len());
    (offset < type_end).then(|| SymbolRef::Type {
        name: type_name.to_string(),
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
                            scope: None,
                            type_name: Some(field.type_.to_string()),
                        }),
                    _ => None,
                })
        }
        SymbolRef::Variant { type_name, .. } => analysis
            .result
            .items
            .iter()
            .find_map(|item| match &item.kind {
                HirItemKind::Enum(enumeration) if enumeration.name == *type_name => Some(Definition {
                    id: 0,
                    name: enumeration.name.clone(),
                    kind: DefinitionKind::Enum,
                    span: enumeration.span,
                    scope_depth: 1,
                    scope: None,
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
            scope: None,
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
                self.member_location(analysis, type_name, name, MemberKind::Field)
                    .await
            }
            SymbolRef::Variant {
                type_name, name, ..
            } => {
                self.member_location(analysis, type_name, name, MemberKind::Variant)
                    .await
            }
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

    async fn member_location(
        &self,
        analysis: &Analysis,
        type_name: &str,
        name: &str,
        kind: MemberKind,
    ) -> Option<Location> {
        let local_type_name = type_name.rsplit("::").next().unwrap_or(type_name);
        let others = self.analyses().await;
        let mut candidates = vec![analysis];
        for candidate in &others {
            if candidate.path != analysis.path {
                candidates.push(candidate);
            }
        }
        for candidate in candidates {
            let hit =
                match kind {
                    MemberKind::Field => candidate.result.items.iter().find_map(|item| match &item
                        .kind
                    {
                        HirItemKind::Struct(structure) if structure.name == local_type_name => {
                            structure
                                .fields
                                .iter()
                                .find(|field| field.name == name)
                                .map(|field| (structure.span, field.span))
                        }
                        _ => None,
                    }),
                    MemberKind::Variant => {
                        candidate
                            .result
                            .items
                            .iter()
                            .find_map(|item| match &item.kind {
                                HirItemKind::Enum(enumeration)
                                    if enumeration.name == local_type_name =>
                                {
                                    enumeration
                                        .variants
                                        .iter()
                                        .find(|variant| variant.name == name)
                                        .map(|variant| (enumeration.span, variant.span))
                                }
                                _ => None,
                            })
                    }
                };
            let Some((type_span, member_span)) = hit else {
                continue;
            };
            if name_span(
                &candidate.source,
                (type_span.offset(), type_span.offset() + type_span.len()),
                local_type_name,
            )
            .is_none()
            {
                continue;
            }
            let (start, end) = name_span(
                &candidate.source,
                (
                    member_span.offset(),
                    member_span.offset() + member_span.len(),
                ),
                name,
            )?;
            return Some(Location::new(
                Url::from_file_path(&candidate.path).ok()?,
                span_range(&candidate.line_index, start, end - start),
            ));
        }
        None
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use vinyl_parser::parse_and_lower;
    use vinyl_typecheck::module::ModuleTable;

    fn analyze(source: &str) -> Analysis {
        let items = parse_and_lower(source).expect("source should parse");
        let (result, _warnings) =
            vinyl_typecheck::typeck_with_index(&items, source, "test.vn", &ModuleTable::new())
                .expect("source should typecheck");
        Analysis {
            path: PathBuf::from("test.vn"),
            source: source.to_string(),
            line_index: LineIndex::new(source),
            result,
        }
    }

    fn line_offset(source: &str, line: usize, character: usize) -> usize {
        source
            .lines()
            .take(line)
            .map(|line_text| line_text.len() + 1)
            .sum::<usize>()
            + character
    }

    #[test]
    fn resolves_enum_variant_match_pattern() {
        let source = "enum Shape { Empty, Circle(int32), Square(int32) }\n\nfn classify(s: Shape): int32 {\n    match s {\n        Shape::Circle(r) => r,\n        Shape::Square(r) => r,\n        Shape::Empty() => 0,\n    }\n}\n";
        let analysis = analyze(source);
        let offset = line_offset(source, 4, 16);
        match resolve_symbol(&analysis, offset) {
            Some(SymbolRef::Variant { name, .. }) => assert_eq!(name, "Circle"),
            other => panic!("expected Variant, got {other:?}"),
        }
    }

    #[test]
    fn resolves_struct_match_pattern() {
        let source = "struct Point { x: int32, y: int32 }\n\nfn origin(p: Point): int32 {\n    match p {\n        Point { x, y } => x,\n        _ => 0,\n    }\n}\n";
        let analysis = analyze(source);
        let offset = line_offset(source, 4, 10);
        match resolve_symbol(&analysis, offset) {
            Some(SymbolRef::Type { name }) => assert_eq!(name, "Point"),
            other => panic!("expected Type, got {other:?}"),
        }
    }

    #[test]
    fn pattern_variant_beats_ident_resolution() {
        let source = "enum Shape { Empty, Circle(int32), Square(int32) }\n\nfn classify(s: Shape): int32 {\n    match s {\n        Shape::Circle(r) => r,\n        Shape::Square(r) => r,\n        Shape::Empty() => 0,\n    }\n}\n";
        let analysis = analyze(source);
        let r_offset = line_offset(source, 4, 22);
        match resolve_symbol(&analysis, r_offset) {
            Some(SymbolRef::Ident { name, .. }) => assert_eq!(name, "r"),
            other => panic!("expected Ident, got {other:?}"),
        }
    }
}
