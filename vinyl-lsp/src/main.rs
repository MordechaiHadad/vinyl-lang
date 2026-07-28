use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use clap::{ArgAction, Parser};
use eyre::{Result, eyre};
use tokio::sync::RwLock;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};
use tracing::{debug, info};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::prelude::*;
use vinyl_parser::ast::item::{ImportDef, Item};
use vinyl_resolver::ModuleResolver;
use vinyl_typecheck::hir::{HirFunction, HirItemKind};
use vinyl_typecheck::module::{ModuleExports, ModuleTable};
use vinyl_typecheck::{Definition, DefinitionKind, TypeckResult};

#[derive(Parser)]
#[command(name = "vinyl-lsp", version, about = "Vinyl language server")]
struct Cli {
    /// Increase verbosity (-v for DEBUG, -vv for TRACE, -vvv for global TRACE)
    #[arg(short = 'v', long = "verbose", action = ArgAction::Count)]
    verbose: u8,
}

fn init_tracing(verbose: u8) -> Result<()> {
    let filter = if verbose > 0 {
        let crate_name = env!("CARGO_CRATE_NAME");
        match verbose {
            1 => EnvFilter::new(format!("{crate_name}=debug")),
            2 => EnvFilter::new(format!("{crate_name}=trace")),
            _ => EnvFilter::new("trace"),
        }
    } else {
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"))
    };
    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().with_target(false))
        .try_init()?;
    Ok(())
}

#[derive(Default, Clone)]
struct Vfs {
    files: HashMap<PathBuf, String>,
}

impl Vfs {
    fn set(&mut self, path: PathBuf, source: String) {
        self.files.insert(path, source);
    }

    fn remove(&mut self, path: &Path) {
        self.files.remove(path);
    }

    fn source(&self, path: &Path) -> Option<String> {
        self.files
            .get(path)
            .cloned()
            .or_else(|| std::fs::read_to_string(path).ok())
    }
}

struct Analysis {
    path: PathBuf,
    source: String,
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
    ModuleResolver,
    ModuleTable,
);

#[derive(Default)]
struct State {
    vfs: Vfs,
    cache: HashMap<PathBuf, Arc<Analysis>>,
    workspace_root: Option<PathBuf>,
    update_version: u64,
    resolver: Option<ModuleResolver>,
    module_table: ModuleTable,
}

struct Backend {
    client: Client,
    state: Arc<RwLock<State>>,
}

const KEYWORDS: &[(&str, CompletionItemKind)] = &[
    ("fn", CompletionItemKind::KEYWORD),
    ("let", CompletionItemKind::KEYWORD),
    ("mut", CompletionItemKind::KEYWORD),
    ("return", CompletionItemKind::KEYWORD),
    ("if", CompletionItemKind::KEYWORD),
    ("else", CompletionItemKind::KEYWORD),
    ("match", CompletionItemKind::KEYWORD),
    ("while", CompletionItemKind::KEYWORD),
    ("loop", CompletionItemKind::KEYWORD),
    ("break", CompletionItemKind::KEYWORD),
    ("continue", CompletionItemKind::KEYWORD),
    ("import", CompletionItemKind::KEYWORD),
    ("public", CompletionItemKind::KEYWORD),
    ("true", CompletionItemKind::KEYWORD),
    ("false", CompletionItemKind::KEYWORD),
    ("unit", CompletionItemKind::KEYWORD),
    ("not", CompletionItemKind::KEYWORD),
    ("and", CompletionItemKind::KEYWORD),
    ("or", CompletionItemKind::KEYWORD),
    ("struct", CompletionItemKind::KEYWORD),
    ("enum", CompletionItemKind::KEYWORD),
    ("tuple", CompletionItemKind::KEYWORD),
    ("int", CompletionItemKind::KEYWORD),
    ("float", CompletionItemKind::KEYWORD),
    ("bool", CompletionItemKind::KEYWORD),
    ("char", CompletionItemKind::KEYWORD),
    ("string", CompletionItemKind::KEYWORD),
    ("int8", CompletionItemKind::KEYWORD),
    ("int16", CompletionItemKind::KEYWORD),
    ("int32", CompletionItemKind::KEYWORD),
    ("int64", CompletionItemKind::KEYWORD),
    ("int128", CompletionItemKind::KEYWORD),
    ("isize", CompletionItemKind::KEYWORD),
    ("uint8", CompletionItemKind::KEYWORD),
    ("uint16", CompletionItemKind::KEYWORD),
    ("uint32", CompletionItemKind::KEYWORD),
    ("uint64", CompletionItemKind::KEYWORD),
    ("uint128", CompletionItemKind::KEYWORD),
    ("usize", CompletionItemKind::KEYWORD),
    ("float32", CompletionItemKind::KEYWORD),
    ("float64", CompletionItemKind::KEYWORD),
];

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
            perform_update(&state, &client, &uri).await;
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
                let source = analysis.source.clone();
                let reference_uri = uri.clone();
                let reference_source = source.clone();
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
                    .filter_map(move |(offset, definition)| {
                        reference_uri.clone().map(|uri| {
                            Location::new(
                                uri,
                                span_range(&reference_source, *offset, definition.name.len()),
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
                        span_range(&source, definition.span.offset(), definition.name.len()),
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
                completion_provider: Some(CompletionOptions::default()),
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
            &analysis.source,
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
            &analysis.source,
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
        Ok(Some(GotoDefinitionResponse::Scalar(Location::new(
            Url::from_file_path(&target_path).unwrap_or(uri),
            span_range(
                &target_source,
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
        let Some(analysis) = self.analysis(uri).await else {
            return Ok(None);
        };
        let offset = offset_at(&analysis.source, params.text_document_position.position);
        let prefix = word_prefix(&analysis.source, offset);

        let mut items: Vec<CompletionItem> = Vec::new();

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

        let state = self.state.read().await;
        if let Some(resolver) = &state.resolver {
            let current_path = uri.to_file_path().ok();
            let existing_imports = current_imports(&analysis.source);
            let workspace_root = state
                .workspace_root
                .as_deref()
                .unwrap_or(resolver.source_root());
            let current_dir_canonical = current_path
                .as_ref()
                .and_then(|p| p.parent())
                .and_then(|d| std::fs::canonicalize(d).ok());

            for info in resolver.all_modules().values() {
                let import_name = &info.import_name;
                if existing_imports.contains(import_name) {
                    continue;
                }
                if current_path
                    .as_ref()
                    .is_some_and(|p| same_file(p, &info.file_path))
                {
                    continue;
                }
                if let Some(ref current_dir) = current_dir_canonical
                    && !info.file_path.starts_with(current_dir)
                {
                    continue;
                }
                let cache_key =
                    non_canonical_key(&info.file_path, resolver.source_root(), workspace_root);
                let Some(module_analysis) = state.cache.get(&cache_key) else {
                    continue;
                };
                let import_path = current_path
                    .as_ref()
                    .map(|p| relative_import_path(p, &info.file_path, resolver.source_root()))
                    .unwrap_or_else(|| import_name.clone());
                if existing_imports.contains(&import_path) {
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
                    let qualified = format!("{import_path}::{name}");
                    let edit_range = Range::new(
                        position_at(&analysis.source, offset.saturating_sub(prefix.len())),
                        params.text_document_position.position,
                    );
                    let import_edit =
                        TextEdit::new(import_edit_range(&analysis.source), format!("import {import_path};\n"));
                    items.push(CompletionItem {
                        label: qualified.clone(),
                        kind: Some(kind),
                        detail,
                        text_edit: Some(CompletionTextEdit::Edit(TextEdit::new(
                            edit_range,
                            qualified,
                        ))),
                        additional_text_edits: Some(vec![import_edit]),
                        ..CompletionItem::default()
                    });
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
        let offset = offset_at(&analysis.source, params.text_document_position.position);
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
        let offset = offset_at(&analysis.source, params.text_document_position.position);
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
                        range: span_range(&analysis.source, item.span.offset(), item.span.len()),
                        selection_range: span_range(
                            &analysis.source,
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
            &analysis.source,
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
        actions.push(CodeActionOrCommand::CodeAction(CodeAction {
            title: "Format document".to_string(),
            kind: Some(CodeActionKind::SOURCE_FIX_ALL),
            diagnostics: None,
            edit: Some(WorkspaceEdit {
                changes: Some(HashMap::from([(
                    uri.clone(),
                    vec![TextEdit::new(full_range(&source), formatted)],
                )])),
                ..WorkspaceEdit::default()
            }),
            command: None,
            is_preferred: Some(true),
            disabled: None,
            data: None,
        }));

        let cursor_offset = offset_at(&source, params.range.start);
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
                    let workspace_root = state
                        .workspace_root
                        .as_deref()
                        .unwrap_or(resolver.source_root());
                    let current_dir_canonical = current_path
                        .as_ref()
                        .and_then(|p| p.parent())
                        .and_then(|d| std::fs::canonicalize(d).ok());
                    for info in resolver.all_modules().values() {
                        if current_path
                            .as_ref()
                            .is_some_and(|p| same_file(p, &info.file_path))
                        {
                            continue;
                        }
                        if let Some(ref current_dir) = current_dir_canonical
                            && !info.file_path.starts_with(current_dir)
                        {
                            continue;
                        }
                        let cache_key = non_canonical_key(
                            &info.file_path,
                            resolver.source_root(),
                            workspace_root,
                        );
                        let Some(module_analysis) = state.cache.get(&cache_key) else {
                            continue;
                        };
                        let import_path = current_path
                            .as_ref()
                            .map(|p| {
                                relative_import_path(p, &info.file_path, resolver.source_root())
                            })
                            .unwrap_or_else(|| info.import_name.clone());
                        if existing_imports.contains(&import_path)
                            || existing_imports.contains(&info.import_name)
                        {
                            continue;
                        }
                        if module_analysis.result.definitions.contains_key(&prefix) {
                            let edit_range = import_edit_range(&source);
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
        Ok(Some(vec![TextEdit::new(full_range(&source), formatted)]))
    }
}

async fn perform_update(state: &Arc<RwLock<State>>, client: &Client, uri: &Url) {
    debug!(%uri, "performing update");
    let Some(path) = uri.to_file_path().ok() else {
        return;
    };

    let (vfs, root, entry_path, existing_resolver) = {
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
        let reuse = guard.resolver.as_ref().is_some_and(|resolver| {
            resolver
                .all_modules()
                .values()
                .any(|info| same_file(&info.file_path, &path))
        });
        (
            guard.vfs.clone(),
            root,
            entry_path,
            if reuse { guard.resolver.clone() } else { None },
        )
    };

    match analyze_workspace(&vfs, &root, &entry_path, existing_resolver.as_ref()) {
        Ok((analyses, diagnostics, resolver, module_table)) => {
            info!(files = analyses.len(), "workspace analysis complete");
            let entry_source = vfs.source(&entry_path).unwrap_or_default();
            let mut entry_diagnostics: Vec<Diagnostic> = diagnostics
                .get(&entry_path)
                .map(|diags| {
                    diags
                        .iter()
                        .map(|d| {
                            Diagnostic::new_simple(
                                span_range(&entry_source, d.offset, d.length),
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
                                &entry_source,
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

fn offset_at(source: &str, position: Position) -> usize {
    let line_start = source
        .split_inclusive('\n')
        .take(position.line as usize)
        .map(str::len)
        .sum::<usize>();
    let line = &source[line_start.min(source.len())..];
    line_start + utf16_offset(line, position.character as usize).min(line.len())
}

fn span_range(source: &str, offset: usize, length: usize) -> Range {
    Range::new(
        position_at(source, offset),
        position_at(source, offset + length),
    )
}

fn full_range(source: &str) -> Range {
    Range::new(Position::new(0, 0), position_at(source, source.len()))
}

fn position_at(source: &str, offset: usize) -> Position {
    let offset = offset.min(source.len());
    let line = source[..offset]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count();
    let line_start = source[..offset].rfind('\n').map_or(0, |index| index + 1);
    Position::new(
        line as u32,
        source[line_start..offset].encode_utf16().count() as u32,
    )
}

fn utf16_offset(source: &str, column: usize) -> usize {
    let mut utf16 = 0;
    for (offset, character) in source.char_indices() {
        if utf16 >= column {
            return offset;
        }
        utf16 += character.len_utf16();
    }
    source.len()
}

fn word_prefix(source: &str, offset: usize) -> String {
    let before = &source[..offset.min(source.len())];
    before
        .rsplit(|character: char| !character.is_alphanumeric() && character != '_')
        .next()
        .unwrap_or_default()
        .to_string()
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

fn import_edit_range(source: &str) -> Range {
    let mut offset = 0usize;
    for line in source.lines() {
        if line.trim_start().starts_with("import ") {
            offset += line.len() + 1;
        } else {
            break;
        }
    }
    let pos = position_at(source, offset.min(source.len()));
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
        result,
    }))
}

fn parse_file(vfs: &Vfs, path: &Path) -> Result<(String, Vec<Item>)> {
    let source = vfs
        .source(path)
        .ok_or_else(|| eyre!("could not read {}", path.display()))?;
    let name = path.to_string_lossy();
    let tree = vinyl_parser::parse_with_name(&name, &source).map_err(|errors| {
        eyre!(
            errors
                .into_iter()
                .map(|error| format!("{error}"))
                .collect::<Vec<_>>()
                .join("\n")
        )
    })?;
    let items = vinyl_parser::lower::lower(&tree, &source, &name).map_err(|errors| {
        eyre!(
            errors
                .into_iter()
                .map(|error| format!("{error}"))
                .collect::<Vec<_>>()
                .join("\n")
        )
    })?;
    Ok((source.to_string(), items))
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

fn non_canonical_key(path: &Path, canonical_root: &Path, non_canonical_root: &Path) -> PathBuf {
    path.strip_prefix(canonical_root)
        .map(|relative| non_canonical_root.join(relative))
        .unwrap_or_else(|_| path.to_path_buf())
}

fn relative_import_path(from_file: &Path, to_module: &Path, source_root: &Path) -> String {
    let to_stem = to_module
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    if let (Ok(from_canon), Ok(to_canon)) = (
        from_file.parent().unwrap().canonicalize(),
        to_module.canonicalize(),
    ) && from_canon == to_canon.parent().unwrap_or(Path::new(""))
    {
        return to_stem;
    }
    if let Ok(relative) = to_module.strip_prefix(source_root) {
        let relative = relative.with_extension("");
        relative.to_string_lossy().replace('\\', "/")
    } else {
        to_stem
    }
}

fn analyze_workspace(
    vfs: &Vfs,
    root: &Path,
    entry_path: &Path,
    existing_resolver: Option<&ModuleResolver>,
) -> Result<WorkspaceState> {
    let resolver = match existing_resolver {
        Some(r) => r.clone(),
        None => ModuleResolver::new(root)?,
    };
    let mut module_table = ModuleTable::new();
    let mut visited = HashSet::new();
    let mut analyses = HashMap::new();
    let mut diagnostics = HashMap::new();

    match parse_file_with_diagnostics(vfs, entry_path) {
        Ok((entry_source, entry_items)) => {
            let mut all_items = entry_items.clone();
            let _ = collect_modules(
                vfs,
                &resolver,
                entry_path,
                &entry_items,
                &mut all_items,
                &mut module_table,
                &mut visited,
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
                        diagnostics.insert(
                            non_canonical_key(path, resolver.source_root(), root),
                            file_diagnostics,
                        );
                        continue;
                    }
                };
                match analyze_with_diagnostics(path, &source, &items, &ModuleTable::new()) {
                    Ok(analysis) => {
                        let key = non_canonical_key(path, resolver.source_root(), root);
                        analyses.insert(key, analysis);
                    }
                    Err(error) => {
                        diagnostics
                            .insert(non_canonical_key(path, resolver.source_root(), root), error);
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
            // store with non-canonical path key so URI lookups match
            let key = non_canonical_key(canonical_path, resolver.source_root(), root);
            analyses.insert(key, analysis);
        }
    }
    Ok((analyses, diagnostics, resolver, module_table))
}

fn collect_modules(
    vfs: &Vfs,
    resolver: &vinyl_resolver::ModuleResolver,
    file_path: &Path,
    items: &[Item],
    all_items: &mut Vec<Item>,
    module_table: &mut ModuleTable,
    visited: &mut HashSet<PathBuf>,
) -> Result<()> {
    for import in items.iter().filter_map(|item| match item {
        Item::Import(ImportDef { path, .. }) => Some(path),
        _ => None,
    }) {
        let info = resolver.resolve_from_file(import, file_path)?;
        let path = info
            .file_path
            .canonicalize()
            .unwrap_or(info.file_path.clone());
        if !visited.insert(path.clone()) {
            continue;
        }
        let (_, module_items) = parse_file(vfs, &path)?;
        let mut functions = Vec::new();
        let mut types = Vec::new();
        for item in &module_items {
            match item {
                Item::Function(function) if function.public => {
                    functions.push(function.clone());
                    let mut imported = function.clone();
                    imported.name = format!("{}::{}", info.import_name, imported.name);
                    all_items.push(Item::Function(imported));
                }
                Item::Struct(structure) if structure.public => {
                    types.push(structure.name.clone());
                    all_items.push(item.clone());
                }
                Item::TupleStruct(tuple) if tuple.public => {
                    types.push(tuple.name.clone());
                    all_items.push(item.clone());
                }
                Item::Enum(enumeration) if enumeration.public => {
                    types.push(enumeration.name.clone());
                    all_items.push(item.clone());
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
            &path,
            &module_items,
            all_items,
            module_table,
            visited,
        )?;
    }
    Ok(())
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
        assert_eq!(offset_at(source, Position::new(0, 2)), 4);
        assert_eq!(position_at(source, 4), Position::new(0, 2));
    }
}
