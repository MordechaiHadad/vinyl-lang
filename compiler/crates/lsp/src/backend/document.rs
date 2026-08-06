use tower_lsp::lsp_types::*;
use vinyl_typecheck::hir::HirItemKind;

use crate::backend::definition::{definition_detail, function_signature};
use crate::backend::state::Backend;
use crate::backend::symbol::{resolve_symbol, target_definition, SymbolRef};
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
        let (detail, type_symbol) = match resolve_symbol(&analysis, offset) {
            Some(target) => {
                let detail = match target_definition(&analysis, &target) {
                    Some(definition) => {
                        let source = self
                            .definition_source(&definition)
                            .await
                            .unwrap_or_else(|| analysis.source.clone());
                        definition_detail(&definition, &analysis.result, &source)
                    }
                    None => None,
                };
                (detail, Some(target))
            }
            None => (None, None),
        };
        let content = if let Some(detail) = detail {
            detail
        } else if let Some(SymbolRef::Type { name }) = type_symbol {
            name
        } else if let Some(expression) = analysis
            .result
            .expr_at_pos
            .range(..=offset)
            .next_back()
            .map(|(_, expression)| expression)
            .filter(|expression| offset < expression.span.offset() + expression.span.len())
        {
            format!("type: {}", expression.type_)
        } else {
            return Ok(None);
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
