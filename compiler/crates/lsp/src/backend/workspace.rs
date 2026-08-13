use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use eyre::{Result, eyre};
use line_index::LineIndex;
use vinyl_parser::ast::item::{ImportDef, Item};
use vinyl_resolver::resolver::{ImportPrefix, Resolver, ResolverMode};
use vinyl_resolver::structs::DiskFileSystem;
use vinyl_typecheck::module::ModuleTable;

use crate::backend::state::{
    Analysis, FileId, FileInterner, PublicSymbol, SourceDiagnostic, WorkspaceState,
};
use crate::vfs::{LspFileSystem, Vfs};

pub(crate) fn is_public_symbol(analysis: &Analysis, name: &str) -> bool {
    analysis.result.items.iter().any(|item| {
        let (item_name, public) = match &item.kind {
            vinyl_typecheck::hir::HirItemKind::Function(function) => {
                (&function.name, function.public)
            }
            vinyl_typecheck::hir::HirItemKind::Struct(structure) => {
                (&structure.name, structure.public)
            }
            vinyl_typecheck::hir::HirItemKind::TupleStruct(tuple) => (&tuple.name, tuple.public),
            vinyl_typecheck::hir::HirItemKind::Enum(enumeration) => {
                (&enumeration.name, enumeration.public)
            }
            vinyl_typecheck::hir::HirItemKind::TypeAlias(alias) => (&alias.name, alias.public),
        };
        item_name == name && public
    })
}

pub(crate) fn is_imported(imports: &HashSet<String>, import_name: &str) -> bool {
    imports
        .iter()
        .any(|import| import == import_name || import.ends_with(&format!("::{import_name}")))
}

pub(crate) fn analyze_with_diagnostics(
    path: &Path,
    file_id: FileId,
    source: &str,
    items: &[Item],
    source_items: &[Item],
    module_table: &ModuleTable,
) -> std::result::Result<Arc<Analysis>, Vec<SourceDiagnostic>> {
    let name = path.to_string_lossy();
    let (result, warnings) = vinyl_typecheck::typeck_with_index(items, source, &name, module_table)
        .map_err(|errors| {
            errors
                .into_iter()
                .map(|error| SourceDiagnostic {
                    message: format!("{error}"),
                    offset: error.span.offset(),
                    length: error.span.len(),
                    warning: false,
                })
                .collect::<Vec<_>>()
        })?;
    let warnings = warnings
        .into_iter()
        .map(|warning| SourceDiagnostic {
            message: warning.to_string(),
            offset: warning.span.offset(),
            length: warning.span.len(),
            warning: true,
        })
        .chain(
            vinyl_typecheck::unused_import_warnings(source_items, source, &name, module_table)
                .into_iter()
                .map(|warning| SourceDiagnostic {
                    message: warning.to_string(),
                    offset: warning.span.offset(),
                    length: warning.span.len(),
                    warning: true,
                }),
        )
        .collect();
    Ok(Arc::new(Analysis {
        file_id,
        source: source.to_string(),
        line_index: LineIndex::new(source),
        result,
        warnings,
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
                warning: false,
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
                    warning: false,
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
                    warning: false,
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

pub(crate) fn non_canonical_key(
    path: &Path,
    resolver: &Resolver,
    workspace_root: &Path,
) -> PathBuf {
    let plain = PathBuf::from(
        path.to_string_lossy()
            .trim_start_matches(r"\\?\")
            .to_string(),
    );
    let source_root = match resolver.mode() {
        ResolverMode::Manifest => resolver.root().join("src"),
        ResolverMode::Script => resolver.root().to_path_buf(),
    };
    plain
        .strip_prefix(&source_root)
        .map(|relative| workspace_root.join(relative))
        .unwrap_or(plain)
}

pub(crate) fn analyze_workspace(
    vfs: &Vfs,
    root: &Path,
    entry_path: &Path,
) -> Result<WorkspaceState> {
    let vfs_map: HashMap<PathBuf, String> = vfs.files().clone();
    let fs = Box::new(LspFileSystem::new(vfs_map));
    let mut resolver = Resolver::detect_with(root, fs).map_err(|e| eyre!("resolver error: {e}"))?;

    if let ResolverMode::Script = resolver.mode()
        && vfs.source(entry_path).is_some()
    {
        let root = resolver.root().to_path_buf();
        let mut files = resolver.list_vn_files(&root).unwrap_or_default();
        files.extend(
            vfs.files()
                .keys()
                .filter(|file_path| file_path.extension().is_some_and(|ext| ext == "vn"))
                .cloned(),
        );
        for file_path in files {
            resolver.register_module(&file_path);
        }
    }

    let mut entry_module_table = ModuleTable::new();
    let mut files_interner = FileInterner::default();
    let mut analyses = HashMap::new();
    let mut diagnostics: HashMap<FileId, Vec<SourceDiagnostic>> = HashMap::new();
    let mut publics = HashMap::new();

    let mut read_source = |path: &Path| {
        vfs.source(path)
            .ok_or_else(|| format!("could not read {}", path.display()))
    };

    let mut graph_files: Vec<(PathBuf, String, Vec<Item>)> = Vec::new();
    match parse_file_with_diagnostics(vfs, entry_path) {
        Ok((entry_source, entry_items)) => {
            collect_publics(
                &entry_items,
                files_interner.intern(entry_path),
                &mut publics,
            );
            let graph = resolver.build_module_graph(entry_path, &entry_items, &mut read_source);
            graph_files = graph.files.clone();
            for issue in &graph.issues {
                if issue.warning {
                    continue;
                }
                diagnostics
                    .entry(files_interner.intern(&issue.file))
                    .or_default()
                    .push(SourceDiagnostic {
                        message: issue.message.clone(),
                        offset: issue.offset,
                        length: issue.length,
                        warning: false,
                    });
            }
            entry_module_table = graph.module_table.clone();
            match vinyl_typecheck::validate_main_return_type(
                &entry_items,
                &entry_source,
                &entry_path.to_string_lossy(),
            ) {
                Ok(()) => match analyze_with_diagnostics(
                    entry_path,
                    files_interner.intern(entry_path),
                    &entry_source,
                    &graph.all_items,
                    &entry_items,
                    &entry_module_table,
                ) {
                    Ok(analysis) => {
                        if !analysis.warnings.is_empty() {
                            diagnostics
                                .entry(files_interner.intern(entry_path))
                                .or_default()
                                .extend(analysis.warnings.clone());
                        }
                        analyses.insert(files_interner.intern(entry_path), analysis);
                    }
                    Err(error) => {
                        diagnostics.insert(files_interner.intern(entry_path), error);
                    }
                },
                Err(error) => {
                    diagnostics.insert(
                        files_interner.intern(entry_path),
                        vec![SourceDiagnostic {
                            message: error.to_string(),
                            offset: error.span.offset(),
                            length: error.span.len(),
                            warning: false,
                        }],
                    );
                }
            }
        }
        Err(entry_diagnostics) => {
            diagnostics.insert(files_interner.intern(entry_path), entry_diagnostics);
        }
    }

    let mut files: Vec<(PathBuf, String, Vec<Item>)> = graph_files;
    for info in resolver.all_modules().values() {
        let already_covered = files.iter().any(|(file, _, _)| {
            non_canonical_key(file, &resolver, root)
                == non_canonical_key(&info.file_path, &resolver, root)
        });
        if already_covered {
            continue;
        }
        let Ok((source, items)) = parse_file_with_diagnostics(vfs, &info.file_path) else {
            continue;
        };
        files.push((info.file_path.clone(), source, items));
    }

    let mut seen = HashSet::new();
    for (file, file_source, file_items) in &files {
        if non_canonical_key(file, &resolver, root)
            == non_canonical_key(entry_path, &resolver, root)
        {
            continue;
        }
        let key = files_interner.intern(file);
        if !seen.insert(key) {
            continue;
        }
        collect_publics(file_items, files_interner.intern(file), &mut publics);
        let sub_graph = resolver.build_module_graph(file, file_items, &mut read_source);
        for issue in &sub_graph.issues {
            if issue.warning {
                continue;
            }
            diagnostics
                .entry(files_interner.intern(&issue.file))
                .or_default()
                .push(SourceDiagnostic {
                    message: issue.message.clone(),
                    offset: issue.offset,
                    length: issue.length,
                    warning: false,
                });
        }
        match analyze_with_diagnostics(
            file,
            files_interner.intern(file),
            file_source,
            &sub_graph.all_items,
            file_items,
            &sub_graph.module_table,
        ) {
            Ok(analysis) => {
                if !analysis.warnings.is_empty() {
                    diagnostics
                        .entry(key)
                        .or_default()
                        .extend(analysis.warnings.clone());
                }
                analyses.insert(key, analysis);
            }
            Err(error) => {
                diagnostics.insert(key, error);
            }
        }
    }

    for file_diags in diagnostics.values_mut() {
        file_diags.dedup_by(|a, b| {
            a.offset == b.offset && a.length == b.length && a.message == b.message
        });
    }
    let modules = resolver
        .all_modules()
        .values()
        .map(|info| {
            (
                info.import_name.clone(),
                files_interner.intern(&info.file_path),
            )
        })
        .collect();
    Ok((
        analyses,
        diagnostics,
        resolver,
        entry_module_table,
        publics,
        modules,
        files_interner,
    ))
}

fn collect_publics(
    items: &[Item],
    file_id: crate::backend::state::FileId,
    publics: &mut HashMap<String, PublicSymbol>,
) {
    for item in items {
        let (name, span) = match item {
            Item::Function(f) if f.public => (&f.name, f.span),
            Item::Struct(s) if s.public => (&s.name, s.span),
            Item::TupleStruct(t) if t.public => (&t.name, t.span),
            Item::Enum(e) if e.public => (&e.name, e.span),
            _ => continue,
        };
        publics.insert(name.clone(), PublicSymbol { file_id, span });
    }
}

/// Loads the sources of every module imported by `opened_path` into the VFS, so
/// the analyzer can resolve them even before the user opens those files.
pub(crate) fn load_imported_modules(vfs: &mut Vfs, root: &Path, opened_path: &Path) {
    let mut resolver = match Resolver::detect_with(root, Box::new(DiskFileSystem)) {
        Ok(resolver) => resolver,
        Err(_) => return,
    };
    if !matches!(resolver.mode(), ResolverMode::Script) {
        return;
    }
    let Ok((_, items)) = parse_file_with_diagnostics(vfs, opened_path) else {
        return;
    };
    for import in items.iter().filter_map(|item| match item {
        Item::Import(def) => Some(def),
        _ => None,
    }) {
        if !import.symbols.is_empty() {
            for symbol in &import.symbols {
                let mut path = import.path.clone();
                path.push(symbol.clone());
                let synthetic = ImportDef {
                    span: import.span,
                    prefix: import.prefix.clone(),
                    path,
                    path_spans: Vec::new(),
                    symbols: Vec::new(),
                    symbol_spans: Vec::new(),
                    wildcard: false,
                };
                load_imported_module(vfs, &mut resolver, opened_path, &synthetic);
            }
            continue;
        }
        load_imported_module(vfs, &mut resolver, opened_path, import);
    }
}

fn load_imported_module(
    vfs: &mut Vfs,
    resolver: &mut Resolver,
    opened_path: &Path,
    import: &ImportDef,
) {
    let Ok(prefix) = vinyl_resolver::module_graph::import_prefix(import) else {
        return;
    };
    let resolved = if import.wildcard || import.path.len() <= 1 {
        resolve_module_with_prefix(resolver, &prefix, &import.path, opened_path)
    } else {
        resolve_module_with_prefix(resolver, &prefix, &import.path, opened_path).or_else(|_| {
            resolve_module_with_prefix(
                resolver,
                &prefix,
                &import.path[..import.path.len() - 1],
                opened_path,
            )
        })
    };
    if let Ok(info) = resolved
        && !vfs.files().contains_key(&info.file_path)
        && let Ok(source) = std::fs::read_to_string(&info.file_path)
    {
        vfs.set(info.file_path, source);
    }
}

fn resolve_module_with_prefix(
    resolver: &mut Resolver,
    prefix: &Option<ImportPrefix>,
    path: &[String],
    from: &Path,
) -> Result<vinyl_resolver::resolver::ModuleInfo, vinyl_resolver::ResolveDiagnostic> {
    match prefix {
        None => resolver.resolve_module_path(path),
        Some(prefix) => {
            let path_strs: Vec<&str> = path.iter().map(|segment| segment.as_str()).collect();
            resolver.resolve(prefix, &path_strs, from)
        }
    }
}
