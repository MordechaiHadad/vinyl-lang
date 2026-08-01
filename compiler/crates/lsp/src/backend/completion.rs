use std::path::Path;

use line_index::LineIndex;
use tower_lsp::lsp_types::*;
use vinyl_resolver::Resolver;
use vinyl_typecheck::hir::HirItemKind;
use vinyl_typecheck::DefinitionKind;

use crate::consts::KEYWORDS;
use crate::backend::definition::definition_detail;
use crate::position::{offset_at, position_at};
use crate::backend::state::{Analysis, Backend, State};
use crate::text::{
    current_imports, detect_import_prefix, import_edit_range, module_ref_prefix, word_before_colon,
    word_prefix,
};
use crate::backend::workspace::{non_canonical_key, relative_import_path, same_file};

impl Backend {
    pub(crate) async fn completion(
        &self,
        params: CompletionParams,
    ) -> tower_lsp::jsonrpc::Result<Option<CompletionResponse>> {
        let uri = &params.text_document_position.text_document.uri;
        let Some(path) = uri.to_file_path().ok() else {
            return Ok(None);
        };
        let analysis = self.analysis(uri).await;

        let state = self.state.read().await;
        let current_source = state.vfs.source(&path).unwrap_or_default();
        let current_line_index = LineIndex::new(&current_source);
        let offset = offset_at(&current_line_index, params.text_document_position.position);
        let prefix = word_prefix(&current_source, offset);
        let import_prefix_info = detect_import_prefix(&current_source, offset);
        let in_import_context = import_prefix_info.is_some();
        let module_ref_simple = module_ref_prefix(&current_source, offset);
        let is_colon_trigger =
            params.context.and_then(|c| c.trigger_character).as_deref() == Some(":");

        let mut items = if !in_import_context && module_ref_simple.is_none() {
            analysis
                .as_deref()
                .map(|analysis| local_completions(analysis, &prefix))
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        if let Some(resolver) = &state.resolver {
            let existing_imports = current_imports(&current_source);
            let workspace_root = state.workspace_root.as_deref().unwrap_or(resolver.root());

            let has_module_ref = module_ref_simple.as_ref().and_then(|(name, _)| {
                existing_imports
                    .iter()
                    .any(|imp| imp == name || imp.ends_with(&format!("::{name}")))
                    .then_some(name.clone())
            });

            if is_colon_trigger && !in_import_context && module_ref_simple.is_none() {
                let has_pending_module = word_before_colon(&current_source, offset).is_some_and(
                    |word| {
                        resolver
                            .all_modules()
                            .values()
                            .any(|info| info.import_name == word)
                    },
                );
                if !has_pending_module {
                    return Ok(Some(CompletionResponse::Array(Vec::new())));
                }
            }

            if let Some(module_name) = has_module_ref {
                let (_, partial) = module_ref_simple.as_ref().unwrap();
                items.extend(module_ref_completions(
                    &state,
                    resolver,
                    &module_name,
                    partial,
                    &current_line_index,
                    offset,
                    workspace_root,
                ));
            } else {
                items.extend(auto_import_completions(
                    &state,
                    resolver,
                    &path,
                    &prefix,
                    &current_source,
                    &current_line_index,
                    offset,
                ));
                if let Some((prefix_count, partial)) = import_prefix_info {
                    items.extend(import_prefix_completions(
                        resolver,
                        &path,
                        prefix_count,
                        &partial,
                        &current_line_index,
                        offset,
                    ));
                }
            }
        }
        drop(state);

        Ok(Some(CompletionResponse::Array(items)))
    }
}

fn local_completions(analysis: &Analysis, prefix: &str) -> Vec<CompletionItem> {
    let mut items = Vec::new();
    for (name, definitions) in &analysis.result.definitions {
        if !name.starts_with(prefix) {
            continue;
        }
        let Some(definition) = definitions.first() else {
            continue;
        };
        if definition.name == "main" && matches!(definition.kind, DefinitionKind::Function) {
            continue;
        }
        let kind = match definition.kind {
            DefinitionKind::Function => CompletionItemKind::FUNCTION,
            DefinitionKind::Struct => CompletionItemKind::STRUCT,
            DefinitionKind::Enum => CompletionItemKind::ENUM,
            DefinitionKind::TupleStruct => CompletionItemKind::STRUCT,
            DefinitionKind::Variable => CompletionItemKind::VARIABLE,
            DefinitionKind::Parameter => CompletionItemKind::VARIABLE,
        };
        let detail = definition_detail(definition, &analysis.result, &analysis.source);
        items.push(CompletionItem {
            label: name.clone(),
            kind: Some(kind),
            detail,
            ..CompletionItem::default()
        });
    }

    for (keyword, kind) in KEYWORDS {
        if keyword.starts_with(prefix) {
            items.push(CompletionItem {
                label: keyword.to_string(),
                kind: Some(*kind),
                ..CompletionItem::default()
            });
        }
    }
    items
}

fn module_ref_completions(
    state: &State,
    resolver: &Resolver,
    module_name: &str,
    partial: &str,
    current_line_index: &LineIndex,
    offset: usize,
    workspace_root: &Path,
) -> Vec<CompletionItem> {
    let mut items = Vec::new();
    for info in resolver.all_modules().values() {
        if info.import_name != module_name {
            continue;
        }
        let cache_key = non_canonical_key(&info.file_path, resolver, workspace_root);
        let Some(module_analysis) = state.cache.get(&cache_key) else {
            continue;
        };
        for (name, definitions) in &module_analysis.result.definitions {
            if !name.starts_with(partial) || name.contains("::") {
                continue;
            }
            let Some(definition) = definitions.first() else {
                continue;
            };
            let is_public = module_analysis.result.items.iter().any(|item| {
                let (item_name, item_public) = match &item.kind {
                    HirItemKind::Function(f) => (&f.name, f.public),
                    HirItemKind::Struct(s) => (&s.name, s.public),
                    HirItemKind::TupleStruct(t) => (&t.name, t.public),
                    HirItemKind::Enum(e) => (&e.name, e.public),
                };
                item_name == name && item_public
            });
            if !is_public {
                continue;
            }
            let kind = match definition.kind {
                DefinitionKind::Function => CompletionItemKind::FUNCTION,
                DefinitionKind::Struct => CompletionItemKind::STRUCT,
                DefinitionKind::Enum => CompletionItemKind::ENUM,
                DefinitionKind::TupleStruct => CompletionItemKind::STRUCT,
                _ => continue,
            };
            let detail = definition_detail(
                definition,
                &module_analysis.result,
                &module_analysis.source,
            );
            let cursor_pos = position_at(current_line_index, offset);
            let edit_range = Range::new(
                position_at(current_line_index, offset.saturating_sub(partial.len())),
                cursor_pos,
            );
            items.push(CompletionItem {
                label: name.clone(),
                kind: Some(kind),
                detail,
                text_edit: Some(CompletionTextEdit::Edit(TextEdit::new(
                    edit_range, name.clone(),
                ))),
                ..CompletionItem::default()
            });
        }
    }
    items
}

fn auto_import_completions(
    state: &State,
    resolver: &Resolver,
    path: &Path,
    prefix: &str,
    current_source: &str,
    current_line_index: &LineIndex,
    offset: usize,
) -> Vec<CompletionItem> {
    let workspace_root = state.workspace_root.as_deref().unwrap_or(resolver.root());
    let existing_imports = current_imports(current_source);
    let mut items = Vec::new();
    for info in resolver.all_modules().values() {
        if same_file(path, &info.file_path) {
            continue;
        }
        let cache_key = non_canonical_key(&info.file_path, resolver, workspace_root);
        let Some(module_analysis) = state.cache.get(&cache_key) else {
            continue;
        };
        let import_path = relative_import_path(path, &info.file_path, resolver);
        let already_imported = existing_imports.iter().any(|imp| {
            imp == &info.import_name || imp.ends_with(&format!("::{}", info.import_name))
        });
        if already_imported {
            continue;
        }
        for (name, definitions) in &module_analysis.result.definitions {
            if !name.starts_with(prefix) || name.contains("::") {
                continue;
            }
            let Some(definition) = definitions.first() else {
                continue;
            };
            let kind = match definition.kind {
                DefinitionKind::Function => CompletionItemKind::FUNCTION,
                DefinitionKind::Struct => CompletionItemKind::STRUCT,
                DefinitionKind::Enum => CompletionItemKind::ENUM,
                DefinitionKind::TupleStruct => CompletionItemKind::STRUCT,
                _ => continue,
            };
            let detail = definition_detail(
                definition,
                &module_analysis.result,
                &module_analysis.source,
            );
            let detail = Some(
                detail
                    .map(|d| format!("{d} (from {import_path})"))
                    .unwrap_or_else(|| format!("(from {import_path})")),
            );
            let import_name = &info.import_name;
            let qualified = format!("{import_name}::{name}");
            let cursor_pos = position_at(current_line_index, offset);
            let edit_range = Range::new(
                position_at(current_line_index, offset.saturating_sub(prefix.len())),
                cursor_pos,
            );
            let import_edit = TextEdit::new(
                import_edit_range(current_line_index, current_source),
                format!("import {import_path};\n"),
            );
            items.push(CompletionItem {
                label: qualified.clone(),
                kind: Some(kind),
                detail,
                text_edit: Some(CompletionTextEdit::Edit(TextEdit::new(
                    edit_range, qualified,
                ))),
                additional_text_edits: Some(vec![import_edit]),
                ..CompletionItem::default()
            });
        }
    }

    items
}

fn import_prefix_completions(
    resolver: &Resolver,
    path: &Path,
    prefix_count: usize,
    partial: &str,
    current_line_index: &LineIndex,
    offset: usize,
) -> Vec<CompletionItem> {
    let mut items = Vec::new();
    let mut dir = path.parent().unwrap_or(Path::new("")).to_path_buf();
    for _ in 1..prefix_count {
        dir.push("..");
    }
    let dir = std::fs::canonicalize(&dir).unwrap_or(dir);
    let files = resolver.list_vn_files(&dir).unwrap_or_default();
    for file_path in &files {
        if file_path.parent() != Some(&dir) {
            continue;
        }
        let stem = match file_path.file_stem() {
            Some(s) => s.to_string_lossy().to_string(),
            None => continue,
        };
        if !stem.starts_with(partial) {
            continue;
        }
        items.push(CompletionItem {
            label: stem.clone(),
            kind: Some(CompletionItemKind::MODULE),
            detail: Some("module".to_string()),
            text_edit: Some(CompletionTextEdit::Edit(TextEdit::new(
                Range::new(
                    position_at(
                        current_line_index,
                        offset.saturating_sub(partial.len()),
                    ),
                    position_at(current_line_index, offset),
                ),
                stem,
            ))),
            ..CompletionItem::default()
        });
    }
    items
}
