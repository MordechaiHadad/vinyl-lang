use std::collections::HashMap;

use tower_lsp::lsp_types::*;
use vinyl_typecheck::hir::{HirExpressionKind, HirFunction, HirItemKind};
use vinyl_typecheck::{Definition, DefinitionKind, TypeckResult};

use crate::backend::state::{Analysis, Backend};
use crate::position::{offset_at, span_range};
use crate::text::{extract_type_from_span, name_range};

impl Backend {
    pub(crate) async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> tower_lsp::jsonrpc::Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        let Some(analysis) = self.analysis(&uri).await else {
            return Ok(None);
        };
        let offset = offset_at(
            &analysis.line_index,
            params.text_document_position_params.position,
        );
        let Some(target) = symbol_at(&analysis, offset) else {
            return Ok(None);
        };
        let Some(target_analysis) = self
            .analyses()
            .await
            .into_iter()
            .find(|candidate| contains_definition(candidate, &target))
        else {
            return Ok(None);
        };
        let Some(definition) = find_definition(&target_analysis, &target) else {
            return Ok(None);
        };
        let (start, end) = name_range(
            &target_analysis.source,
            (definition.span.offset(), definition.span.offset() + definition.span.len()),
            &definition.name,
        );
        Ok(Some(GotoDefinitionResponse::Scalar(Location::new(
            Url::from_file_path(&target_analysis.path).unwrap_or(uri),
            span_range(&target_analysis.line_index, start, end - start),
        ))))
    }

    pub(crate) async fn references(
        &self,
        params: ReferenceParams,
    ) -> tower_lsp::jsonrpc::Result<Option<Vec<Location>>> {
        let uri = params.text_document_position.text_document.uri;
        let Some(analysis) = self.analysis(&uri).await else {
            return Ok(None);
        };
        let offset = offset_at(&analysis.line_index, params.text_document_position.position);
        let Some(target) = symbol_at(&analysis, offset) else {
            return Ok(Some(Vec::new()));
        };
        let locations = self.workspace_locations(&target).await;
        Ok(Some(locations))
    }

    pub(crate) async fn rename(
        &self,
        params: RenameParams,
    ) -> tower_lsp::jsonrpc::Result<Option<WorkspaceEdit>> {
        let uri = params.text_document_position.text_document.uri;
        let Some(analysis) = self.analysis(&uri).await else {
            return Ok(None);
        };
        let offset = offset_at(&analysis.line_index, params.text_document_position.position);
        let Some(target) = symbol_at(&analysis, offset) else {
            return Ok(None);
        };
        let mut changes: HashMap<Url, Vec<TextEdit>> = HashMap::new();
        for location in self.workspace_locations(&target).await {
            changes
                .entry(location.uri)
                .or_default()
                .push(TextEdit::new(location.range, params.new_name.clone()));
        }
        Ok(Some(WorkspaceEdit {
            changes: Some(changes),
            ..WorkspaceEdit::default()
        }))
    }
}

fn symbol_at(analysis: &Analysis, offset: usize) -> Option<Definition> {
    for definitions in analysis.result.definitions.values() {
        if let Some(definition) = definitions.iter().find(|d| {
            !d.name.contains("::")
                && span_contains(
                    name_range(
                        &analysis.source,
                        (d.span.offset(), d.span.offset() + d.span.len()),
                        &d.name,
                    ),
                    offset,
                )
        }) {
            return Some(definition.clone());
        }
    }
    let (_, expr) = analysis.result.expr_at_pos.range(..=offset).next_back()?;
    if matches!(expr.kind, HirExpressionKind::Ident(..))
        && span_contains((expr.span.offset(), expr.span.offset() + expr.span.len()), offset)
        && let Some(definition) = analysis.result.references.get(&expr.span.offset())
    {
        return Some(definition.clone());
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
    Some(definition.clone())
}

fn span_contains((start, end): (usize, usize), offset: usize) -> bool {
    offset >= start && offset < end
}

fn contains_definition(analysis: &Analysis, target: &Definition) -> bool {
    analysis
        .result
        .definitions
        .values()
        .flatten()
        .any(|d| d.span == target.span && !d.name.contains("::"))
}

fn find_definition(analysis: &Analysis, target: &Definition) -> Option<Definition> {
    analysis
        .result
        .definitions
        .values()
        .flatten()
        .find(|d| d.span == target.span && !d.name.contains("::"))
        .cloned()
}

pub(crate) fn definition_detail(
    definition: &Definition,
    result: &TypeckResult,
    source: &str,
) -> Option<String> {
    match definition.kind {
        DefinitionKind::Function => result.items.iter().find_map(|item| match &item.kind {
            HirItemKind::Function(f) if f.name == definition.name => {
                Some(function_signature(f, source))
            }
            _ => None,
        }),
        DefinitionKind::Struct => result.items.iter().find_map(|item| match &item.kind {
            HirItemKind::Struct(s) if s.name == definition.name => {
                let fields: Vec<_> = s
                    .fields
                    .iter()
                    .map(|f| format!("{}: {}", f.name, f.type_))
                    .collect();
                Some(format!("struct {} {{ {} }}", s.name, fields.join(", ")))
            }
            _ => None,
        }),
        DefinitionKind::Enum => result.items.iter().find_map(|item| match &item.kind {
            HirItemKind::Enum(e) if e.name == definition.name => {
                let variants: Vec<_> = e.variants.iter().map(|v| v.name.clone()).collect();
                Some(format!("enum {} {{ {} }}", e.name, variants.join(", ")))
            }
            _ => None,
        }),
        DefinitionKind::TupleStruct => result.items.iter().find_map(|item| match &item.kind {
            HirItemKind::TupleStruct(t) if t.name == definition.name => {
                let types: Vec<_> = t.types.iter().map(|t| t.to_string()).collect();
                Some(format!("struct {}({})", t.name, types.join(", ")))
            }
            _ => None,
        }),
        DefinitionKind::Parameter => extract_type_from_span(
            source,
            definition.span.offset(),
            definition.span.len(),
            false,
        )
        .map(|type_name| format!("{}: {}", definition.name, type_name)),
        DefinitionKind::Variable => {
            let type_text = extract_type_from_span(
                source,
                definition.span.offset(),
                definition.span.len(),
                true,
            )
            .or_else(|| definition.type_name.clone());
            type_text.map(|type_name| format!("{}: {}", definition.name, type_name))
        }
    }
}

fn function_signature(function: &HirFunction, source: &str) -> String {
    let params: Vec<_> = function
        .params
        .iter()
        .map(|p| {
            let original_type =
                extract_type_from_span(source, p.span.offset(), p.span.len(), false)
                    .unwrap_or_else(|| p.type_.to_string());
            format!("{}: {}", p.name, original_type)
        })
        .collect();
    let span_offset = function.span.offset();
    let span_len = function.span.len();
    let span_end = span_offset.checked_add(span_len).unwrap_or(0);
    let text = if span_end <= source.len() {
        &source[span_offset..span_end]
    } else {
        return format!("fn {}: {}", function.name, function.return_type);
    };
    let paren_close = text.find(')').unwrap_or(0);
    let brace_open = text.find('{').unwrap_or(text.len());
    let return_type = text[paren_close + 1..brace_open].trim();
    let return_type = if let Some(stripped_return_type) = return_type.strip_prefix(':') {
        stripped_return_type.trim().to_string()
    } else {
        function.return_type.to_string()
    };
    format!(
        "fn {}({}): {}",
        function.name,
        params.join(", "),
        return_type
    )
}
