use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

use line_index::LineIndex;
use tokio::sync::RwLock;
use tower_lsp::Client;
use tower_lsp::lsp_types::{Location, Url};
use vinyl_resolver::resolver::Resolver;
use vinyl_typecheck::module::ModuleTable;
use vinyl_typecheck::{Definition, SourceSpan};

use crate::position::span_range;
use crate::text::name_range;
use crate::vfs::Vfs;

pub(crate) struct Analysis {
    pub(crate) path: PathBuf,
    pub(crate) source: String,
    pub(crate) line_index: LineIndex,
    pub(crate) result: vinyl_typecheck::TypeckResult,
}

#[derive(Debug, Clone)]
pub(crate) struct PublicSymbol {
    pub(crate) path: PathBuf,
    pub(crate) span: SourceSpan,
}

#[derive(Clone)]
pub(crate) struct SourceDiagnostic {
    pub(crate) message: String,
    pub(crate) offset: usize,
    pub(crate) length: usize,
}

pub(crate) type WorkspaceAnalyses = HashMap<PathBuf, Arc<Analysis>>;
pub(crate) type WorkspaceDiagnostics = HashMap<PathBuf, Vec<SourceDiagnostic>>;
pub(crate) type WorkspaceSymbols = HashMap<String, PublicSymbol>;
pub(crate) type WorkspaceState = (
    WorkspaceAnalyses,
    WorkspaceDiagnostics,
    Resolver,
    ModuleTable,
    WorkspaceSymbols,
    HashMap<String, PathBuf>,
);

#[derive(Default)]
pub(crate) struct State {
    pub(crate) vfs: Vfs,
    pub(crate) cache: HashMap<PathBuf, Arc<Analysis>>,
    pub(crate) workspace_root: Option<PathBuf>,
    pub(crate) update_version: u64,
    pub(crate) resolver: Option<Resolver>,
    pub(crate) module_table: ModuleTable,
    pub(crate) publics: WorkspaceSymbols,
    pub(crate) modules: HashMap<String, PathBuf>,
    pub(crate) diagnostic_files: HashSet<PathBuf>,
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

    pub(crate) async fn definition_source(&self, definition: &Definition) -> Option<String> {
        let module = definition.name.split_once("::")?.0;
        let path = self.state.read().await.modules.get(module)?.clone();
        self.state
            .read()
            .await
            .cache
            .get(&path)
            .map(|analysis| analysis.source.clone())
    }

    pub(crate) async fn workspace_locations(&self, target: &Definition) -> Vec<Location> {
        self.analyses()
            .await
            .into_iter()
            .flat_map(|analysis| {
                let Some(uri) = Url::from_file_path(&analysis.path).ok() else {
                    return Vec::new();
                };
                let line_index = &analysis.line_index;
                let mut locations = Vec::new();
                for (offset, referenced) in analysis.result.references.iter() {
                    if referenced.span != target.span {
                        continue;
                    }
                    let (start, end) = match referenced.name.rsplit_once("::") {
                        Some((_, last)) => {
                            let end = *offset + referenced.name.len();
                            (end - last.len(), end)
                        }
                        None => (*offset, *offset + referenced.name.len()),
                    };
                    locations.push(Location::new(
                        uri.clone(),
                        span_range(line_index, start, end - start),
                    ));
                }
                if let Some(definition) = analysis
                    .result
                    .definitions
                    .values()
                    .flatten()
                    .find(|d| d.span == target.span && !d.name.contains("::"))
                {
                    let (start, end) = name_range(
                        &analysis.source,
                        (
                            definition.span.offset(),
                            definition.span.offset() + definition.span.len(),
                        ),
                        &definition.name,
                    );
                    locations.push(Location::new(
                        uri.clone(),
                        span_range(line_index, start, end - start),
                    ));
                }
                locations
            })
            .collect()
    }
}
