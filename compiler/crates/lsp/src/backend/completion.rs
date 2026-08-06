use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;

use line_index::LineIndex;
use tower_lsp::lsp_types::*;
use vinyl_parser::ast::item::Item;
use vinyl_resolver::resolver::{Resolver, ResolverMode};
use vinyl_typecheck::module::ModuleTable;
use vinyl_typecheck::DefinitionKind;

use crate::backend::definition::definition_detail;
use crate::backend::state::{Analysis, Backend, State};
use crate::backend::workspace::{
    add_resolved_modules, analyze_with_diagnostics, collect_modules, is_imported, is_public_symbol,
    non_canonical_key, parse_file_with_diagnostics, relative_import_path, same_file,
};
use crate::consts::{KEYWORDS, MODULE_PREFIXES};
use crate::position::{offset_at, position_at};
use crate::text::{
    current_imports, detect_import_prefix, import_edit_range, module_ref_prefix, word_before_colon,
    word_prefix,
};
use crate::vfs::LspFileSystem;

impl Backend {
    pub(crate) async fn completion(
        &self,
        params: CompletionParams,
    ) -> tower_lsp::jsonrpc::Result<Option<CompletionResponse>> {
        let uri = &params.text_document_position.text_document.uri;
        let Some(path) = uri.to_file_path().ok() else {
            return Ok(None);
        };
        let state = self.state.read().await;
        let current_source = state.vfs.source(&path).unwrap_or_default();
        let analysis = (|| {
            let (name, items) = parse_file_with_diagnostics(&state.vfs, &path).ok()?;
            analyze_with_diagnostics(&path, &name, &items, &state.module_table).ok()
        })()
        .or(self.analysis(uri).await);
        let current_line_index = LineIndex::new(&current_source);
        let offset = offset_at(&current_line_index, params.text_document_position.position)
            .min(current_source.len());
        let prefix = word_prefix(&current_source, offset);
        let import_prefix_info = detect_import_prefix(&current_source, offset);
        let in_import_context = import_prefix_info.is_some();
        let module_ref_simple = module_ref_prefix(&current_source, offset)
            .map(|(module_name, _)| (module_name, prefix.clone()));
        let is_colon_trigger =
            params.context.and_then(|c| c.trigger_character).as_deref() == Some(":");

        let source_bytes = current_source.as_bytes();
        let field_access_dot = field_access_context(&current_source, offset);
        let variant_trigger =
            (offset >= 2 && source_bytes[offset - 2] == b':' && source_bytes[offset - 1] == b':')
                || (offset > 0
                    && offset < source_bytes.len()
                    && source_bytes[offset - 1] == b':'
                    && source_bytes[offset] == b':');
        if !in_import_context {
            if variant_trigger
                && let Some(items) =
                    variant_completions(&state, &path, &current_source, offset, &prefix)
            {
                drop(state);
                return Ok(Some(CompletionResponse::Array(items)));
            }
            if let Some(dot_index) = field_access_dot {
                let items = field_completions(
                    &state,
                    &path,
                    &current_source,
                    offset,
                    dot_index,
                    &prefix,
                )
                .unwrap_or_default();
                drop(state);
                return Ok(Some(CompletionResponse::Array(items)));
            }
        }

        let mut items = if !in_import_context && module_ref_simple.is_none() {
            analysis
                .as_deref()
                .map(|analysis| local_completions(analysis, &prefix))
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        if !in_import_context && module_ref_simple.is_none() {
            items.extend(keyword_completions(&prefix));
        }
        if !in_import_context && module_ref_simple.is_none() {
            items.extend(module_prefix_completions(
                &prefix,
                &current_line_index,
                offset,
            ));
        }

        if let Some(resolver) = &state.resolver {
            let existing_imports = current_imports(&current_source);

            if is_colon_trigger && !in_import_context && module_ref_simple.is_none() {
                let has_pending_module =
                    word_before_colon(&current_source, offset).is_some_and(|word| {
                        resolver
                            .all_modules()
                            .values()
                            .any(|info| info.import_name == word)
                    });
                if !has_pending_module {
                    return Ok(Some(CompletionResponse::Array(Vec::new())));
                }
            }

            if in_import_context {
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
            } else if let Some((module_name, partial)) = module_ref_simple.as_ref() {
                let Some(info) = resolver
                    .all_modules()
                    .values()
                    .find(|info| info.import_name == *module_name)
                else {
                    items.clear();
                    drop(state);
                    return Ok(Some(CompletionResponse::Array(items)));
                };
                let import_path = relative_import_path(&path, &info.file_path, resolver);
                let imported = is_imported(&existing_imports, module_name);
                items.extend(module_ref_completions(
                    &state,
                    resolver,
                    module_name,
                    partial,
                    &current_line_index,
                    offset,
                    (!imported).then_some((current_source.as_str(), import_path.as_str())),
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
            }
        }
        drop(state);

        Ok(Some(CompletionResponse::Array(items)))
    }
}

fn clean_completion_source(source: &str, offset: usize) -> String {
    let offset = offset.min(source.len());
    let tree = vinyl_parser::parse_tree(source);
    match vinyl_parser::statement_range_at(&tree, offset) {
        Some((start, end)) => format!("{}{}", &source[..start], &source[end..]),
        None => source.to_string(),
    }
}

fn analyze_completion_source(state: &State, path: &Path, source: &str) -> Option<Arc<Analysis>> {
    let name = path.to_string_lossy();
    let tree = vinyl_parser::parse_with_name(&name, source).ok()?;
    let items = vinyl_parser::lower::lower(&tree, source, &name).ok()?;
    if let Some(analysis) = analyze_completion_source_with_imports(state, path, source, &items) {
        return Some(analysis);
    }
    analyze_with_diagnostics(path, source, &items, &state.module_table).ok()
}

fn analyze_completion_source_with_imports(
    state: &State,
    path: &Path,
    source: &str,
    items: &[Item],
) -> Option<Arc<Analysis>> {
    let workspace_root = state.workspace_root.as_deref()?;
    let fs = Box::new(LspFileSystem::new(state.vfs.files().clone()));
    let mut resolver = Resolver::detect_with(workspace_root, fs).ok()?;
    if let ResolverMode::Script = resolver.mode() {
        for file_path in state.vfs.files().keys() {
            if file_path.extension().is_some_and(|extension| extension == "vn") {
                resolver.register_module(file_path);
            }
        }
    }
    let mut all_items = items.to_vec();
    let mut module_table = ModuleTable::new();
    let mut visited = HashSet::new();
    let mut bare_imported_symbols = HashSet::new();
    let mut diagnostics = HashMap::new();
    collect_modules(
        &state.vfs,
        &mut resolver,
        workspace_root,
        path,
        items,
        &mut all_items,
        &mut module_table,
        &mut visited,
        &mut bare_imported_symbols,
        &mut diagnostics,
    );
    add_resolved_modules(&state.vfs, &resolver, path, &mut module_table);
    analyze_with_diagnostics(path, source, &all_items, &module_table).ok()
}

fn field_access_context(source: &str, offset: usize) -> Option<usize> {
    let offset = offset.min(source.len());
    let bytes = source.as_bytes();
    let mut word_start = offset;
    while word_start > 0
        && (bytes[word_start - 1].is_ascii_alphanumeric() || bytes[word_start - 1] == b'_')
    {
        word_start -= 1;
    }
    if word_start == 0 || bytes[word_start - 1] != b'.' {
        return None;
    }
    if word_start >= 2 && bytes[word_start - 2] == b'.' {
        return None;
    }
    if word_start >= 2 {
        let before_dot = bytes[word_start - 2];
        if !(before_dot.is_ascii_alphanumeric()
            || before_dot == b'_'
            || before_dot == b')'
            || before_dot == b']')
        {
            return None;
        }
    }
    Some(word_start - 1)
}

fn field_completions(
    state: &State,
    path: &Path,
    source: &str,
    offset: usize,
    dot_index: usize,
    prefix: &str,
) -> Option<Vec<CompletionItem>> {
    let variable_name = source[..dot_index]
        .rsplit(|character: char| !character.is_alphanumeric() && character != '_')
        .next()?;
    let clean_source = clean_completion_source(source, offset);
    let analysis = analyze_completion_source(state, path, &clean_source)?;
    let definition = analysis.result.definitions.get(variable_name)?.first()?;
    let type_name = definition.type_name.as_ref()?;
    let type_lookup_name = type_name.rsplit("::").next().unwrap_or(type_name);
    let line_index = LineIndex::new(source);
    let edit_range = Range::new(
        position_at(&line_index, offset.saturating_sub(prefix.len())),
        position_at(&line_index, offset),
    );
    let structure = analysis
        .result
        .items
        .iter()
        .find_map(|item| match &item.kind {
            vinyl_typecheck::hir::HirItemKind::Struct(structure)
                if structure.name == type_lookup_name =>
            {
                Some(structure.clone())
            }
            _ => None,
        });
    if let Some(structure) = structure {
        let completions = structure
            .fields
            .iter()
            .filter(|field| {
                field.name.starts_with(prefix)
                    && (!is_imported_type(state, type_lookup_name) || field.public)
            })
            .map(|field| CompletionItem {
                label: field.name.clone(),
                kind: Some(CompletionItemKind::FIELD),
                detail: Some(field.type_.to_string()),
                text_edit: Some(CompletionTextEdit::Edit(TextEdit::new(
                    edit_range,
                    field.name.clone(),
                ))),
                ..CompletionItem::default()
            })
            .collect();
        return Some(completions);
    }
    if let Some(tuple) = analysis
        .result
        .items
        .iter()
        .find_map(|item| match &item.kind {
            vinyl_typecheck::hir::HirItemKind::TupleStruct(tuple)
                if tuple.name == type_lookup_name =>
            {
                Some(tuple.clone())
            }
            _ => None,
        })
    {
        return Some(tuple_member_completions(&tuple.types, prefix, edit_range));
    }
    let tuple_types = type_name.strip_prefix('(')?.strip_suffix(')')?;
    let tuple_len = if tuple_types.trim().is_empty() {
        0
    } else {
        tuple_types.split(',').count()
    };
    Some(
        (0..tuple_len)
            .map(|index| index.to_string())
            .filter(|label| label.starts_with(prefix))
            .map(|label| CompletionItem {
                label: label.clone(),
                kind: Some(CompletionItemKind::FIELD),
                detail: Some("tuple member".to_string()),
                text_edit: Some(CompletionTextEdit::Edit(TextEdit::new(edit_range, label))),
                ..CompletionItem::default()
            })
            .collect(),
    )
}

fn tuple_member_completions(
    types: &[vinyl_typecheck::hir::Type],
    prefix: &str,
    edit_range: Range,
) -> Vec<CompletionItem> {
    types
        .iter()
        .enumerate()
        .map(|(index, _type_)| index.to_string())
        .filter(|label| label.starts_with(prefix))
        .map(|label| CompletionItem {
            detail: Some("tuple member".to_string()),
            kind: Some(CompletionItemKind::FIELD),
            text_edit: Some(CompletionTextEdit::Edit(TextEdit::new(
                edit_range,
                label.clone(),
            ))),
            label,
            ..CompletionItem::default()
        })
        .collect()
}

fn variant_completions(
    state: &State,
    path: &Path,
    source: &str,
    offset: usize,
    prefix: &str,
) -> Option<Vec<CompletionItem>> {
    let between_colons = offset > 0
        && offset < source.len()
        && source.as_bytes()[offset - 1] == b':'
        && source.as_bytes()[offset] == b':';
    let enum_end = if between_colons {
        offset - 1
    } else {
        offset.saturating_sub(2)
    };
    let enum_path = source[..enum_end]
        .rsplit(|character: char| !character.is_alphanumeric() && character != '_')
        .next()?;
    let qualified_name = source[..enum_end]
        .rsplit(|character: char| character.is_whitespace() || "=({[,;".contains(character))
        .next()
        .unwrap_or(enum_path)
        .trim();
    let enum_name = qualified_name.rsplit("::").next().unwrap_or(enum_path);
    let clean_end = if between_colons { offset + 1 } else { offset };
    let clean_source = clean_completion_source(source, clean_end);
    let variants = if qualified_name.contains("::") {
        let (module_name, enum_name) = qualified_name.rsplit_once("::")?;
        let resolver = state.resolver.as_ref()?;
        let info = resolver
            .all_modules()
            .values()
            .find(|info| info.import_name == module_name)?;
        let workspace_root = state.workspace_root.as_deref().unwrap_or(resolver.root());
        let cache_key =
            crate::backend::workspace::non_canonical_key(&info.file_path, resolver, workspace_root);
        state
            .cache
            .get(&cache_key)?
            .result
            .items
            .iter()
            .find_map(|item| match &item.kind {
                vinyl_typecheck::hir::HirItemKind::Enum(enumeration)
                    if enumeration.name == enum_name =>
                {
                    Some(
                        enumeration
                            .variants
                            .iter()
                            .filter(|variant| variant.public)
                            .map(|variant| variant.name.clone())
                            .collect::<Vec<_>>(),
                    )
                }
                _ => None,
            })?
    } else {
        let analysis = analyze_completion_source(state, path, &clean_source)?;
        analysis
            .result
            .items
            .iter()
            .find_map(|item| match &item.kind {
                vinyl_typecheck::hir::HirItemKind::Enum(enumeration)
                    if enumeration.name == enum_name =>
                {
                    Some(
                        enumeration
                            .variants
                            .iter()
                            .filter(|variant| !is_imported_type(state, enum_name) || variant.public)
                            .map(|variant| variant.name.clone())
                            .collect::<Vec<_>>(),
                    )
                }
                _ => None,
            })?
    };
    let line_index = LineIndex::new(source);
    let edit_range = Range::new(
        position_at(&line_index, offset.saturating_sub(prefix.len())),
        position_at(&line_index, offset),
    );
    Some(
        variants
            .into_iter()
            .filter(|variant| variant.starts_with(prefix))
            .map(|variant| CompletionItem {
                label: variant.clone(),
                kind: Some(CompletionItemKind::ENUM_MEMBER),
                text_edit: Some(CompletionTextEdit::Edit(TextEdit::new(
                    edit_range,
                    variant.clone(),
                ))),
                ..CompletionItem::default()
            })
            .collect(),
    )
}

fn is_imported_type(state: &State, type_name: &str) -> bool {
    state
        .module_table
        .values()
        .any(|exports| exports.imported && exports.types.iter().any(|name| name == type_name))
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
            DefinitionKind::TypeAlias => CompletionItemKind::STRUCT,
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

    for item in &analysis.result.items {
        let vinyl_typecheck::hir::HirItemKind::Function(function) = &item.kind else {
            continue;
        };
        for parameter in &function.params {
            if parameter.name.starts_with(prefix)
                && !items.iter().any(|item| item.label == parameter.name)
            {
                items.push(CompletionItem {
                    label: parameter.name.clone(),
                    kind: Some(CompletionItemKind::VARIABLE),
                    detail: Some(parameter.type_.to_string()),
                    ..CompletionItem::default()
                });
            }
        }
    }

    for definition in analysis.result.references.values() {
        if !matches!(definition.kind, DefinitionKind::Parameter)
            || !definition.name.starts_with(prefix)
            || items.iter().any(|item| item.label == definition.name)
        {
            continue;
        }
        items.push(CompletionItem {
            label: definition.name.clone(),
            kind: Some(CompletionItemKind::VARIABLE),
            detail: definition.type_name.clone(),
            ..CompletionItem::default()
        });
    }

    items
}

fn keyword_completions(prefix: &str) -> Vec<CompletionItem> {
    KEYWORDS
        .iter()
        .filter(|(keyword, _)| keyword.starts_with(prefix))
        .map(|(keyword, kind)| CompletionItem {
            label: (*keyword).to_string(),
            kind: Some(*kind),
            ..CompletionItem::default()
        })
        .collect()
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use super::keyword_completions;

    #[test]
    fn includes_type_and_value_keywords() {
        let labels: Vec<_> = keyword_completions("")
            .into_iter()
            .map(|item| item.label)
            .collect();
        for keyword in [
            "struct", "enum", "tuple", "type", "int", "float", "bool", "char", "string", "unit",
            "int8", "int16", "int32", "int64", "int128", "isize", "uint8", "uint16", "uint32",
            "uint64", "uint128", "usize", "float32", "float64",
        ] {
            assert!(labels.iter().any(|label| label == keyword));
        }
    }
}

fn module_ref_completions(
    state: &State,
    resolver: &Resolver,
    module_name: &str,
    partial: &str,
    current_line_index: &LineIndex,
    offset: usize,
    auto_import: Option<(&str, &str)>,
) -> Vec<CompletionItem> {
    let workspace_root = state.workspace_root.as_deref().unwrap_or(resolver.root());
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
            if !is_public_symbol(module_analysis, name) {
                continue;
            }
            let kind = match definition.kind {
                DefinitionKind::Function => CompletionItemKind::FUNCTION,
                DefinitionKind::Struct => CompletionItemKind::STRUCT,
                DefinitionKind::Enum => CompletionItemKind::ENUM,
                DefinitionKind::TupleStruct => CompletionItemKind::STRUCT,
                DefinitionKind::TypeAlias => CompletionItemKind::STRUCT,
                _ => continue,
            };
            let detail =
                definition_detail(definition, &module_analysis.result, &module_analysis.source);
            let cursor_pos = position_at(current_line_index, offset);
            let edit_range = Range::new(
                position_at(current_line_index, offset.saturating_sub(partial.len())),
                cursor_pos,
            );
            let additional_text_edits = auto_import.map(|(source, import_path)| {
                vec![TextEdit::new(
                    import_edit_range(current_line_index, source),
                    format!("import {import_path};\n"),
                )]
            });
            items.push(CompletionItem {
                label: name.clone(),
                kind: Some(kind),
                detail,
                text_edit: Some(CompletionTextEdit::Edit(TextEdit::new(
                    edit_range,
                    name.clone(),
                ))),
                additional_text_edits,
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
        let already_imported = is_imported(&existing_imports, &info.import_name);
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
            if !is_public_symbol(module_analysis, name) {
                continue;
            }
            let kind = match definition.kind {
                DefinitionKind::Function => CompletionItemKind::FUNCTION,
                DefinitionKind::Struct => CompletionItemKind::STRUCT,
                DefinitionKind::Enum => CompletionItemKind::ENUM,
                DefinitionKind::TupleStruct => CompletionItemKind::STRUCT,
                DefinitionKind::TypeAlias => CompletionItemKind::STRUCT,
                _ => continue,
            };
            let detail =
                definition_detail(definition, &module_analysis.result, &module_analysis.source);
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

fn module_prefix_completions(
    prefix: &str,
    current_line_index: &LineIndex,
    offset: usize,
) -> Vec<CompletionItem> {
    let cursor = position_at(current_line_index, offset);
    let range = Range::new(
        position_at(current_line_index, offset.saturating_sub(prefix.len())),
        cursor,
    );
    MODULE_PREFIXES
        .iter()
        .filter(|(label, _)| label.starts_with(prefix))
        .map(|(label, kind)| CompletionItem {
            label: (*label).to_string(),
            kind: Some(*kind),
            text_edit: Some(CompletionTextEdit::Edit(TextEdit::new(
                range,
                (*label).to_string(),
            ))),
            ..CompletionItem::default()
        })
        .collect()
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
    if prefix_count == 0 {
        items.extend(module_prefix_completions(
            partial,
            current_line_index,
            offset,
        ));
    }
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
                    position_at(current_line_index, offset.saturating_sub(partial.len())),
                    position_at(current_line_index, offset),
                ),
                stem,
            ))),
            ..CompletionItem::default()
        });
    }
    items
}
