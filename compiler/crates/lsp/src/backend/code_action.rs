use std::collections::HashMap;

use line_index::LineIndex;
use tower_lsp::lsp_types::*;

use crate::position::{full_range, offset_at};
use crate::backend::state::Backend;
use crate::text::{current_imports, import_edit_range, word_prefix};
use crate::backend::workspace::{non_canonical_key, relative_import_path, same_file};

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

        let Ok(formatted) = vinyl_formatter::format_source(&source) else {
            return Ok(None);
        };
        let source_line_index = LineIndex::new(&source);
        actions.push(CodeActionOrCommand::CodeAction(CodeAction {
            title: "Format document".to_string(),
            kind: Some(CodeActionKind::SOURCE_FIX_ALL),
            diagnostics: None,
            edit: Some(WorkspaceEdit {
                changes: Some(HashMap::from([(
                    uri.clone(),
                    vec![TextEdit::new(full_range(&source_line_index), formatted)],
                )])),
                ..WorkspaceEdit::default()
            }),
            command: None,
            is_preferred: Some(true),
            disabled: None,
            data: None,
        }));

        let cursor_offset = offset_at(&source_line_index, params.range.start);
        let prefix = word_prefix(&source, cursor_offset);
        if !prefix.is_empty() {
            let analysis = self.analysis(&uri).await;
            let is_local = analysis
                .as_ref()
                .is_some_and(|a| a.result.definitions.keys().any(|k| k == &prefix));
            if !is_local {
                let existing_imports = current_imports(&source);
                if let Some(resolver) = &state.resolver {
                    let current_path = uri.to_file_path().ok();
                    let workspace_root = state.workspace_root.as_deref().unwrap_or(resolver.root());
                    for info in resolver.all_modules().values() {
                        if current_path
                            .as_ref()
                            .is_some_and(|p| same_file(p, &info.file_path))
                        {
                            continue;
                        }
                        let cache_key =
                            non_canonical_key(&info.file_path, resolver, workspace_root);
                        let Some(module_analysis) = state.cache.get(&cache_key) else {
                            continue;
                        };
                        let import_path = current_path
                            .as_ref()
                            .map(|p| relative_import_path(p, &info.file_path, resolver))
                            .unwrap_or_else(|| info.import_name.clone());
                        if existing_imports.contains(&import_path)
                            || existing_imports.contains(&info.import_name)
                        {
                            continue;
                        }
                        if module_analysis.result.definitions.contains_key(&prefix) {
                            let line_index = LineIndex::new(&source);
                            let edit_range = import_edit_range(&line_index, &source);
                            let title = format!("Add import `{import_path}`");
                            actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                                title,
                                kind: Some(CodeActionKind::QUICKFIX),
                                diagnostics: None,
                                edit: Some(WorkspaceEdit {
                                    changes: Some(HashMap::from([(
                                        uri.clone(),
                                        vec![TextEdit::new(
                                            edit_range,
                                            format!("import {import_path};\n"),
                                        )],
                                    )])),
                                    ..WorkspaceEdit::default()
                                }),
                                command: None,
                                is_preferred: Some(false),
                                disabled: None,
                                data: None,
                            }));
                        }
                    }
                }
            }
        }
        drop(state);

        Ok(Some(actions))
    }

    pub(crate) async fn format(&self, uri: Url) -> tower_lsp::jsonrpc::Result<Option<Vec<TextEdit>>> {
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
        let line_index = LineIndex::new(&source);
        Ok(Some(vec![TextEdit::new(
            full_range(&line_index),
            formatted,
        )]))
    }
}
