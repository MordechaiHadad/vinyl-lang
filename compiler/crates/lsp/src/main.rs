mod consts;
mod utils;
mod vfs;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use crate::vfs::LspFileSystem;
use clap::Parser;
use eyre::{Result, eyre};
use line_index::{LineCol, LineIndex, TextSize, WideEncoding, WideLineCol};
use tokio::sync::RwLock;
use tower_lsp::lsp_types::notification::Progress;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};
use tracing::{debug, info};
use vinyl_parser::ast::item::Item;
use vinyl_resolver::{ImportPrefix, Resolver, ResolverMode};
use vinyl_typecheck::hir::{HirFunction, HirItemKind};
use vinyl_typecheck::module::{ModuleExports, ModuleTable};
use vinyl_typecheck::{Definition, DefinitionKind, TypeckResult};

use crate::consts::KEYWORDS;
use crate::utils::{Cli, init_tracing};
use crate::vfs::Vfs;

struct Analysis {
    path: PathBuf,
    source: String,
    line_index: LineIndex,
    result: vinyl_typecheck::TypeckResult,
}

#[derive(Clone)]
struct SourceDiagnostic {
    message: String,
    offset: usize,
    length: usize,
}

type WorkspaceAnalyses = HashMap<PathBuf, Arc<Analysis>>;
type WorkspaceDiagnostics = HashMap<PathBuf, Vec<SourceDiagnostic>>;
type WorkspaceState = (
    WorkspaceAnalyses,
    WorkspaceDiagnostics,
    Resolver,
    ModuleTable,
);

#[derive(Default)]
struct State {
    vfs: Vfs,
    cache: HashMap<PathBuf, Arc<Analysis>>,
    workspace_root: Option<PathBuf>,
    update_version: u64,
    resolver: Option<Resolver>,
    module_table: ModuleTable,
}

struct Backend {
    client: Client,
    state: Arc<RwLock<State>>,
}

impl Backend {
    async fn schedule_update(&self, uri: &Url) {
        let version = {
            let mut state = self.state.write().await;
            state.update_version += 1;
            state.update_version
        };
        let state = self.state.clone();
        let client = self.client.clone();
        let uri = uri.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(150)).await;
            if state.read().await.update_version != version {
                return;
            }
            let token = ProgressToken::String(format!("vinyl-lsp-update-{version}"));
            client
                .send_notification::<Progress>(ProgressParams {
                    token: token.clone(),
                    value: ProgressParamsValue::WorkDone(WorkDoneProgress::Begin(
                        WorkDoneProgressBegin {
                            title: "Analyzing workspace".into(),
                            cancellable: Some(false),
                            message: None,
                            percentage: None,
                        },
                    )),
                })
                .await;
            perform_update(&state, &client, &uri).await;
            client
                .send_notification::<Progress>(ProgressParams {
                    token,
                    value: ProgressParamsValue::WorkDone(WorkDoneProgress::End(
                        WorkDoneProgressEnd { message: None },
                    )),
                })
                .await;
        });
    }

    async fn analysis(&self, uri: &Url) -> Option<Arc<Analysis>> {
        let path = uri.to_file_path().ok()?;
        self.state.read().await.cache.get(&path).cloned()
    }

    async fn analyses(&self) -> Vec<Arc<Analysis>> {
        self.state.read().await.cache.values().cloned().collect()
    }

    async fn workspace_locations(&self, target: &vinyl_typecheck::Definition) -> Vec<Location> {
        let target_name = target.name.rsplit("::").next().unwrap_or(&target.name);
        self.analyses()
            .await
            .into_iter()
            .flat_map(|analysis| {
                let uri = Url::from_file_path(&analysis.path).ok();
                let line_index = &analysis.line_index;
                let mut locations = analysis
                    .result
                    .references
                    .iter()
                    .filter(|(_, definition)| {
                        definition
                            .name
                            .rsplit("::")
                            .next()
                            .unwrap_or(&definition.name)
                            == target_name
                    })
                    .filter_map(|(offset, definition)| {
                        uri.clone().map(|uri| {
                            Location::new(
                                uri,
                                span_range(line_index, *offset, definition.name.len()),
                            )
                        })
                    })
                    .collect::<Vec<_>>();
                if let Some(definition) =
                    analysis
                        .result
                        .definitions
                        .get(target_name)
                        .and_then(|definitions| {
                            definitions.iter().find(|definition| {
                                definition.span == target.span || definition.name == target_name
                            })
                        })
                    && let Some(uri) = uri
                {
                    locations.push(Location::new(
                        uri,
                        span_range(line_index, definition.span.offset(), definition.name.len()),
                    ));
                }
                locations
            })
            .collect()
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(
        &self,
        params: InitializeParams,
    ) -> tower_lsp::jsonrpc::Result<InitializeResult> {
        if let Some(root_uri) = params.root_uri
            && let Ok(root) = root_uri.to_file_path()
        {
            self.state.write().await.workspace_root = Some(root);
        }
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![":".to_string()]),
                    ..CompletionOptions::default()
                }),
                references_provider: Some(OneOf::Left(true)),
                rename_provider: Some(OneOf::Left(true)),
                document_symbol_provider: Some(OneOf::Left(true)),
                signature_help_provider: Some(SignatureHelpOptions::default()),
                code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
                document_formatting_provider: Some(OneOf::Left(true)),
                document_range_formatting_provider: Some(OneOf::Left(true)),
                ..ServerCapabilities::default()
            },
            server_info: Some(ServerInfo {
                name: "vinyl-lsp".to_string(),
                version: None,
            }),
        })
    }

    async fn shutdown(&self) -> tower_lsp::jsonrpc::Result<()> {
        Ok(())
    }

    async fn initialized(&self, _params: InitializedParams) {
        let state = self.state.read().await;
        let has_workspace = state.workspace_root.is_some();
        drop(state);

        if !has_workspace {
            return;
        }

        let token = ProgressToken::String("vinyl-lsp-workspace".to_string());
        self.client
            .send_notification::<Progress>(ProgressParams {
                token: token.clone(),
                value: ProgressParamsValue::WorkDone(WorkDoneProgress::Begin(
                    WorkDoneProgressBegin {
                        title: "Analyzing workspace".into(),
                        cancellable: Some(false),
                        message: None,
                        percentage: None,
                    },
                )),
            })
            .await;

        let uri = {
            let state = self.state.read().await;
            state
                .workspace_root
                .as_ref()
                .and_then(|root| {
                    [root.join("main.vn"), root.join("lib.vn")]
                        .into_iter()
                        .find(|p| p.exists())
                        .or_else(|| {
                            std::fs::read_dir(root)
                                .ok()
                                .and_then(|mut entries| {
                                    entries.find_map(|e| {
                                        e.ok().filter(|e| {
                                            e.path().extension().is_some_and(|ext| ext == "vn")
                                        })
                                    })
                                })
                                .map(|e| e.path())
                        })
                })
                .and_then(|path| Url::from_file_path(path).ok())
        };

        if let Some(uri) = uri {
            perform_update(&self.state, &self.client, &uri).await;
        }

        self.client
            .send_notification::<Progress>(ProgressParams {
                token,
                value: ProgressParamsValue::WorkDone(WorkDoneProgress::End(WorkDoneProgressEnd {
                    message: None,
                })),
            })
            .await;
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        if let Ok(path) = params.text_document.uri.to_file_path() {
            self.state
                .write()
                .await
                .vfs
                .set(path, params.text_document.text);
        }
        self.schedule_update(&params.text_document.uri).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.into_iter().next()
            && let Ok(path) = params.text_document.uri.to_file_path()
        {
            self.state.write().await.vfs.set(path, change.text);
        }
        self.schedule_update(&params.text_document.uri).await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        if let Ok(path) = params.text_document.uri.to_file_path() {
            let mut state = self.state.write().await;
            state.vfs.remove(&path);
            state.cache.remove(&path);
        }
        self.client
            .publish_diagnostics(params.text_document.uri, Vec::new(), None)
            .await;
    }

    async fn hover(&self, params: HoverParams) -> tower_lsp::jsonrpc::Result<Option<Hover>> {
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
        let Some(expression) = analysis
            .result
            .expr_at_pos
            .range(..=offset)
            .next_back()
            .map(|(_, expression)| expression)
            .filter(|expression| offset < expression.span.offset() + expression.span.len())
        else {
            return Ok(None);
        };
        Ok(Some(Hover {
            contents: HoverContents::Scalar(MarkedString::String(format!(
                "type: {:?}",
                expression.type_
            ))),
            range: None,
        }))
    }

    async fn goto_definition(
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

    async fn formatting(
        &self,
        params: DocumentFormattingParams,
    ) -> tower_lsp::jsonrpc::Result<Option<Vec<TextEdit>>> {
        self.format(params.text_document.uri).await
    }

    async fn range_formatting(
        &self,
        params: DocumentRangeFormattingParams,
    ) -> tower_lsp::jsonrpc::Result<Option<Vec<TextEdit>>> {
        self.format(params.text_document.uri).await
    }

    async fn completion(
        &self,
        params: CompletionParams,
    ) -> tower_lsp::jsonrpc::Result<Option<CompletionResponse>> {
        let uri = &params.text_document_position.text_document.uri;
        let Some(path) = uri.to_file_path().ok() else {
            return Ok(None);
        };
        let Some(analysis) = self.analysis(uri).await else {
            return Ok(None);
        };

        let state = self.state.read().await;
        let current_source = state.vfs.source(&path).unwrap_or_default();
        let current_line_index = LineIndex::new(&current_source);
        let offset = offset_at(&current_line_index, params.text_document_position.position);
        let prefix = word_prefix(&current_source, offset);
        let import_prefix_info = detect_import_prefix(&current_source, offset);
        let in_import_context = import_prefix_info.is_some();
        let module_ref_simple = module_ref_prefix(&current_source, offset);
        let is_colon_trigger = params.context.and_then(|c| c.trigger_character).as_deref() == Some(":");

        let mut items: Vec<CompletionItem> = Vec::new();

        if !in_import_context && module_ref_simple.is_none() {
            for (name, definitions) in &analysis.result.definitions {
                if !name.starts_with(&prefix) {
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
                if keyword.starts_with(&prefix) {
                    items.push(CompletionItem {
                        label: keyword.to_string(),
                        kind: Some(*kind),
                        ..CompletionItem::default()
                    });
                }
            }
        }

        if let Some(resolver) = &state.resolver {
            let existing_imports = current_imports(&current_source);
            let workspace_root = state.workspace_root.as_deref().unwrap_or(resolver.root());

            let has_module_ref = module_ref_simple.as_ref().and_then(|(name, _)| {
                existing_imports.iter().any(|imp| {
                    imp == name || imp.ends_with(&format!("::{name}"))
                }).then_some(name.clone())
            });

            if is_colon_trigger && !in_import_context && module_ref_simple.is_none() {
                let has_pending_module = word_before_colon(&current_source, offset)
                    .is_some_and(|word| {
                        resolver.all_modules().values().any(|info| info.import_name == word)
                    });
                if !has_pending_module {
                    return Ok(Some(CompletionResponse::Array(Vec::new())));
                }
            }

            if let Some(ref module_name) = has_module_ref {
                let (_, partial) = module_ref_simple.as_ref().unwrap();
                for info in resolver.all_modules().values() {
                    if info.import_name != *module_name {
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
                        let cursor_pos = position_at(&current_line_index, offset);
                        let edit_range = Range::new(
                            position_at(&current_line_index, offset.saturating_sub(partial.len())),
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
            }

            if has_module_ref.is_none() {
                for info in resolver.all_modules().values() {
                    if same_file(&path, &info.file_path) {
                        continue;
                    }
                    let cache_key = non_canonical_key(&info.file_path, resolver, workspace_root);
                    let Some(module_analysis) = state.cache.get(&cache_key) else {
                        continue;
                    };
                    let import_path = relative_import_path(&path, &info.file_path, resolver);
                    let already_imported = existing_imports.iter().any(|imp| {
                        imp == &info.import_name
                            || imp.ends_with(&format!("::{}", info.import_name))
                    });
                    if already_imported {
                        continue;
                    }
                    for (name, definitions) in &module_analysis.result.definitions {
                        if !name.starts_with(&prefix) || name.contains("::") {
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
                        let cursor_pos = position_at(&current_line_index, offset);
                        let edit_range = Range::new(
                            position_at(&current_line_index, offset.saturating_sub(prefix.len())),
                            cursor_pos,
                        );
                        let import_edit = TextEdit::new(
                            import_edit_range(&current_line_index, &current_source),
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

                if let Some((prefix_count, partial)) = import_prefix_info {
                    let mut dir =
                        path.parent().unwrap_or(Path::new("")).to_path_buf();
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
                        if !stem.starts_with(&partial) {
                            continue;
                        }
                        items.push(CompletionItem {
                            label: stem.clone(),
                            kind: Some(CompletionItemKind::MODULE),
                            detail: Some("module".to_string()),
                            text_edit: Some(CompletionTextEdit::Edit(TextEdit::new(
                                Range::new(
                                    position_at(
                                        &current_line_index,
                                        offset.saturating_sub(partial.len()),
                                    ),
                                    position_at(&current_line_index, offset),
                                ),
                                stem,
                            ))),
                            ..CompletionItem::default()
                        });
                    }
                }
            }
        }
        drop(state);

        Ok(Some(CompletionResponse::Array(items)))
    }

    async fn references(
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

    async fn rename(
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

    async fn document_symbol(
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
                        vinyl_typecheck::hir::HirItemKind::Function(function) => {
                            (&function.name, SymbolKind::FUNCTION)
                        }
                        vinyl_typecheck::hir::HirItemKind::Struct(structure) => {
                            (&structure.name, SymbolKind::STRUCT)
                        }
                        vinyl_typecheck::hir::HirItemKind::TupleStruct(tuple) => {
                            (&tuple.name, SymbolKind::STRUCT)
                        }
                        vinyl_typecheck::hir::HirItemKind::Enum(enumeration) => {
                            (&enumeration.name, SymbolKind::ENUM)
                        }
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

    async fn signature_help(
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
                vinyl_typecheck::hir::HirItemKind::Function(function) => {
                    if !prefix.is_empty() && !function.name.starts_with(&prefix) {
                        return None;
                    }
                    Some(SignatureInformation {
                        label: format!(
                            "fn {}({}): {}",
                            function.name,
                            function
                                .params
                                .iter()
                                .map(|param| format!("{}: {}", param.name, param.type_))
                                .collect::<Vec<_>>()
                                .join(", "),
                            function.return_type
                        ),
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

    async fn code_action(
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
}

impl Backend {
    async fn format(&self, uri: Url) -> tower_lsp::jsonrpc::Result<Option<Vec<TextEdit>>> {
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

async fn perform_update(state: &Arc<RwLock<State>>, client: &Client, uri: &Url) {
    debug!(%uri, "performing update");
    let Some(path) = uri.to_file_path().ok() else {
        return;
    };

    let (vfs, root, entry_path) = {
        let guard = state.read().await;
        if guard.vfs.source(&path).is_none() {
            return;
        }
        let root = guard
            .workspace_root
            .clone()
            .or_else(|| path.parent().map(Path::to_path_buf));
        let Some(root) = root else {
            return;
        };
        let entry_path = [root.join("main.vn"), root.join("lib.vn")]
            .into_iter()
            .find(|candidate| candidate.exists())
            .unwrap_or(path.clone());
        (guard.vfs.clone(), root, entry_path)
    };

    match analyze_workspace(&vfs, &root, &entry_path) {
        Ok((analyses, diagnostics, resolver, module_table)) => {
            info!(files = analyses.len(), "workspace analysis complete");
            let entry_source = vfs.source(&entry_path).unwrap_or_default();
            let entry_line_index = LineIndex::new(&entry_source);
            let mut entry_diagnostics: Vec<Diagnostic> = diagnostics
                .get(&entry_path)
                .map(|diags| {
                    diags
                        .iter()
                        .map(|d| {
                            Diagnostic::new_simple(
                                span_range(&entry_line_index, d.offset, d.length),
                                d.message.clone(),
                            )
                        })
                        .collect()
                })
                .unwrap_or_default();

            {
                let mut guard = state.write().await;
                guard.resolver = Some(resolver);
                guard.module_table = module_table;
                guard.cache.extend(analyses);
                if let Some(analysis) = guard.cache.get(&entry_path) {
                    for definition in &analysis.result.unused {
                        entry_diagnostics.push(Diagnostic {
                            range: span_range(
                                &entry_line_index,
                                definition.span.offset(),
                                definition.span.len(),
                            ),
                            severity: Some(DiagnosticSeverity::WARNING),
                            message: format!(
                                "unused {}",
                                match definition.kind {
                                    DefinitionKind::Function => "function",
                                    DefinitionKind::Variable => "variable",
                                    DefinitionKind::Parameter => "parameter",
                                    _ => "symbol",
                                }
                            ),
                            ..Diagnostic::default()
                        });
                    }
                }
            }

            client
                .publish_diagnostics(
                    Url::from_file_path(&entry_path).unwrap_or(uri.clone()),
                    entry_diagnostics,
                    None,
                )
                .await;

            for (file_path, file_diags) in &diagnostics {
                if file_path == &entry_path || file_diags.is_empty() {
                    continue;
                }
                let source = vfs.source(file_path).unwrap_or_default();
                let line_index = LineIndex::new(&source);
                let diags: Vec<Diagnostic> = file_diags
                    .iter()
                    .map(|d| {
                        Diagnostic::new_simple(
                            span_range(&line_index, d.offset, d.length),
                            d.message.clone(),
                        )
                    })
                    .collect();
                client
                    .publish_diagnostics(
                        Url::from_file_path(file_path).unwrap_or(uri.clone()),
                        diags,
                        None,
                    )
                    .await;
            }
        }
        Err(error) => {
            let diagnostics = vec![Diagnostic::new_simple(Range::default(), error.to_string())];
            client
                .publish_diagnostics(
                    Url::from_file_path(&entry_path).unwrap_or(uri.clone()),
                    diagnostics,
                    None,
                )
                .await;
        }
    }
}

fn offset_at(line_index: &LineIndex, position: Position) -> usize {
    let wide_col = WideLineCol {
        line: position.line,
        col: position.character,
    };
    let line_col = line_index
        .to_utf8(WideEncoding::Utf16, wide_col)
        .unwrap_or(LineCol {
            line: position.line,
            col: 0,
        });
    line_index.offset(line_col).map(TextSize::into).unwrap_or(0)
}

fn span_range(line_index: &LineIndex, offset: usize, length: usize) -> Range {
    Range::new(
        position_at(line_index, offset),
        position_at(line_index, offset + length),
    )
}

fn full_range(line_index: &LineIndex) -> Range {
    Range::new(
        Position::new(0, 0),
        position_at(line_index, line_index.len().into()),
    )
}

fn position_at(line_index: &LineIndex, offset: usize) -> Position {
    let offset = TextSize::try_from(offset).unwrap_or(line_index.len());
    let line_col = line_index.line_col(offset.min(line_index.len()));
    let wide_col = line_index
        .to_wide(WideEncoding::Utf16, line_col)
        .unwrap_or(WideLineCol {
            line: line_col.line,
            col: 0,
        });
    Position::new(wide_col.line, wide_col.col)
}

fn word_prefix(source: &str, offset: usize) -> String {
    let before = &source[..offset.min(source.len())];
    before
        .rsplit(|character: char| !character.is_alphanumeric() && character != '_')
        .next()
        .unwrap_or_default()
        .to_string()
}

fn detect_import_prefix(source: &str, offset: usize) -> Option<(usize, String)> {
    let before = &source[..offset.min(source.len())];
    let line_start = before.rfind('\n').map(|i| i + 1).unwrap_or(0);
    let line_prefix = &before[line_start..];
    let after_import = line_prefix.strip_prefix("import ").unwrap_or(line_prefix);

    let segments: Vec<&str> = after_import.split("::").collect();

    let mut prefix_count = 0;
    for segment in &segments {
        match *segment {
            "parent" | "self" => prefix_count += 1,
            _ => break,
        }
    }

    if prefix_count == 0 || segments.len() - prefix_count > 1 {
        return None;
    }

    let partial = if prefix_count >= segments.len() {
        String::new()
    } else {
        segments[prefix_count].to_string()
    };

    Some((prefix_count, partial))
}

fn word_before_colon(source: &str, offset: usize) -> Option<String> {
    let offset = offset.min(source.len());
    if offset == 0 {
        return None;
    }
    let before = &source[..offset];
    let bytes = before.as_bytes();
    if bytes[offset - 1] != b':' {
        return None;
    }
    if offset >= 2 && bytes[offset - 2] == b':' {
        return None;
    }
    let word_start = before[..offset - 1]
        .rfind(|c: char| !c.is_alphanumeric() && c != '_')
        .map(|i| i + 1)
        .unwrap_or(0);
    let word = &before[word_start..offset - 1];
    if word.is_empty() { None } else { Some(word.to_string()) }
}

fn module_ref_prefix(source: &str, offset: usize) -> Option<(String, String)> {
    let offset = offset.min(source.len());
    let before = &source[..offset];
    let bytes = source.as_bytes();

    let colon_at = before.rfind("::")
        .or_else(|| {
            if offset >= 1 && offset < source.len() && bytes[offset - 1] == b':' && bytes[offset] == b':' {
                Some(offset - 1)
            } else {
                None
            }
        })
        .or_else(|| {
            if offset + 1 < source.len() && &source[offset..offset + 2] == "::" {
                Some(offset)
            } else {
                None
            }
        })?;

    let before_colon = &source[..colon_at];
    let module_name = before_colon
        .rsplit(|c: char| !c.is_alphanumeric() && c != '_')
        .next()
        .unwrap_or("")
        .to_string();
    if module_name.is_empty() {
        return None;
    }
    let after_colon = &source[colon_at + 2..];
    let partial = after_colon
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .next()
        .unwrap_or("")
        .to_string();
    Some((module_name, partial))
}

fn extract_type_from_span(
    source: &str,
    offset: usize,
    length: usize,
    is_let: bool,
) -> Option<String> {
    let text = &source[offset..(offset + length)];
    let type_text = if is_let {
        let colon = text.find(':')?;
        let after_colon = &text[colon + 1..];
        let eq = after_colon.find('=').unwrap_or(after_colon.len());
        after_colon[..eq].trim().to_string()
    } else {
        text.split(':').nth(1)?.trim().to_string()
    };
    if type_text.is_empty() {
        None
    } else {
        Some(type_text)
    }
}

fn definition_detail(
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

fn import_edit_range(line_index: &LineIndex, source: &str) -> Range {
    let mut offset = 0usize;
    for line in source.lines() {
        if line.trim_start().starts_with("import ") {
            offset += line.len() + 1;
        } else {
            break;
        }
    }
    let pos = position_at(line_index, offset.min(source.len()));
    Range::new(pos, pos)
}

fn current_imports(source: &str) -> HashSet<String> {
    source
        .lines()
        .filter_map(|line| line.trim().strip_prefix("import "))
        .map(|s| s.trim_end_matches(';').trim().to_string())
        .collect()
}

fn analyze_with_diagnostics(
    path: &Path,
    source: &str,
    items: &[Item],
    module_table: &ModuleTable,
) -> std::result::Result<Arc<Analysis>, Vec<SourceDiagnostic>> {
    let name = path.to_string_lossy();
    let (result, _warnings) =
        vinyl_typecheck::typeck_with_index(items, source, &name, module_table).map_err(
            |errors| {
                errors
                    .into_iter()
                    .map(|error| SourceDiagnostic {
                        message: format!("{error}"),
                        offset: error.span.offset(),
                        length: error.span.len(),
                    })
                    .collect::<Vec<_>>()
            },
        )?;
    Ok(Arc::new(Analysis {
        path: path.to_path_buf(),
        source: source.to_string(),
        line_index: LineIndex::new(source),
        result,
    }))
}

fn parse_file_with_diagnostics(
    vfs: &Vfs,
    path: &Path,
) -> std::result::Result<(String, Vec<Item>), Vec<SourceDiagnostic>> {
    let source = match vfs.source(path) {
        Some(source) => source,
        None => {
            return Err(vec![SourceDiagnostic {
                message: format!("could not read {}", path.display()),
                offset: 0,
                length: 0,
            }]);
        }
    };
    let name = path.to_string_lossy();
    let tree = match vinyl_parser::parse_with_name(&name, &source) {
        Ok(tree) => tree,
        Err(errors) => {
            return Err(errors
                .into_iter()
                .map(|error| SourceDiagnostic {
                    message: format!("{error}"),
                    offset: error.span.offset(),
                    length: error.span.len(),
                })
                .collect());
        }
    };
    let items = match vinyl_parser::lower::lower(&tree, &source, &name) {
        Ok(items) => items,
        Err(errors) => {
            return Err(errors
                .into_iter()
                .map(|error| SourceDiagnostic {
                    message: format!("{error}"),
                    offset: error.span.offset(),
                    length: error.span.len(),
                })
                .collect());
        }
    };
    Ok((source, items))
}

fn same_file(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}

fn non_canonical_key(path: &Path, resolver: &Resolver, workspace_root: &Path) -> PathBuf {
    let source_root = match resolver.mode() {
        ResolverMode::Manifest => resolver.root().join("src"),
        ResolverMode::Script => resolver.root().to_path_buf(),
    };
    path.strip_prefix(&source_root)
        .map(|relative| workspace_root.join(relative))
        .unwrap_or_else(|_| path.to_path_buf())
}

fn relative_import_path(from_file: &Path, to_module: &Path, resolver: &Resolver) -> String {
    let to_stem = to_module
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    if let (Ok(from_canon), Ok(to_canon)) = (
        from_file.parent().unwrap().canonicalize(),
        to_module.canonicalize(),
    ) && from_canon == to_canon.parent().unwrap_or(Path::new(""))
    {
        return format!("parent::{to_stem}");
    }
    let source_root = match resolver.mode() {
        ResolverMode::Manifest => resolver.root().join("src"),
        ResolverMode::Script => resolver.root().to_path_buf(),
    };
    if let Ok(relative) = to_module.strip_prefix(&source_root) {
        let relative = relative.with_extension("");
        let relative = relative.to_string_lossy().replace('\\', "/").replace('/', "::");
        format!("parent::{relative}")
    } else {
        to_stem
    }
}

fn analyze_workspace(vfs: &Vfs, root: &Path, entry_path: &Path) -> Result<WorkspaceState> {
    let vfs_map: HashMap<PathBuf, String> = vfs.files().clone();
    let fs = Box::new(LspFileSystem::new(vfs_map));
    let mut resolver = Resolver::detect_with(root, fs).map_err(|e| eyre!("resolver error: {e}"))?;

    if let ResolverMode::Script = resolver.mode() {
        for file_path in vfs.files().keys() {
            if file_path.extension().is_some_and(|ext| ext == "vn") {
                resolver.register_module(file_path);
            }
        }
    }

    let mut module_table = ModuleTable::new();
    let mut visited = HashSet::new();
    let mut analyses = HashMap::new();
    let mut diagnostics = HashMap::new();

    match parse_file_with_diagnostics(vfs, entry_path) {
        Ok((entry_source, entry_items)) => {
            let mut all_items = entry_items.clone();
            collect_modules(
                vfs,
                &mut resolver,
                entry_path,
                &entry_items,
                &mut all_items,
                &mut module_table,
                &mut visited,
                &mut diagnostics,
            );
            match analyze_with_diagnostics(entry_path, &entry_source, &all_items, &module_table) {
                Ok(analysis) => {
                    analyses.insert(entry_path.to_path_buf(), analysis);
                }
                Err(error) => {
                    diagnostics.insert(entry_path.to_path_buf(), error);
                }
            }
            for path in &visited {
                let (source, items) = match parse_file_with_diagnostics(vfs, path) {
                    Ok(result) => result,
                    Err(file_diagnostics) => {
                        diagnostics
                            .insert(non_canonical_key(path, &resolver, root), file_diagnostics);
                        continue;
                    }
                };
                match analyze_with_diagnostics(path, &source, &items, &ModuleTable::new()) {
                    Ok(analysis) => {
                        let key = non_canonical_key(path, &resolver, root);
                        analyses.insert(key, analysis);
                    }
                    Err(error) => {
                        diagnostics.insert(non_canonical_key(path, &resolver, root), error);
                    }
                }
            }
        }
        Err(entry_diagnostics) => {
            diagnostics.insert(entry_path.to_path_buf(), entry_diagnostics);
        }
    }
    for info in resolver.all_modules().values() {
        let canonical_path = &info.file_path;
        if visited.contains(canonical_path) || canonical_path == entry_path {
            continue;
        }
        let Some(source) = vfs.source(canonical_path) else {
            continue;
        };
        let name = canonical_path.to_string_lossy();
        let Ok(tree) = vinyl_parser::parse_with_name(&name, &source) else {
            continue;
        };
        let Ok(items) = vinyl_parser::lower::lower(&tree, &source, &name) else {
            continue;
        };
        if let Ok(analysis) =
            analyze_with_diagnostics(canonical_path, &source, &items, &ModuleTable::new())
        {
            let key = non_canonical_key(canonical_path, &resolver, root);
            analyses.insert(key, analysis);
        }
    }
    Ok((analyses, diagnostics, resolver, module_table))
}

fn collect_modules(
    vfs: &Vfs,
    resolver: &mut Resolver,
    from: &Path,
    items: &[Item],
    all_items: &mut Vec<Item>,
    module_table: &mut ModuleTable,
    visited: &mut HashSet<PathBuf>,
    diagnostics: &mut HashMap<PathBuf, Vec<SourceDiagnostic>>,
) {
    for item in items.iter().filter_map(|item| match item {
        Item::Import(def) => Some(def),
        _ => None,
    }) {
        let info = if item.prefix.is_empty() {
            match resolver.resolve_module_path(&item.path) {
                Ok(info) => info,
                Err(err) => {
                    diagnostics.entry(from.to_path_buf()).or_default().push(
                        SourceDiagnostic {
                            message: format!("{err}"),
                            offset: item.span.offset(),
                            length: item.span.len(),
                        },
                    );
                    continue;
                }
            }
        } else {
            let package_count =
                item.prefix.iter().filter(|s| s.as_str() == "package").count();
            let parent_count =
                item.prefix.iter().filter(|s| s.as_str() == "parent").count();
            let total = item.prefix.len();
            if total != package_count + parent_count {
                diagnostics.entry(from.to_path_buf()).or_default().push(
                    SourceDiagnostic {
                        message:
                            "`self::` prefix refers to the current file, not an external module; \
                             use `parent::` for relative imports"
                                .to_string(),
                        offset: item.span.offset(),
                        length: item.span.len(),
                    },
                );
                continue;
            }
            let p = if package_count > 0 {
                ImportPrefix::Package
            } else if parent_count == 1 {
                ImportPrefix::Self_
            } else {
                ImportPrefix::Parent(parent_count - 1)
            };
            let path_strs: Vec<&str> = item.path.iter().map(|s| s.as_str()).collect();
            match resolver.resolve(&p, &path_strs, from) {
                Ok(info) => info,
                Err(err) => {
                    diagnostics.entry(from.to_path_buf()).or_default().push(
                        SourceDiagnostic {
                            message: format!("{err}"),
                            offset: item.span.offset(),
                            length: item.span.len(),
                        },
                    );
                    continue;
                }
            }
        };
        let path = info
            .file_path
            .canonicalize()
            .unwrap_or(info.file_path.clone());
        if !visited.insert(path.clone()) {
            continue;
        }
        let (_, module_items) = match parse_file_with_diagnostics(vfs, &path) {
            Ok(result) => result,
            Err(file_diagnostics) => {
                diagnostics.entry(path).or_default().extend(file_diagnostics);
                continue;
            }
        };
        let mut functions = Vec::new();
        let mut types = Vec::new();
        for module_item in &module_items {
            match module_item {
                Item::Function(function) if function.public => {
                    functions.push(function.clone());
                    let mut imported = function.clone();
                    imported.name = format!("{}::{}", info.import_name, imported.name);
                    all_items.push(Item::Function(imported));
                }
                Item::Struct(structure) if structure.public => {
                    types.push(structure.name.clone());
                    all_items.push(module_item.clone());
                }
                Item::TupleStruct(tuple) if tuple.public => {
                    types.push(tuple.name.clone());
                    all_items.push(module_item.clone());
                }
                Item::Enum(enumeration) if enumeration.public => {
                    types.push(enumeration.name.clone());
                    all_items.push(module_item.clone());
                }
                _ => {}
            }
        }
        module_table.insert(
            info.import_name.clone(),
            ModuleExports {
                import_name: info.import_name.clone(),
                functions,
                types,
            },
        );
        collect_modules(
            vfs,
            resolver,
            &info.file_path,
            &module_items,
            all_items,
            module_table,
            visited,
            diagnostics,
        );
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    init_tracing(cli.verbose)?;
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let (service, socket) = LspService::new(|client| Backend {
        client,
        state: Arc::new(RwLock::new(State::default())),
    });
    info!("Starting Vinyl Language Server...");
    Server::new(stdin, stdout, socket).serve(service).await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positions_use_utf16_columns() {
        let source = "😀value";
        let line_index = LineIndex::new(source);
        assert_eq!(offset_at(&line_index, Position::new(0, 2)), 4);
        assert_eq!(position_at(&line_index, 4), Position::new(0, 2));
    }
}
