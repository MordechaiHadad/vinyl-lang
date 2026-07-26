use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use eyre::{Result, eyre};
use tokio::sync::RwLock;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

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
    source: String,
    result: vinyl_typecheck::TypeckResult,
}

#[derive(Default)]
struct State {
    vfs: Vfs,
    cache: HashMap<PathBuf, Arc<Analysis>>,
}

struct Backend {
    client: Client,
    state: Arc<RwLock<State>>,
}

impl Backend {
    async fn update(&self, uri: &Url) {
        let Some(path) = uri.to_file_path().ok() else {
            return;
        };
        let mut state = self.state.write().await;
        let Some(source) = state.vfs.source(&path) else {
            return;
        };
        let name = path.to_string_lossy().to_string();
        match analyze(&source, &name) {
            Ok(analysis) => {
                state.cache.insert(path, analysis);
                drop(state);
                self.client
                    .publish_diagnostics(uri.clone(), Vec::new(), None)
                    .await;
            }
            Err(error) => {
                let diagnostics = vec![Diagnostic::new_simple(Range::default(), error.to_string())];
                drop(state);
                self.client
                    .publish_diagnostics(uri.clone(), diagnostics, None)
                    .await;
            }
        }
    }

    async fn analysis(&self, uri: &Url) -> Option<Arc<Analysis>> {
        let path = uri.to_file_path().ok()?;
        self.state.read().await.cache.get(&path).cloned()
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(
        &self,
        _: InitializeParams,
    ) -> tower_lsp::jsonrpc::Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
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
        Ok(Some(GotoDefinitionResponse::Scalar(Location::new(
            uri,
            span_range(
                &analysis.source,
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
    source
        .split_inclusive('\n')
        .take(position.line as usize)
        .map(str::len)
        .sum::<usize>()
        .saturating_add(position.character as usize)
        .min(source.len())
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
    Position::new(line as u32, offset.saturating_sub(line_start) as u32)
}

fn analyze(source: &str, name: &str) -> Result<Arc<Analysis>> {
    let tree = vinyl_parser::parse_with_name(name, source).map_err(|errors| {
        eyre!(
            errors
                .into_iter()
                .map(|error| error.message)
                .collect::<Vec<_>>()
                .join("\n")
        )
    })?;
    let items = vinyl_parser::lower::lower(&tree, source, name).map_err(|errors| {
        eyre!(
            errors
                .into_iter()
                .map(|error| error.message)
                .collect::<Vec<_>>()
                .join("\n")
        )
    })?;
    let mut warnings = Vec::new();
    let result = vinyl_typecheck::typeck_with_index(
        &items,
        source,
        name,
        &mut warnings,
        &Default::default(),
    )
    .map_err(|errors| {
        eyre!(
            errors
                .into_iter()
                .map(|error| error.message)
                .collect::<Vec<_>>()
                .join("\n")
        )
    })?;
    Ok(Arc::new(Analysis {
        source: source.to_string(),
        result,
    }))
}

#[tokio::main]
async fn main() -> Result<()> {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let (service, socket) = LspService::new(|client| Backend {
        client,
        state: Arc::new(RwLock::new(State::default())),
    });
    Server::new(stdin, stdout, socket).serve(service).await;
    Ok(())
}
