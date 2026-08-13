use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct FileId(usize);

impl FileId {
    pub(crate) const INVALID: Self = Self(usize::MAX);
}

#[derive(Default, Clone)]
pub(crate) struct FileInterner {
    paths: Vec<PathBuf>,
    ids: HashMap<PathBuf, FileId>,
}

impl FileInterner {
    fn key(path: &Path) -> PathBuf {
        path.canonicalize().unwrap_or_else(|_| {
            let mut normalized = PathBuf::new();
            for component in PathBuf::from(
                path.to_string_lossy()
                    .trim_start_matches(r"\\?\")
                    .to_string(),
            )
            .components()
            {
                match component {
                    std::path::Component::ParentDir => {
                        normalized.pop();
                    }
                    other => normalized.push(other.as_os_str()),
                }
            }
            normalized
        })
    }

    pub(crate) fn intern(&mut self, path: &Path) -> FileId {
        let key = Self::key(path);
        if let Some(id) = self.ids.get(&key) {
            return *id;
        }
        let id = FileId(self.paths.len());
        self.paths.push(key.clone());
        self.ids.insert(key, id);
        id
    }

    pub(crate) fn get(&self, path: &Path) -> Option<FileId> {
        self.ids.get(&Self::key(path)).copied()
    }

    pub(crate) fn path(&self, id: FileId) -> Option<&Path> {
        self.paths.get(id.0).map(PathBuf::as_path)
    }
}

#[cfg(test)]
mod tests {
    use super::FileInterner;
    use std::path::Path;

    #[test]
    fn interns_equivalent_paths_once() {
        let mut interner = FileInterner::default();
        let first = interner.intern(Path::new("target/../src/main.vn"));
        let second = interner.intern(Path::new("src/main.vn"));
        assert_eq!(first, second);
        assert_eq!(interner.path(first), Some(Path::new("src/main.vn")));
    }
}

pub(crate) struct Analysis {
    pub(crate) file_id: FileId,
    pub(crate) source: String,
    pub(crate) line_index: LineIndex,
    pub(crate) result: vinyl_typecheck::TypeckResult,
    pub(crate) warnings: Vec<SourceDiagnostic>,
}

#[derive(Debug, Clone)]
pub(crate) struct PublicSymbol {
    pub(crate) file_id: FileId,
    pub(crate) span: SourceSpan,
}

#[derive(Clone)]
pub(crate) struct SourceDiagnostic {
    pub(crate) message: String,
    pub(crate) offset: usize,
    pub(crate) length: usize,
    pub(crate) warning: bool,
}

pub(crate) type WorkspaceAnalyses = HashMap<FileId, Arc<Analysis>>;
pub(crate) type WorkspaceDiagnostics = HashMap<FileId, Vec<SourceDiagnostic>>;
pub(crate) type WorkspaceSymbols = HashMap<String, PublicSymbol>;
pub(crate) type WorkspaceState = (
    WorkspaceAnalyses,
    WorkspaceDiagnostics,
    Resolver,
    ModuleTable,
    WorkspaceSymbols,
    HashMap<String, FileId>,
    FileInterner,
);

#[derive(Default)]
pub(crate) struct State {
    pub(crate) vfs: Vfs,
    pub(crate) cache: HashMap<FileId, Arc<Analysis>>,
    pub(crate) workspace_root: Option<PathBuf>,
    pub(crate) update_version: u64,
    pub(crate) resolver: Option<Resolver>,
    pub(crate) module_table: ModuleTable,
    pub(crate) publics: WorkspaceSymbols,
    pub(crate) modules: HashMap<String, FileId>,
    pub(crate) diagnostic_files: HashSet<FileId>,
    pub(crate) files: FileInterner,
}

pub(crate) struct Backend {
    pub(crate) client: Client,
    pub(crate) state: Arc<RwLock<State>>,
}

impl Backend {
    pub(crate) async fn file_path(&self, file_id: FileId) -> Option<PathBuf> {
        self.state
            .read()
            .await
            .files
            .path(file_id)
            .map(Path::to_path_buf)
    }

    pub(crate) async fn analysis(&self, uri: &Url) -> Option<Arc<Analysis>> {
        let state = self.state.read().await;
        let id = state.files.get(&uri.to_file_path().ok()?)?;
        state.cache.get(&id).cloned()
    }

    pub(crate) async fn analyses(&self) -> Vec<Arc<Analysis>> {
        self.state.read().await.cache.values().cloned().collect()
    }

    pub(crate) async fn definition_source(&self, definition: &Definition) -> Option<String> {
        let module = definition.name.split_once("::")?.0;
        let state = self.state.read().await;
        let file_id = state.modules.get(module)?;
        state
            .cache
            .get(file_id)
            .map(|analysis| analysis.source.clone())
    }

    pub(crate) async fn workspace_locations(&self, target: &Definition) -> Vec<Location> {
        let state = self.state.read().await;
        let files = &state.files;
        state
            .cache
            .values()
            .cloned()
            .flat_map(|analysis| {
                let Some(path) = files.path(analysis.file_id) else {
                    return Vec::new();
                };
                let Some(uri) = Url::from_file_path(path).ok() else {
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
