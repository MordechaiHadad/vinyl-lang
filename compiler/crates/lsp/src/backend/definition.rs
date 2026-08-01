use std::collections::HashMap;

use line_index::LineIndex;
use tower_lsp::lsp_types::*;
use vinyl_typecheck::hir::{HirFunction, HirItemKind};
use vinyl_typecheck::{Definition, DefinitionKind, TypeckResult};

use crate::position::{offset_at, span_range};
use crate::backend::state::Backend;
use crate::text::extract_type_from_span;

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
        let Some((_, definition)) = analysis
            .result
            .references
            .range(..=offset)
            .next_back()
            .filter(|(reference_offset, definition)| {
                offset < **reference_offset + definition.name.len()
            })
        else {
            return Ok(None);
        };
        let target = definition.clone();
        let target_name = target.name.rsplit("::").next().unwrap_or(&target.name);
        let target_path = self
            .analyses()
            .await
            .into_iter()
            .find(|candidate| {
                candidate
                    .result
                    .definitions
                    .get(target_name)
                    .is_some_and(|definitions| {
                        definitions.iter().any(|item| item.span == target.span)
                    })
            })
            .map(|candidate| candidate.path.clone())
            .unwrap_or_else(|| uri.to_file_path().unwrap_or_default());
        let target_source = self
            .state
            .read()
            .await
            .vfs
            .source(&target_path)
            .unwrap_or_default();
        let target_line_index = LineIndex::new(&target_source);
        Ok(Some(GotoDefinitionResponse::Scalar(Location::new(
            Url::from_file_path(&target_path).unwrap_or(uri),
            span_range(
                &target_line_index,
                definition.span.offset(),
                definition.span.len(),
            ),
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
        let Some((_, target)) = analysis.result.references.range(..=offset).next_back() else {
            return Ok(Some(Vec::new()));
        };
        let locations = self.workspace_locations(target).await;
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
        let Some((_, target)) = analysis.result.references.range(..=offset).next_back() else {
            return Ok(None);
        };
        let mut changes: HashMap<Url, Vec<TextEdit>> = HashMap::new();
        for location in self.workspace_locations(target).await {
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
