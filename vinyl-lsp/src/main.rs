use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use clap::{ArgAction, Parser};
use eyre::{Result, eyre};
use tokio::sync::RwLock;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};
use tracing::{debug, info};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::prelude::*;
use vinyl_parser::ast::item::{ImportDef, Item};
use vinyl_typecheck::module::{ModuleExports, ModuleTable};

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
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn"))
    };
    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().with_target(false))
        .try_init()?;
    Ok(())
}

#[derive(Default)]
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
type WorkspaceResult = (WorkspaceAnalyses, WorkspaceDiagnostics);

#[derive(Default)]
struct State {
    vfs: Vfs,
    cache: HashMap<PathBuf, Arc<Analysis>>,
    workspace_root: Option<PathBuf>,
}

struct Backend {
    client: Client,
    state: Arc<RwLock<State>>,
}

impl Backend {
    async fn update(&self, uri: &Url) {
        debug!(%uri, "updating workspace analysis");
        let Some(path) = uri.to_file_path().ok() else {
            return;
        };
        let mut state = self.state.write().await;
        if state.vfs.source(&path).is_none() {
            return;
        }
        let root = state
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
        match analyze_workspace(&state.vfs, &root, &entry_path) {
            Ok((analyses, diagnostics)) => {
                info!(files = analyses.len(), "workspace analysis complete");
                state.cache.extend(analyses);
                drop(state);
                for (diagnostic_path, file_diagnostics) in &diagnostics {
                    let source = state_source(&self.state, diagnostic_path).await;
                    let lsp_diagnostics = file_diagnostics
                        .iter()
                        .map(|diagnostic| {
                            Diagnostic::new_simple(
                                span_range(&source, diagnostic.offset, diagnostic.length),
                                diagnostic.message.clone(),
                            )
                        })
                        .collect();
                    self.client
                        .publish_diagnostics(
                            Url::from_file_path(diagnostic_path).unwrap_or(uri.clone()),
                            lsp_diagnostics,
                            None,
                        )
                        .await;
                }
                if !diagnostics.contains_key(&entry_path) {
                    self.client
                        .publish_diagnostics(
                            Url::from_file_path(&entry_path).unwrap_or(uri.clone()),
                            Vec::new(),
                            None,
                        )
                        .await;
                }
            }
            Err(error) => {
                let diagnostics = vec![Diagnostic::new_simple(Range::default(), error.to_string())];
                drop(state);
                self.client
                    .publish_diagnostics(
                        Url::from_file_path(&entry_path).unwrap_or(uri.clone()),
                        diagnostics,
                        None,
                    )
                    .await;
            }
        }
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
        self.update(&params.text_document.uri).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.into_iter().next()
            && let Ok(path) = params.text_document.uri.to_file_path()
        {
            self.state.write().await.vfs.set(path, change.text);
        }
        self.update(&params.text_document.uri).await;
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
        let Some(analysis) = self
            .analysis(&params.text_document_position.text_document.uri)
            .await
        else {
            return Ok(None);
        };
        let offset = offset_at(&analysis.source, params.text_document_position.position);
        let prefix = word_prefix(&analysis.source, offset);
        let items = analysis
            .result
            .definitions
            .values()
            .flatten()
            .filter(|definition| definition.name.starts_with(&prefix))
            .map(|definition| definition.name.clone())
            .collect::<HashSet<_>>()
            .into_iter()
            .map(|name| CompletionItem {
                label: name,
                kind: Some(CompletionItemKind::VARIABLE),
                ..CompletionItem::default()
            })
            .collect();
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
                            "{}({})",
                            function.name,
                            function
                                .params
                                .iter()
                                .map(|param| format!("{}: {:?}", param.name, param.type_))
                                .collect::<Vec<_>>()
                                .join(", ")
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
        let Ok(formatted) = vinyl_formatter::format_source(&source) else {
            return Ok(None);
        };
        Ok(Some(vec![CodeActionOrCommand::CodeAction(CodeAction {
            title: "Format document".to_string(),
            kind: Some(CodeActionKind::SOURCE_FIX_ALL),
            diagnostics: None,
            edit: Some(WorkspaceEdit {
                changes: Some(HashMap::from([(
                    uri,
                    vec![TextEdit::new(full_range(&source), formatted)],
                )])),
                ..WorkspaceEdit::default()
            }),
            command: None,
            is_preferred: Some(true),
            disabled: None,
            data: None,
        })]))
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

fn analyze_with_diagnostics(
    path: &Path,
    source: &str,
    items: &[Item],
    module_table: &ModuleTable,
) -> std::result::Result<Arc<Analysis>, Vec<SourceDiagnostic>> {
    let name = path.to_string_lossy();
    let mut warnings = Vec::new();
    let result =
        vinyl_typecheck::typeck_with_index(items, source, &name, &mut warnings, module_table)
            .map_err(|errors| {
                errors
                    .into_iter()
                    .map(|error| SourceDiagnostic {
                        message: error.message,
                        offset: error.span.offset(),
                        length: error.span.len(),
                    })
                    .collect::<Vec<_>>()
            })?;
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
                .map(|error| error.message)
                .collect::<Vec<_>>()
                .join("\n")
        )
    })?;
    let items = vinyl_parser::lower::lower(&tree, &source, &name).map_err(|errors| {
        eyre!(
            errors
                .into_iter()
                .map(|error| error.message)
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
                    message: error.message,
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
                    message: error.message,
                    offset: error.span.offset(),
                    length: error.span.len(),
                })
                .collect());
        }
    };
    Ok((source, items))
}

fn analyze_workspace(vfs: &Vfs, root: &Path, entry_path: &Path) -> Result<WorkspaceResult> {
    let resolver = vinyl_resolver::ModuleResolver::new(root)?;
    let (entry_source, entry_items) = match parse_file_with_diagnostics(vfs, entry_path) {
        Ok(result) => result,
        Err(diagnostics) => {
            return Ok((
                HashMap::new(),
                HashMap::from([(entry_path.to_path_buf(), diagnostics)]),
            ));
        }
    };
    let mut all_items = entry_items.clone();
    let mut module_table = ModuleTable::new();
    let mut visited = HashSet::new();
    collect_modules(
        vfs,
        &resolver,
        &entry_items,
        &mut all_items,
        &mut module_table,
        &mut visited,
    )?;
    let mut analyses = HashMap::new();
    match analyze_with_diagnostics(entry_path, &entry_source, &all_items, &module_table) {
        Ok(analysis) => {
            analyses.insert(entry_path.to_path_buf(), analysis);
        }
        Err(error) => {
            return Ok((analyses, HashMap::from([(entry_path.to_path_buf(), error)])));
        }
    }
    let mut diagnostics = HashMap::new();
    for path in visited {
        let (source, items) = match parse_file_with_diagnostics(vfs, &path) {
            Ok(result) => result,
            Err(file_diagnostics) => {
                diagnostics.insert(path.clone(), file_diagnostics);
                continue;
            }
        };
        match analyze_with_diagnostics(&path, &source, &items, &ModuleTable::new()) {
            Ok(analysis) => {
                analyses.insert(path.clone(), analysis);
            }
            Err(error) => {
                diagnostics.insert(path.clone(), error);
            }
        }
    }
    Ok((analyses, diagnostics))
}

async fn state_source(state: &Arc<RwLock<State>>, path: &Path) -> String {
    state.read().await.vfs.source(path).unwrap_or_default()
}

fn collect_modules(
    vfs: &Vfs,
    resolver: &vinyl_resolver::ModuleResolver,
    items: &[Item],
    all_items: &mut Vec<Item>,
    module_table: &mut ModuleTable,
    visited: &mut HashSet<PathBuf>,
) -> Result<()> {
    for import in items.iter().filter_map(|item| match item {
        Item::Import(ImportDef { path, .. }) => Some(path),
        _ => None,
    }) {
        let info = resolver.resolve(import)?;
        let path = info.file_path.clone();
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
