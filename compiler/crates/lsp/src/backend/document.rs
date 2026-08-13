use tower_lsp::lsp_types::*;
use vinyl_typecheck::hir::HirItemKind;

use crate::backend::definition::{definition_detail, function_signature};
use crate::backend::state::Backend;
use crate::backend::symbol::{SymbolRef, resolve_symbol, target_definition};
use crate::position::{offset_at, span_range};
use crate::text::word_prefix;

impl Backend {
    pub(crate) async fn hover(
        &self,
        params: HoverParams,
    ) -> tower_lsp::jsonrpc::Result<Option<Hover>> {
        let Some(analysis) = self
            .analysis(&params.text_document_position_params.text_document.uri)
            .await
        else {
            return Ok(None);
        };
        let offset = offset_at(
            &analysis.line_index,
            params.text_document_position_params.position,
        );
        let Some(target) = resolve_symbol(&analysis, offset) else {
            return Ok(None);
        };
        if let SymbolRef::Variant { type_name, .. } = &target {
            let local_type_name = type_name.rsplit("::").next().unwrap_or(type_name);
            let module_path = type_name.rsplit_once("::").map(|(module, _)| module);
            for candidate in self.analyses().await {
                if candidate.file_id == analysis.file_id
                    || !candidate
                        .result
                        .items
                        .iter()
                        .any(|item| matches!(&item.kind, HirItemKind::Enum(enumeration) if enumeration.name == local_type_name))
                {
                    continue;
                }
                if let Some(module_path) = module_path {
                    let Some(candidate_path) = self.file_path(candidate.file_id).await else {
                        continue;
                    };
                    if !crate::backend::symbol::path_matches_module(&candidate_path, module_path) {
                        continue;
                    }
                }
                let Some(definition) =
                    candidate
                        .result
                        .items
                        .iter()
                        .find_map(|item| match &item.kind {
                            HirItemKind::Enum(enumeration)
                                if enumeration.name == local_type_name =>
                            {
                                Some(vinyl_typecheck::Definition {
                                    id: 0,
                                    name: enumeration.name.clone(),
                                    kind: vinyl_typecheck::DefinitionKind::Enum,
                                    span: enumeration.span,
                                    scope_depth: 1,
                                    scope: None,
                                    type_name: None,
                                })
                            }
                            _ => None,
                        })
                else {
                    continue;
                };
                let Some(detail) =
                    definition_detail(&definition, &candidate.result, &candidate.source)
                else {
                    return Ok(None);
                };
                return Ok(Some(Hover {
                    contents: HoverContents::Scalar(MarkedString::String(detail)),
                    range: None,
                }));
            }
        }
        let content = match target_definition(&analysis, &target) {
            Some(definition) => {
                let source = self
                    .definition_source(&definition)
                    .await
                    .unwrap_or_else(|| analysis.source.clone());
                let Some(detail) = definition_detail(&definition, &analysis.result, &source) else {
                    return Ok(None);
                };
                detail
            }
            None => {
                let SymbolRef::Type { name } = &target else {
                    return Ok(None);
                };
                if !is_primitive_type(name) {
                    return Ok(None);
                }
                name.clone()
            }
        };
        Ok(Some(Hover {
            contents: HoverContents::Scalar(MarkedString::String(content)),
            range: None,
        }))
    }

    pub(crate) async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> tower_lsp::jsonrpc::Result<Option<DocumentSymbolResponse>> {
        #[allow(deprecated)]
        let symbols = {
            let uri = params.text_document.uri;
            let Some(analysis) = self.analysis(&uri).await else {
                return Ok(None);
            };
            analysis
                .result
                .items
                .iter()
                .map(|item| {
                    let (name, kind) = match &item.kind {
                        HirItemKind::Function(function) => (&function.name, SymbolKind::FUNCTION),
                        HirItemKind::Struct(structure) => (&structure.name, SymbolKind::STRUCT),
                        HirItemKind::TupleStruct(tuple) => (&tuple.name, SymbolKind::STRUCT),
                        HirItemKind::Enum(enumeration) => (&enumeration.name, SymbolKind::ENUM),
                        HirItemKind::TypeAlias(alias) => (&alias.name, SymbolKind::STRUCT),
                    };
                    DocumentSymbol {
                        name: name.clone(),
                        detail: None,
                        kind,
                        tags: None,
                        deprecated: None,
                        range: span_range(
                            &analysis.line_index,
                            item.span.offset(),
                            item.span.len(),
                        ),
                        selection_range: span_range(
                            &analysis.line_index,
                            item.span.offset(),
                            item.span.len(),
                        ),
                        children: None,
                    }
                })
                .collect()
        };
        Ok(Some(DocumentSymbolResponse::Nested(symbols)))
    }

    pub(crate) async fn signature_help(
        &self,
        params: SignatureHelpParams,
    ) -> tower_lsp::jsonrpc::Result<Option<SignatureHelp>> {
        let Some(analysis) = self
            .analysis(&params.text_document_position_params.text_document.uri)
            .await
        else {
            return Ok(None);
        };
        let offset = offset_at(
            &analysis.line_index,
            params.text_document_position_params.position,
        );
        let prefix = word_prefix(&analysis.source, offset);
        let signatures = analysis
            .result
            .items
            .iter()
            .filter_map(|item| match &item.kind {
                HirItemKind::Function(function) => {
                    if !prefix.is_empty() && !function.name.starts_with(&prefix) {
                        return None;
                    }
                    Some(SignatureInformation {
                        label: function_signature(function, &analysis.source),
                        documentation: None,
                        parameters: None,
                        active_parameter: None,
                    })
                }
                _ => None,
            })
            .collect();
        Ok(Some(SignatureHelp {
            signatures,
            active_signature: Some(0),
            active_parameter: Some(0),
        }))
    }
}

fn is_primitive_type(name: &str) -> bool {
    matches!(
        name,
        "unit"
            | "int"
            | "int8"
            | "int16"
            | "int32"
            | "int64"
            | "int128"
            | "isize"
            | "uint8"
            | "uint16"
            | "uint32"
            | "uint64"
            | "uint128"
            | "usize"
            | "float"
            | "float32"
            | "float64"
            | "bool"
            | "char"
            | "string"
    )
}
