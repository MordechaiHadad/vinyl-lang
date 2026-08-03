use std::collections::HashMap;

use tower_lsp::lsp_types::*;
use vinyl_typecheck::hir::{HirFunction, HirItemKind};
use vinyl_typecheck::{Definition, DefinitionKind, TypeckResult};

use crate::backend::state::Backend;
use crate::backend::symbol::{resolve_symbol, target_definition};
use crate::position::offset_at;
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
        let Some(target) = resolve_symbol(&analysis, offset) else {
            return Ok(None);
        };
        let Some(location) = self.definition_location(&target, &analysis).await else {
            return Ok(None);
        };
        Ok(Some(GotoDefinitionResponse::Scalar(location)))
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
        let Some(target) = resolve_symbol(&analysis, offset) else {
            return Ok(Some(Vec::new()));
        };
        let Some(definition) = target_definition(&analysis, &target) else {
            return Ok(Some(Vec::new()));
        };
        let locations = self.workspace_locations(&definition).await;
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
        let Some(target) = resolve_symbol(&analysis, offset) else {
            return Ok(None);
        };
        let Some(definition) = target_definition(&analysis, &target) else {
            return Ok(None);
        };
        let mut changes: HashMap<Url, Vec<TextEdit>> = HashMap::new();
        for location in self.workspace_locations(&definition).await {
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
                let module = definition
                    .name
                    .contains("::")
                    .then(|| definition.name.split("::").next().unwrap().to_string());
                let mut function = f.clone();
                function.name = definition.name.rsplit("::").next().unwrap().to_string();
                let mut signature = function_signature(&function, source);
                if let Some(module) = module {
                    signature.push_str(&format!(" (from {module})"));
                }
                Some(signature)
            }
            _ => None,
        }),
        DefinitionKind::Struct => result.items.iter().find_map(|item| match &item.kind {
            HirItemKind::Struct(s) if s.name == definition.name => {
                let fields: Vec<_> = s
                    .fields
                    .iter()
                    .map(|f| {
                        let type_name =
                            extract_type_from_span(source, f.span.offset(), f.span.len(), false)
                                .unwrap_or_else(|| f.type_.to_string());
                        format!("{}: {}", f.name, type_name)
                    })
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
        DefinitionKind::TypeAlias => result.items.iter().find_map(|item| match &item.kind {
            HirItemKind::TypeAlias(a) if a.name == definition.name => {
                Some(format!("type {} = {}", a.name, a.type_))
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

pub(crate) fn function_signature(function: &HirFunction, source: &str) -> String {
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
