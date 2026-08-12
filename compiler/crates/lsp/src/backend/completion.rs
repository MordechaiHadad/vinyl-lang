use std::path::Path;
use std::sync::Arc;

use line_index::LineIndex;
use tower_lsp::lsp_types::*;
use vinyl_parser::ast::item::Item;
use vinyl_resolver::resolver::{Resolver, ResolverMode};
use vinyl_typecheck::DefinitionKind;
use vinyl_typecheck::index::types::Definition;

use crate::backend::definition::definition_detail;
use crate::backend::state::{Analysis, Backend, State};
use crate::backend::workspace::{
    analyze_with_diagnostics, is_imported, is_public_symbol, non_canonical_key,
    parse_file_with_diagnostics, same_file,
};
use crate::consts::{KEYWORDS, MODULE_PREFIXES};
use crate::position::{offset_at, position_at};
use crate::text::{
    ModulePathContext, current_imports, import_edit_range, module_path_context, word_before_colon,
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
        if let Some(items) = attribute_completions(&current_source, &current_line_index, offset) {
            drop(state);
            return Ok(Some(CompletionResponse::Array(items)));
        }
        let module_context = module_path_context(&current_source, offset);
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
        let is_import_context = matches!(
            &module_context,
            Some(ModulePathContext::ImportPath { .. } | ModulePathContext::ImportSymbol { .. })
        );
        let is_module_ref = matches!(&module_context, Some(ModulePathContext::ModuleRef { .. }));
        if !is_import_context {
            if variant_trigger
                && !is_module_ref
                && let Some(items) =
                    variant_completions(&state, &path, &current_source, offset, &prefix)
            {
                drop(state);
                return Ok(Some(CompletionResponse::Array(items)));
            }
            if let Some(dot_index) = field_access_dot {
                let items =
                    field_completions(&state, &path, &current_source, offset, dot_index, &prefix)
                        .unwrap_or_default();
                drop(state);
                return Ok(Some(CompletionResponse::Array(items)));
            }
        }
        if let Some(struct_type) = struct_literal_context(&current_source, offset)
            && let Some(items) = struct_literal_field_completions(
                &state,
                &path,
                &current_source,
                offset,
                &struct_type,
                &prefix,
            )
        {
            drop(state);
            return Ok(Some(CompletionResponse::List(CompletionList {
                is_incomplete: true,
                items,
            })));
        }

        let mut items = if !is_import_context && !is_module_ref {
            analysis
                .as_deref()
                .map(|analysis| local_completions(analysis, &prefix, offset))
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        if !is_import_context && !is_module_ref {
            items.extend(keyword_completions(&prefix));
        }
        if !is_import_context && !is_module_ref {
            items.extend(module_prefix_completions(
                &prefix,
                &current_line_index,
                offset,
            ));
        }

        if let Some(resolver) = &state.resolver {
            let existing_imports = current_imports(&current_source);

            if !is_import_context
                && let Some(module_items) = expression_module_completions(
                    resolver,
                    &path,
                    &current_source,
                    &current_line_index,
                    offset,
                )
            {
                drop(state);
                return Ok(Some(CompletionResponse::Array(module_items)));
            }

            if is_colon_trigger && !is_import_context && !is_module_ref {
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

            match &module_context {
                Some(ModulePathContext::ImportPath { segments, partial }) => {
                    if segments.is_empty() {
                        items.extend(module_prefix_completions(
                            &prefix,
                            &current_line_index,
                            offset,
                        ));
                    } else {
                        items.extend(import_prefix_completions(
                            resolver,
                            &path,
                            segments.len(),
                            partial,
                            &current_line_index,
                            offset,
                        ));
                    }
                }
                Some(ModulePathContext::ImportSymbol {
                    module_name,
                    partial,
                }) => {
                    items.extend(module_ref_completions(
                        &state,
                        resolver,
                        module_name,
                        partial,
                        &current_line_index,
                        offset,
                        None,
                    ));
                }
                Some(ModulePathContext::ModuleRef {
                    module_name,
                    partial,
                    scope_qualified,
                }) => {
                    let Some(info) = resolver
                        .all_modules()
                        .values()
                        .find(|info| info.import_name == *module_name)
                    else {
                        items.clear();
                        drop(state);
                        return Ok(Some(CompletionResponse::Array(items)));
                    };
                    let import_path = resolver.relative_import_path(&path, &info.file_path);
                    let imported = is_imported(&existing_imports, module_name);
                    items.extend(module_ref_completions(
                        &state,
                        resolver,
                        module_name,
                        partial,
                        &current_line_index,
                        offset,
                        (!imported && !scope_qualified)
                            .then_some((current_source.as_str(), import_path.as_str())),
                    ));
                }
                None => {
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

fn attribute_completions(
    source: &str,
    line_index: &LineIndex,
    offset: usize,
) -> Option<Vec<CompletionItem>> {
    let line_start = source[..offset].rfind('\n').map_or(0, |index| index + 1);
    let line = &source[line_start..offset];
    if let Some(open_paren) = line.rfind('(')
        && let Some(at_index) = line[..open_paren].rfind('@')
    {
        let before_at = &line[..at_index];
        let attr_name = line[at_index + 1..open_paren].trim();
        if !before_at.trim().is_empty() || attr_name != "allow" {
            return None;
        }
        let partial = &line[open_paren + 1..];
        if !partial
            .chars()
            .all(|character| character == '_' || character.is_ascii_alphanumeric())
        {
            return None;
        }
        return attribute_arg_completions(line_index, offset, partial);
    }
    let at_index = line.rfind('@')?;
    let before_at = &line[..at_index];
    let partial = &line[at_index + 1..];
    if !before_at.trim().is_empty()
        || !partial
            .chars()
            .all(|character| character == '_' || character.is_ascii_alphanumeric())
    {
        return None;
    }
    attribute_name_completions(line_index, offset, partial)
}

fn attribute_name_completions(
    line_index: &LineIndex,
    offset: usize,
    partial: &str,
) -> Option<Vec<CompletionItem>> {
    let range = Range::new(
        position_at(line_index, offset.saturating_sub(partial.len())),
        position_at(line_index, offset),
    );
    let attributes = [
        (
            "doc",
            "documentation attribute",
            "Attach Markdown documentation to a function, struct, tuple, enum, or type alias.\n\n```vinyl\n@doc(\"Adds two numbers\")\nfn add() {}\n```\n\nThe documentation is shown in LSP hover results.",
        ),
        (
            "allow",
            "lint suppression attribute",
            "Suppress a compiler diagnostic on the attached item.\n\n```vinyl\n@allow(large_array)\nfn main() {}\n```\n\nThe diagnostic name to suppress goes inside the parentheses.",
        ),
        (
            "intrinsic",
            "compiler intrinsic attribute",
            "Mark a function as a compiler intrinsic. The body is never compiled; calls are lowered directly by the compiler instead.\n\n```vinyl\n@intrinsic\npublic fn len(values: [int;1]): usize { 0 }\n```\n\nTypically only used inside the standard library.",
        ),
    ];
    let items: Vec<CompletionItem> = attributes
        .iter()
        .filter(|(name, _, _)| name.starts_with(partial))
        .map(|(name, detail, docs)| CompletionItem {
            label: (*name).to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some((*detail).to_string()),
            documentation: Some(Documentation::MarkupContent(MarkupContent {
                kind: MarkupKind::Markdown,
                value: (*docs).to_string(),
            })),
            text_edit: Some(CompletionTextEdit::Edit(TextEdit::new(
                range,
                (*name).to_string(),
            ))),
            ..CompletionItem::default()
        })
        .collect();
    (!items.is_empty()).then_some(items)
}

fn attribute_arg_completions(
    line_index: &LineIndex,
    offset: usize,
    partial: &str,
) -> Option<Vec<CompletionItem>> {
    let range = Range::new(
        position_at(line_index, offset.saturating_sub(partial.len())),
        position_at(line_index, offset),
    );
    let args = [(
        "large_array",
        "Suppress the `large_array` warning (arrays of 32 KiB or more) and the `array_too_large` error (arrays of 1 MiB or more) for array fills in the attached function.\n\nHeap-allocated arrays are not implemented yet, so large stack arrays are flagged.",
    )];
    let items: Vec<CompletionItem> = args
        .iter()
        .filter(|(name, _)| name.starts_with(partial))
        .map(|(name, docs)| CompletionItem {
            label: (*name).to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some("large array diagnostic suppression".to_string()),
            documentation: Some(Documentation::MarkupContent(MarkupContent {
                kind: MarkupKind::Markdown,
                value: (*docs).to_string(),
            })),
            text_edit: Some(CompletionTextEdit::Edit(TextEdit::new(
                range,
                (*name).to_string(),
            ))),
            ..CompletionItem::default()
        })
        .collect();
    (!items.is_empty()).then_some(items)
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
            if file_path
                .extension()
                .is_some_and(|extension| extension == "vn")
            {
                resolver.register_module(file_path);
            }
        }
    }
    let mut read_source = |path: &Path| {
        state
            .vfs
            .source(path)
            .ok_or_else(|| format!("could not read {}", path.display()))
    };
    let graph = resolver.build_module_graph(path, items, &mut read_source);
    analyze_with_diagnostics(path, source, &graph.all_items, &graph.module_table).ok()
}

fn struct_literal_context(source: &str, offset: usize) -> Option<String> {
    let offset = offset.min(source.len());
    let before = &source[..offset];
    let brace = before.rfind('{')?;
    if before[brace + 1..].contains('}') {
        return None;
    }
    let type_name = before[..brace]
        .rsplit(|character: char| !character.is_alphanumeric() && character != '_')
        .find(|chunk| !chunk.is_empty())?;
    if !type_name
        .chars()
        .next()
        .is_some_and(|character| character.is_uppercase())
    {
        return None;
    }
    Some(type_name.to_string())
}

fn struct_literal_field_completions(
    state: &State,
    path: &Path,
    source: &str,
    offset: usize,
    type_name: &str,
    prefix: &str,
) -> Option<Vec<CompletionItem>> {
    let offset = offset.min(source.len());
    let clean_source = clean_completion_source(source, offset);
    let analysis = analyze_completion_source(state, path, &clean_source)?;
    let structure = analysis
        .result
        .items
        .iter()
        .find_map(|item| match &item.kind {
            vinyl_typecheck::hir::HirItemKind::Struct(structure) if structure.name == type_name => {
                Some(structure.clone())
            }
            _ => None,
        })?;
    let brace = if source.as_bytes().get(offset) == Some(&b'{') {
        offset
    } else {
        source[..offset].rfind('{')?
    };
    let written: Vec<&str> = source[brace + 1..offset]
        .split(',')
        .filter_map(|part| {
            let name = part.split(':').next()?.trim();
            if name.is_empty() { None } else { Some(name) }
        })
        .collect();
    let line_index = LineIndex::new(source);
    let edit_range = Range::new(
        position_at(&line_index, offset.saturating_sub(prefix.len())),
        position_at(&line_index, offset),
    );
    let completions = structure
        .fields
        .iter()
        .filter(|field| {
            field.name.starts_with(prefix)
                && field.public
                && !written.iter().any(|written| *written == field.name)
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
    Some(completions)
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

fn definition_in_scope(definition: &Definition, offset: usize) -> bool {
    definition.scope.is_none_or(|span| {
        let start = span.offset();
        offset >= start && offset < start + span.len()
    })
}

fn local_completions(analysis: &Analysis, prefix: &str, offset: usize) -> Vec<CompletionItem> {
    let mut items = Vec::new();
    for (name, definitions) in &analysis.result.definitions {
        if !name.starts_with(prefix) {
            continue;
        }
        let Some(definition) = definitions
            .iter()
            .filter(|definition| definition_in_scope(definition, offset))
            .max_by_key(|definition| definition.scope_depth)
        else {
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
        let function_span = function.span;
        if offset < function_span.offset() || offset >= function_span.offset() + function_span.len()
        {
            continue;
        }
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
            || !definition_in_scope(definition, offset)
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
    use line_index::LineIndex;
    use tower_lsp::lsp_types::{Documentation, MarkupContent, MarkupKind};

    use super::{attribute_completions, keyword_completions};

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

    #[test]
    fn suggests_doc_attribute_after_at() {
        let source = "@d";
        let line_index = LineIndex::new(source);
        let items = attribute_completions(source, &line_index, source.len()).unwrap();
        assert_eq!(items[0].label, "doc");
        assert!(matches!(
            items[0].documentation,
            Some(Documentation::MarkupContent(MarkupContent {
                kind: MarkupKind::Markdown,
                ..
            }))
        ));
    }

    #[test]
    fn suggests_allow_and_doc_after_at() {
        let source = "@";
        let line_index = LineIndex::new(source);
        let items = attribute_completions(source, &line_index, source.len()).unwrap();
        let labels: Vec<_> = items.iter().map(|item| item.label.as_str()).collect();
        assert!(labels.contains(&"doc"));
        assert!(labels.contains(&"allow"));
        assert!(labels.contains(&"intrinsic"));
    }

    #[test]
    fn suggests_intrinsic_attribute_after_at() {
        let source = "@i";
        let line_index = LineIndex::new(source);
        let items = attribute_completions(source, &line_index, source.len()).unwrap();
        assert_eq!(items[0].label, "intrinsic");
        assert!(matches!(
            items[0].documentation,
            Some(Documentation::MarkupContent(MarkupContent {
                kind: MarkupKind::Markdown,
                ..
            }))
        ));
    }

    #[test]
    fn suggests_large_array_inside_allow() {
        let source = "@allow(large_";
        let line_index = LineIndex::new(source);
        let items = attribute_completions(source, &line_index, source.len()).unwrap();
        assert_eq!(items[0].label, "large_array");
        assert!(matches!(
            items[0].documentation,
            Some(Documentation::MarkupContent(MarkupContent {
                kind: MarkupKind::Markdown,
                ..
            }))
        ));
    }

    #[test]
    fn no_completion_inside_doc_args() {
        let source = "@doc(\"hello";
        let line_index = LineIndex::new(source);
        assert!(attribute_completions(source, &line_index, source.len()).is_none());
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
    let mut found_module = false;
    for info in resolver.all_modules().values() {
        if info.import_name != module_name {
            continue;
        }
        found_module = true;
        let cache_key = non_canonical_key(&info.file_path, resolver, workspace_root);
        let Some(module_analysis) = state.cache.get(&cache_key) else {
            return module_ref_file_completions(
                state,
                &info.file_path,
                module_name,
                partial,
                current_line_index,
                offset,
            );
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
    if !found_module {
        let file_path = resolver.root().join(module_name).with_extension("vn");
        return module_ref_file_completions(
            state,
            &file_path,
            module_name,
            partial,
            current_line_index,
            offset,
        );
    }
    items
}

fn module_ref_file_completions(
    state: &State,
    file_path: &Path,
    module_name: &str,
    partial: &str,
    current_line_index: &LineIndex,
    offset: usize,
) -> Vec<CompletionItem> {
    let Ok((_, module_items)) = parse_file_with_diagnostics(&state.vfs, file_path) else {
        return Vec::new();
    };
    let edit_range = Range::new(
        position_at(current_line_index, offset.saturating_sub(partial.len())),
        position_at(current_line_index, offset),
    );
    module_items
        .into_iter()
        .filter_map(|item| {
            let (name, kind) = match item {
                Item::Function(function) if function.public => {
                    (function.name, CompletionItemKind::FUNCTION)
                }
                Item::Struct(structure) if structure.public => {
                    (structure.name, CompletionItemKind::STRUCT)
                }
                Item::TupleStruct(tuple) if tuple.public => {
                    (tuple.name, CompletionItemKind::STRUCT)
                }
                Item::Enum(enumeration) if enumeration.public => {
                    (enumeration.name, CompletionItemKind::ENUM)
                }
                Item::TypeAlias(alias) if alias.public => (alias.name, CompletionItemKind::STRUCT),
                _ => return None,
            };
            name.starts_with(partial).then_some(CompletionItem {
                label: name.clone(),
                kind: Some(kind),
                detail: Some(format!("from {module_name}")),
                text_edit: Some(CompletionTextEdit::Edit(TextEdit::new(edit_range, name))),
                ..CompletionItem::default()
            })
        })
        .collect()
}

fn expression_module_completions(
    resolver: &Resolver,
    path: &Path,
    source: &str,
    line_index: &LineIndex,
    offset: usize,
) -> Option<Vec<CompletionItem>> {
    let line_start = source[..offset].rfind('\n').map_or(0, |index| index + 1);
    let line = &source[line_start..offset];
    let token_start = line
        .rfind(|character: char| character.is_whitespace() || "=({[,;>|".contains(character))
        .map_or(0, |index| index + 1);
    let token = &line[token_start..];
    if !token.contains("::") {
        return None;
    }
    let segments: Vec<&str> = token.split("::").collect();
    if segments.first().copied() != Some("parent") {
        return None;
    }
    let partial = if token.ends_with("::") {
        ""
    } else {
        segments.last().copied().unwrap_or_default()
    };
    let module_segments = if partial.is_empty() {
        &segments[1..segments.len().saturating_sub(1)]
    } else {
        &segments[1..segments.len() - 1]
    };
    if module_segments.len() > 1 {
        return None;
    }
    let mut directory = path.parent()?.to_path_buf();
    for segment in module_segments {
        directory.push(segment);
    }
    let files = resolver.list_vn_files(&directory).ok()?;
    let edit_range = Range::new(
        position_at(line_index, offset.saturating_sub(partial.len())),
        position_at(line_index, offset),
    );
    let items: Vec<_> = files
        .into_iter()
        .filter(|file| file.parent() == Some(directory.as_path()))
        .filter_map(|file| {
            file.file_stem()
                .map(|stem| stem.to_string_lossy().to_string())
        })
        .filter(|stem| stem.starts_with(partial))
        .map(|stem| CompletionItem {
            label: format!("{stem}::"),
            kind: Some(CompletionItemKind::MODULE),
            text_edit: Some(CompletionTextEdit::Edit(TextEdit::new(
                edit_range,
                format!("{stem}::"),
            ))),
            ..CompletionItem::default()
        })
        .collect();
    (!items.is_empty()).then_some(items)
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
        let import_path = resolver.relative_import_path(path, &info.file_path);
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
