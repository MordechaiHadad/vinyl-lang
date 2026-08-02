use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use line_index::LineIndex;
use tokio::sync::RwLock;
use tower_lsp::lsp_types::notification::Progress;
use tower_lsp::lsp_types::*;
use tower_lsp::Client;
use tracing::{debug, info};
use vinyl_typecheck::DefinitionKind;

use crate::position::span_range;
use crate::backend::state::{Backend, State};
use crate::backend::workspace::{analyze_workspace, same_file};

impl Backend {
    pub(crate) async fn schedule_update(&self, uri: &Url) {
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
}

pub(crate) async fn perform_update(state: &Arc<RwLock<State>>, client: &Client, uri: &Url) {
    debug!(%uri, "performing update");
    let update_version = state.read().await.update_version;
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
        let manifest = root.join("vinyl.toml").exists();
        let candidates = if manifest {
            [root.join("src/main.vn"), root.join("src/lib.vn")]
        } else {
            [root.join("main.vn"), root.join("lib.vn")]
        };
        let entry_path = candidates
            .into_iter()
            .find(|candidate| guard.vfs.source(candidate).is_some() || candidate.exists())
            .or_else(|| {
                path.ancestors().skip(1).find_map(|directory| {
                    [directory.join("main.vn"), directory.join("lib.vn")]
                        .into_iter()
                        .find(|candidate| {
                            guard.vfs.source(candidate).is_some() || candidate.exists()
                        })
                })
            })
            .unwrap_or(path.clone());
        (guard.vfs.clone(), root, entry_path)
    };

    match analyze_workspace(&vfs, &root, &entry_path) {
        Ok((analyses, diagnostics, resolver, module_table, publics, modules)) => {
            if state.read().await.update_version != update_version {
                return;
            }
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

            let current_diagnostic_files: Vec<PathBuf> = diagnostics
                .iter()
                .filter(|(_, file_diags)| !file_diags.is_empty())
                .map(|(file_path, _)| file_path.clone())
                .collect();
            let entry_has_diagnostics = !entry_diagnostics.is_empty();

            let should_clear_changed_file = {
                let mut guard = state.write().await;
                let should_clear = guard
                    .diagnostic_files
                    .iter()
                    .any(|file_path| same_file(file_path, &path))
                    && !current_diagnostic_files
                        .iter()
                        .any(|file_path| same_file(file_path, &path));
                guard.resolver = Some(resolver);
                guard.module_table = module_table;
                guard.publics = publics;
                guard.modules = modules;
                guard.cache.extend(analyses);
                guard.diagnostic_files = current_diagnostic_files.iter().cloned().collect();
                if entry_has_diagnostics {
                    guard.diagnostic_files.insert(entry_path.clone());
                }
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
                should_clear
            };

            client
                .publish_diagnostics(
                    Url::from_file_path(&entry_path).unwrap_or(uri.clone()),
                    entry_diagnostics,
                    None,
                )
                .await;

            for (file_path, file_diags) in &diagnostics {
                if file_path == &entry_path {
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

            if should_clear_changed_file {
                client
                    .publish_diagnostics(uri.clone(), Vec::new(), None)
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
