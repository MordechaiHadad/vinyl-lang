mod backend;
mod consts;
mod position;
mod text;
mod utils;
mod vfs;

use std::sync::Arc;

use clap::Parser;
use eyre::Result;
use line_index::LineIndex;
use tokio::sync::RwLock;
use tower_lsp::lsp_types::notification::Progress;
use tower_lsp::lsp_types::*;
use tower_lsp::{LanguageServer, LspService, Server};
use tracing::info;

use crate::backend::state::{Backend, State};
use crate::backend::update::perform_update;
use crate::position::offset_at;
use crate::utils::{Cli, init_tracing};

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
                    let source_root = if root.join("vinyl.toml").exists() {
                        root.join("src")
                    } else {
                        root.clone()
                    };
                    [source_root.join("main.vn"), source_root.join("lib.vn")]
                        .into_iter()
                        .find(|p| p.exists())
                        .or_else(|| {
                            std::fs::read_dir(&source_root)
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
        if let Ok(path) = params.text_document.uri.to_file_path() {
            let mut state = self.state.write().await;
            let mut source = state.vfs.source(&path).unwrap_or_default();
            for change in params.content_changes {
                if let Some(range) = change.range {
                    let line_index = LineIndex::new(&source);
                    let start = offset_at(&line_index, range.start);
                    let end = offset_at(&line_index, range.end);
                    if start <= end
                        && end <= source.len()
                        && source.is_char_boundary(start)
                        && source.is_char_boundary(end)
                    {
                        source.replace_range(start..end, &change.text);
                    }
                } else {
                    source = change.text;
                }
            }
            state.vfs.set(path, source);
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

    async fn did_change_watched_files(&self, params: DidChangeWatchedFilesParams) {
        for change in params.changes {
            self.schedule_update(&change.uri).await;
        }
    }

    async fn hover(&self, params: HoverParams) -> tower_lsp::jsonrpc::Result<Option<Hover>> {
        self.hover(params).await
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> tower_lsp::jsonrpc::Result<Option<GotoDefinitionResponse>> {
        self.goto_definition(params).await
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
        self.completion(params).await
    }

    async fn references(
        &self,
        params: ReferenceParams,
    ) -> tower_lsp::jsonrpc::Result<Option<Vec<Location>>> {
        self.references(params).await
    }

    async fn rename(
        &self,
        params: RenameParams,
    ) -> tower_lsp::jsonrpc::Result<Option<WorkspaceEdit>> {
        self.rename(params).await
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> tower_lsp::jsonrpc::Result<Option<DocumentSymbolResponse>> {
        self.document_symbol(params).await
    }

    async fn signature_help(
        &self,
        params: SignatureHelpParams,
    ) -> tower_lsp::jsonrpc::Result<Option<SignatureHelp>> {
        self.signature_help(params).await
    }

    async fn code_action(
        &self,
        params: CodeActionParams,
    ) -> tower_lsp::jsonrpc::Result<Option<CodeActionResponse>> {
        self.code_action(params).await
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
