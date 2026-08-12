use std::collections::HashMap;

use line_index::LineIndex;
use tower_lsp::lsp_types::*;

use crate::backend::state::Backend;
use crate::backend::workspace::{is_imported, is_public_symbol, non_canonical_key, same_file};
use crate::position::{full_range, offset_at};
use crate::text::{current_imports, import_edit_range, module_ref_prefix, word_prefix};

impl Backend {
    pub(crate) async fn code_action(
        &self,
        params: CodeActionParams,
    ) -> tower_lsp::jsonrpc::Result<Option<CodeActionResponse>> {
        let uri = params.text_document.uri;
        let Some(path) = uri.to_file_path().ok() else {
            return Ok(None);
        };
        let state = self.state.read().await;
        let Some(source) = state.vfs.source(&path) else {
            return Ok(None);
        };

        let mut actions = Vec::new();

        let source_line_index = LineIndex::new(&source);
        let cursor_offset = offset_at(&source_line_index, params.range.start);
        let prefix = word_prefix(&source, cursor_offset);
        let existing_imports = current_imports(&source);
        let module_ref = module_ref_prefix(&source, cursor_offset);
        if let Some((module_name, symbol_name)) = module_ref.as_ref()
            && !symbol_name.is_empty()
            && !is_imported(&existing_imports, module_name)
            && let Some(resolver) = &state.resolver
        {
            let workspace_root = state.workspace_root.as_deref().unwrap_or(resolver.root());
            for info in resolver.all_modules().values() {
                if info.import_name != *module_name || same_file(&path, &info.file_path) {
                    continue;
                }
                let cache_key = non_canonical_key(&info.file_path, resolver, workspace_root);
                let Some(module_analysis) = state.cache.get(&cache_key) else {
                    continue;
                };
                let Some(definitions) = module_analysis.result.definitions.get(symbol_name) else {
                    continue;
                };
                if definitions.is_empty() || !is_public_symbol(module_analysis, symbol_name) {
                    continue;
                }
                let import_path = resolver.relative_import_path(&path, &info.file_path);
                actions.push(add_import_action(
                    &uri,
                    &source,
                    &format!("Add import `{import_path}`"),
                    &import_path,
                ));
            }
        }
        if module_ref.is_none() && !prefix.is_empty() {
            let analysis = self.analysis(&uri).await;
            let is_local = analysis
                .as_ref()
                .is_some_and(|a| a.result.definitions.keys().any(|k| k == &prefix));
            if !is_local && let Some(resolver) = &state.resolver {
                let current_path = uri.to_file_path().ok();
                let workspace_root = state.workspace_root.as_deref().unwrap_or(resolver.root());
                for info in resolver.all_modules().values() {
                    if current_path
                        .as_ref()
                        .is_some_and(|p| same_file(p, &info.file_path))
                    {
                        continue;
                    }
                    let cache_key = non_canonical_key(&info.file_path, resolver, workspace_root);
                    let Some(module_analysis) = state.cache.get(&cache_key) else {
                        continue;
                    };
                    let import_path = current_path
                        .as_ref()
                        .map(|p| resolver.relative_import_path(p, &info.file_path))
                        .unwrap_or_else(|| info.import_name.clone());
                    if is_imported(&existing_imports, &info.import_name) {
                        continue;
                    }
                    if module_analysis.result.definitions.contains_key(&prefix)
                        && is_public_symbol(module_analysis, &prefix)
                    {
                        actions.push(add_import_action(
                            &uri,
                            &source,
                            &format!("Add import `{import_path}`"),
                            &import_path,
                        ));
                    }
                }
            }
        }
        drop(state);

        if let Ok(formatted) = vinyl_formatter::format_source(&source)
            && formatted != source
        {
            actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                title: "Format document".to_string(),
                kind: Some(CodeActionKind::SOURCE_FIX_ALL),
                diagnostics: None,
                edit: Some(WorkspaceEdit {
                    changes: Some(HashMap::from([(
                        uri,
                        vec![TextEdit::new(full_range(&source_line_index), formatted)],
                    )])),
                    ..WorkspaceEdit::default()
                }),
                command: None,
                is_preferred: Some(false),
                disabled: None,
                data: None,
            }));
        }

        Ok(Some(actions))
    }

    pub(crate) async fn format(
        &self,
        uri: Url,
    ) -> tower_lsp::jsonrpc::Result<Option<Vec<TextEdit>>> {
        let Some(path) = uri.to_file_path().ok() else {
            return Ok(None);
        };
        let state = self.state.read().await;
        let Some(source) = state.vfs.source(&path) else {
            return Ok(None);
        };
        let formatted = match vinyl_formatter::format_source(&source) {
            Ok(formatted) => formatted,
            Err(_) => return Ok(None),
        };
        if formatted == source {
            return Ok(None);
        }
        let line_index = LineIndex::new(&source);
        Ok(Some(vec![TextEdit::new(
            full_range(&line_index),
            formatted,
        )]))
    }
}

fn add_import_action(
    uri: &Url,
    source: &str,
    title: &str,
    import_path: &str,
) -> CodeActionOrCommand {
    let line_index = LineIndex::new(source);
    CodeActionOrCommand::CodeAction(CodeAction {
        title: title.to_string(),
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: None,
        edit: Some(WorkspaceEdit {
            changes: Some(HashMap::from([(
                uri.clone(),
                vec![TextEdit::new(
                    import_edit_range(&line_index, source),
                    format!("import {import_path};\n"),
                )],
            )])),
            ..WorkspaceEdit::default()
        }),
        command: None,
        is_preferred: Some(false),
        disabled: None,
        data: None,
    })
}
