use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use eyre::{Result, eyre};
use line_index::LineIndex;
use vinyl_parser::ast::item::Item;
use vinyl_resolver::{ImportPrefix, Resolver, ResolverMode};
use vinyl_typecheck::module::{ModuleExports, ModuleTable};

use crate::backend::state::{Analysis, SourceDiagnostic, WorkspaceState};
use crate::vfs::{LspFileSystem, Vfs};

pub(crate) fn analyze_with_diagnostics(
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

pub(crate) fn parse_file_with_diagnostics(
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

pub(crate) fn same_file(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}

pub(crate) fn non_canonical_key(path: &Path, resolver: &Resolver, workspace_root: &Path) -> PathBuf {
    let plain = PathBuf::from(path.to_string_lossy().trim_start_matches(r"\\?\").to_string());
    let source_root = match resolver.mode() {
        ResolverMode::Manifest => resolver.root().join("src"),
        ResolverMode::Script => resolver.root().to_path_buf(),
    };
    plain
        .strip_prefix(&source_root)
        .map(|relative| workspace_root.join(relative))
        .unwrap_or(plain)
}

pub(crate) fn relative_import_path(from_file: &Path, to_module: &Path, resolver: &Resolver) -> String {
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

pub(crate) fn analyze_workspace(vfs: &Vfs, root: &Path, entry_path: &Path) -> Result<WorkspaceState> {
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

    let mut entry_module_table = ModuleTable::new();
    let mut visited = HashSet::new();
    let mut analyses = HashMap::new();
    let mut diagnostics = HashMap::new();

    match parse_file_with_diagnostics(vfs, entry_path) {
        Ok((entry_source, entry_items)) => {
            let mut all_items = entry_items.clone();
            collect_modules(
                vfs,
                &mut resolver,
                root,
                entry_path,
                &entry_items,
                &mut all_items,
                &mut entry_module_table,
                &mut visited,
                &mut diagnostics,
            );
            match analyze_with_diagnostics(entry_path, &entry_source, &all_items, &entry_module_table) {
                Ok(analysis) => {
                    analyses.insert(entry_path.to_path_buf(), analysis);
                }
                Err(error) => {
                    diagnostics.insert(entry_path.to_path_buf(), error);
                }
            }
        }
        Err(entry_diagnostics) => {
            diagnostics.insert(entry_path.to_path_buf(), entry_diagnostics);
        }
    }
    let mut files: Vec<PathBuf> = visited.iter().cloned().collect();
    for info in resolver.all_modules().values() {
        files.push(info.file_path.clone());
    }
    files.sort();
    files.dedup();
    let mut seen = HashSet::new();
    let mut unique: Vec<PathBuf> = Vec::new();
    for file in files {
        if seen.insert(non_canonical_key(&file, &resolver, root)) {
            unique.push(file);
        }
    }
    for file in &unique {
        if file == entry_path {
            continue;
        }
        let (source, items) = match parse_file_with_diagnostics(vfs, file) {
            Ok(result) => result,
            Err(file_diagnostics) => {
                diagnostics
                    .insert(non_canonical_key(file, &resolver, root), file_diagnostics);
                continue;
            }
        };
        let mut all_items = items.clone();
        let mut module_table = ModuleTable::new();
        let mut file_visited = HashSet::new();
        collect_modules(
            vfs,
            &mut resolver,
            root,
            file,
            &items,
            &mut all_items,
            &mut module_table,
            &mut file_visited,
            &mut diagnostics,
        );
        match analyze_with_diagnostics(file, &source, &all_items, &module_table) {
            Ok(analysis) => {
                let key = non_canonical_key(file, &resolver, root);
                analyses.insert(key, analysis);
            }
            Err(error) => {
                diagnostics.insert(non_canonical_key(file, &resolver, root), error);
            }
        }
    }
    for file_diags in diagnostics.values_mut() {
        file_diags.dedup_by(|a, b| {
            a.offset == b.offset && a.length == b.length && a.message == b.message
        });
    }
    Ok((analyses, diagnostics, resolver, entry_module_table))
}

/// Recursively threads per-module collection state; args are split across
/// shared (`diagnostics`) and per-call (`all_items`, `module_table`, `visited`)
/// lifetimes, so grouping them into a struct would not reduce the surface.
#[allow(clippy::too_many_arguments)]
fn collect_modules(
    vfs: &Vfs,
    resolver: &mut Resolver,
    workspace_root: &Path,
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
                    diagnostics
                        .entry(non_canonical_key(from, resolver, workspace_root))
                        .or_default()
                        .push(
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
                diagnostics
                    .entry(non_canonical_key(from, resolver, workspace_root))
                    .or_default()
                    .push(
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
                    diagnostics
                        .entry(non_canonical_key(from, resolver, workspace_root))
                        .or_default()
                        .push(
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
                diagnostics
                    .entry(non_canonical_key(&path, resolver, workspace_root))
                    .or_default()
                    .extend(file_diagnostics);
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
            workspace_root,
            &info.file_path,
            &module_items,
            all_items,
            module_table,
            visited,
            diagnostics,
        );
    }
}
