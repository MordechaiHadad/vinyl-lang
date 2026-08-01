use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use line_index::LineIndex;
use tokio::sync::RwLock;
use tower_lsp::lsp_types::{Location, Url};
use tower_lsp::Client;
use vinyl_resolver::Resolver;
use vinyl_typecheck::module::ModuleTable;

use crate::position::span_range;
use crate::vfs::Vfs;

pub(crate) struct Analysis {
    pub(crate) path: PathBuf,
    pub(crate) source: String,
    pub(crate) line_index: LineIndex,
    pub(crate) result: vinyl_typecheck::TypeckResult,
}

#[derive(Clone)]
pub(crate) struct SourceDiagnostic {
    pub(crate) message: String,
    pub(crate) offset: usize,
    pub(crate) length: usize,
}

pub(crate) type WorkspaceAnalyses = HashMap<PathBuf, Arc<Analysis>>;
pub(crate) type WorkspaceDiagnostics = HashMap<PathBuf, Vec<SourceDiagnostic>>;
pub(crate) type WorkspaceState = (
    WorkspaceAnalyses,
    WorkspaceDiagnostics,
    Resolver,
    ModuleTable,
);

#[derive(Default)]
pub(crate) struct State {
    pub(crate) vfs: Vfs,
    pub(crate) cache: HashMap<PathBuf, Arc<Analysis>>,
    pub(crate) workspace_root: Option<PathBuf>,
    pub(crate) update_version: u64,
    pub(crate) resolver: Option<Resolver>,
    pub(crate) module_table: ModuleTable,
}

pub(crate) struct Backend {
    pub(crate) client: Client,
    pub(crate) state: Arc<RwLock<State>>,
}

impl Backend {
    pub(crate) async fn analysis(&self, uri: &Url) -> Option<Arc<Analysis>> {
        let path = uri.to_file_path().ok()?;
        self.state.read().await.cache.get(&path).cloned()
    }

    pub(crate) async fn analyses(&self) -> Vec<Arc<Analysis>> {
        self.state.read().await.cache.values().cloned().collect()
    }

    pub(crate) async fn workspace_locations(&self, target: &vinyl_typecheck::Definition) -> Vec<Location> {
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
