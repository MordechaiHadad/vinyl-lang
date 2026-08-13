use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use line_index::LineIndex;
use tokio::sync::RwLock;
use tower_lsp::Client;
use tower_lsp::lsp_types::notification::Progress;
use tower_lsp::lsp_types::*;
use tracing::{debug, info};

use crate::backend::state::{Backend, State};
use crate::backend::workspace::analyze_workspace;
use crate::position::span_range;

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
            let _progress_guard = ProgressGuard {
                client: client.clone(),
                token,
            };
            perform_update(&state, &client, &uri).await;
        });
    }
}

struct ProgressGuard {
    client: Client,
    token: ProgressToken,
}

impl Drop for ProgressGuard {
    fn drop(&mut self) {
        let client = self.client.clone();
        let token = self.token.clone();
        tokio::spawn(async move {
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
        Ok((analyses, diagnostics, resolver, module_table, publics, modules, files)) => {
            if state.read().await.update_version != update_version {
                return;
            }
            info!(files = analyses.len(), "workspace analysis complete");
            let entry_source = vfs.source(&entry_path).unwrap_or_default();
            let entry_line_index = LineIndex::new(&entry_source);
            let entry_file_id = files.get(&entry_path);
            let entry_diagnostics: Vec<Diagnostic> = entry_file_id
                .and_then(|file_id| diagnostics.get(&file_id))
                .map(|diags| {
                    diags
                        .iter()
                        .map(|d| Diagnostic {
                            range: span_range(&entry_line_index, d.offset, d.length),
                            severity: Some(if d.warning {
                                DiagnosticSeverity::WARNING
                            } else {
                                DiagnosticSeverity::ERROR
                            }),
                            message: d.message.clone(),
                            ..Diagnostic::default()
                        })
                        .collect()
                })
                .unwrap_or_default();

            let current_diagnostic_files: Vec<_> = diagnostics
                .iter()
                .filter(|(_, file_diags)| !file_diags.is_empty())
                .map(|(file_id, _)| *file_id)
                .collect();
            let entry_has_diagnostics = !entry_diagnostics.is_empty();

            let should_clear_changed_file = {
                let mut guard = state.write().await;
                let changed_file_id = guard.files.intern(&path);
                let should_clear = guard
                    .diagnostic_files
                    .iter()
                    .any(|file_id| *file_id == changed_file_id)
                    && !current_diagnostic_files.contains(&changed_file_id);
                guard.resolver = Some(resolver);
                guard.module_table = module_table;
                guard.publics = publics;
                guard.modules = modules;
                guard.files = files.clone();
                guard.cache.extend(analyses);
                guard.diagnostic_files = current_diagnostic_files.iter().cloned().collect();
                if entry_has_diagnostics && let Some(entry_file_id) = guard.files.get(&entry_path) {
                    guard.diagnostic_files.insert(entry_file_id);
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

            for (file_id, file_diags) in &diagnostics {
                if Some(*file_id) == entry_file_id {
                    continue;
                }
                let Some(file_path) = files.path(*file_id).map(Path::to_path_buf) else {
                    continue;
                };
                let source = vfs.source(&file_path).unwrap_or_default();
                let line_index = LineIndex::new(&source);
                let diags: Vec<Diagnostic> = file_diags
                    .iter()
                    .map(|d| Diagnostic {
                        range: span_range(&line_index, d.offset, d.length),
                        severity: Some(if d.warning {
                            DiagnosticSeverity::WARNING
                        } else {
                            DiagnosticSeverity::ERROR
                        }),
                        message: d.message.clone(),
                        ..Diagnostic::default()
                    })
                    .collect();
                client
                    .publish_diagnostics(
                        Url::from_file_path(&file_path).unwrap_or(uri.clone()),
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
