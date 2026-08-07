use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use eyre::{Result, eyre};
use line_index::LineIndex;
use vinyl_parser::ast::item::{ImportDef, Item};
use vinyl_resolver::ResolveDiagnostic;
use vinyl_resolver::resolver::{ImportPrefix, ModuleInfo, Resolver, ResolverMode};
use vinyl_resolver::structs::DiskFileSystem;
use vinyl_typecheck::module::{ModuleExports, ModuleTable};

use crate::backend::state::{Analysis, PublicSymbol, SourceDiagnostic, WorkspaceState};
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

pub(crate) fn relative_import_path(
    from_file: &Path,
    to_module: &Path,
    resolver: &Resolver,
) -> String {
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
        let relative = relative
            .to_string_lossy()
            .replace('\\', "/")
            .replace('/', "::");
        format!("parent::{relative}")
    } else {
        to_stem
    }
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
    let mut visited = HashSet::new();
    let mut analyses = HashMap::new();
    let mut diagnostics = HashMap::new();
    let mut publics = HashMap::new();

    match parse_file_with_diagnostics(vfs, entry_path) {
        Ok((entry_source, entry_items)) => {
            collect_publics(&entry_items, entry_path, &mut publics);
            let mut all_items = entry_items.clone();
            let mut bare_imported_symbols = HashSet::new();
            collect_modules(
                vfs,
                &mut resolver,
                root,
                entry_path,
                &entry_items,
                &mut all_items,
                &mut entry_module_table,
                &mut visited,
                &mut bare_imported_symbols,
                &mut diagnostics,
            );
            add_resolved_modules(
                vfs,
                &resolver,
                entry_path,
                &mut all_items,
                &mut entry_module_table,
            );
            match vinyl_typecheck::validate_main_return_type(
                &entry_items,
                &entry_source,
                &entry_path.to_string_lossy(),
            ) {
                Ok(()) => match analyze_with_diagnostics(
                    entry_path,
                    &entry_source,
                    &all_items,
                    &entry_module_table,
                ) {
                    Ok(analysis) => {
                        analyses.insert(entry_path.to_path_buf(), analysis);
                    }
                    Err(error) => {
                        diagnostics.insert(entry_path.to_path_buf(), error);
                    }
                },
                Err(error) => {
                    diagnostics.insert(
                        entry_path.to_path_buf(),
                        vec![SourceDiagnostic {
                            message: error.to_string(),
                            offset: error.span.offset(),
                            length: error.span.len(),
                        }],
                    );
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
    let mut publics = HashMap::new();
    for file in &unique {
        if file == entry_path {
            continue;
        }
        let (source, items) = match parse_file_with_diagnostics(vfs, file) {
            Ok(result) => result,
            Err(file_diagnostics) => {
                diagnostics.insert(non_canonical_key(file, &resolver, root), file_diagnostics);
                continue;
            }
        };
        let mut all_items = items.clone();
        let mut module_table = ModuleTable::new();
        let mut file_visited = HashSet::new();
        let mut file_bare_imported_symbols = HashSet::new();
        collect_modules(
            vfs,
            &mut resolver,
            root,
            file,
            &items,
            &mut all_items,
            &mut module_table,
            &mut file_visited,
            &mut file_bare_imported_symbols,
            &mut diagnostics,
        );
        add_resolved_modules(vfs, &resolver, file, &mut all_items, &mut module_table);
        collect_publics(&items, file, &mut publics);
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
    for file_diags in diagnostics.values_mut() {
        file_diags.dedup_by(|a, b| {
            a.offset == b.offset && a.length == b.length && a.message == b.message
        });
    }
    let modules = resolver
        .all_modules()
        .values()
        .map(|info| (info.import_name.clone(), info.file_path.clone()))
        .collect();
    Ok((
        analyses,
        diagnostics,
        resolver,
        entry_module_table,
        publics,
        modules,
    ))
}

fn collect_publics(items: &[Item], path: &Path, publics: &mut HashMap<String, PublicSymbol>) {
    for item in items {
        let (name, span) = match item {
            Item::Function(f) if f.public => (&f.name, f.span),
            Item::Struct(s) if s.public => (&s.name, s.span),
            Item::TupleStruct(t) if t.public => (&t.name, t.span),
            Item::Enum(e) if e.public => (&e.name, e.span),
            _ => continue,
        };
        publics.insert(
            name.clone(),
            PublicSymbol {
                path: path.to_path_buf(),
                span,
            },
        );
    }
}

pub(crate) fn add_resolved_modules(
    vfs: &Vfs,
    resolver: &Resolver,
    from: &Path,
    all_items: &mut Vec<Item>,
    module_table: &mut ModuleTable,
) {
    for info in resolver.all_modules().values() {
        if module_table.contains_key(&info.import_name) {
            continue;
        }
        let Ok((_, items)) = parse_file_with_diagnostics(vfs, &info.file_path) else {
            continue;
        };
        all_items.extend(items.iter().filter_map(|item| match item {
            Item::Struct(structure) if structure.public => Some(item.clone()),
            Item::TupleStruct(tuple) if tuple.public => Some(item.clone()),
            Item::Enum(enumeration) if enumeration.public => Some(item.clone()),
            Item::TypeAlias(alias) if alias.public => Some(item.clone()),
            _ => None,
        }));
        let functions = items
            .iter()
            .filter_map(|item| match item {
                Item::Function(function) if function.public => Some(function.clone()),
                _ => None,
            })
            .collect();
        let types = items
            .iter()
            .filter_map(|item| match item {
                Item::Struct(structure) if structure.public => Some(structure.name.clone()),
                Item::TupleStruct(tuple) if tuple.public => Some(tuple.name.clone()),
                Item::Enum(enumeration) if enumeration.public => Some(enumeration.name.clone()),
                _ => None,
            })
            .collect();
        let exports = ModuleExports {
            import_name: info.import_name.clone(),
            import_path: relative_import_path(from, &info.file_path, resolver),
            imported: false,
            functions,
            types,
        };
        module_table.insert(info.import_name.clone(), exports.clone());
        module_table.insert(info.path.join("::"), exports.clone());
        let inline_exports = ModuleExports {
            imported: true,
            ..exports.clone()
        };
        module_table.insert(exports.import_path.clone(), inline_exports);
    }
}

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
                    symbols: Vec::new(),
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
    let Ok(import_prefix) = import_prefix(import) else {
        return;
    };
    let resolved = if import.wildcard || import.path.len() <= 1 {
        resolve_module_with_prefix(resolver, &import_prefix, &import.path, opened_path)
    } else {
        resolve_module_with_prefix(resolver, &import_prefix, &import.path, opened_path).or_else(
            |_| {
                resolve_module_with_prefix(
                    resolver,
                    &import_prefix,
                    &import.path[..import.path.len() - 1],
                    opened_path,
                )
            },
        )
    };
    if let Ok(info) = resolved
        && !vfs.files().contains_key(&info.file_path)
        && let Ok(source) = std::fs::read_to_string(&info.file_path)
    {
        vfs.set(info.file_path, source);
    }
}

fn import_prefix(import: &ImportDef) -> std::result::Result<Option<ImportPrefix>, ()> {
    if import.prefix.is_empty() {
        return Ok(None);
    }
    let package_count = import
        .prefix
        .iter()
        .filter(|segment| segment.as_str() == "package")
        .count();
    let parent_count = import
        .prefix
        .iter()
        .filter(|segment| segment.as_str() == "parent")
        .count();
    if import.prefix.len() != package_count + parent_count {
        return Err(());
    }
    let prefix = if package_count > 0 {
        ImportPrefix::Package
    } else if parent_count == 1 {
        ImportPrefix::Self_
    } else {
        ImportPrefix::Parent(parent_count - 1)
    };
    Ok(Some(prefix))
}

fn resolve_module_with_prefix(
    resolver: &mut Resolver,
    prefix: &Option<ImportPrefix>,
    path: &[String],
    from: &Path,
) -> Result<ModuleInfo, ResolveDiagnostic> {
    match prefix {
        None => resolver.resolve_module_path(path),
        Some(prefix) => {
            let path_strs: Vec<&str> = path.iter().map(|segment| segment.as_str()).collect();
            resolver.resolve(prefix, &path_strs, from)
        }
    }
}

fn item_name(item: &Item) -> &str {
    match item {
        Item::Function(function) => &function.name,
        Item::Struct(structure) => &structure.name,
        Item::TupleStruct(tuple) => &tuple.name,
        Item::Enum(enumeration) => &enumeration.name,
        Item::TypeAlias(alias) => &alias.name,
        Item::Import(_) => unreachable!("imports cannot be injected into all_items"),
    }
}

fn push_import_diagnostic(
    diagnostics: &mut HashMap<PathBuf, Vec<SourceDiagnostic>>,
    resolver: &Resolver,
    workspace_root: &Path,
    from: &Path,
    import: &ImportDef,
    message: String,
) {
    diagnostics
        .entry(non_canonical_key(from, resolver, workspace_root))
        .or_default()
        .push(SourceDiagnostic {
            message,
            offset: import.span.offset(),
            length: import.span.len(),
        });
}

/// Recursively threads per-module collection state; args are split across
/// shared (`diagnostics`) and per-call (`all_items`, `module_table`, `visited`)
/// lifetimes, so grouping them into a struct would not reduce the surface.
#[allow(clippy::too_many_arguments)]
pub(crate) fn collect_modules(
    vfs: &Vfs,
    resolver: &mut Resolver,
    workspace_root: &Path,
    from: &Path,
    items: &[Item],
    all_items: &mut Vec<Item>,
    module_table: &mut ModuleTable,
    visited: &mut HashSet<PathBuf>,
    bare_imported_symbols: &mut HashSet<String>,
    diagnostics: &mut HashMap<PathBuf, Vec<SourceDiagnostic>>,
) {
    let imports: Vec<ImportDef> = items
        .iter()
        .filter_map(|item| match item {
            Item::Import(def) => Some(def),
            _ => None,
        })
        .flat_map(|import| {
            if import.symbols.is_empty() {
                return vec![import.clone()];
            }
            import
                .symbols
                .iter()
                .map(|symbol| {
                    let mut path = import.path.clone();
                    path.push(symbol.clone());
                    ImportDef {
                        span: import.span,
                        prefix: import.prefix.clone(),
                        path,
                        symbols: Vec::new(),
                        wildcard: false,
                    }
                })
                .collect()
        })
        .collect();
    for item in &imports {
        let Ok(import_prefix) = import_prefix(item) else {
            push_import_diagnostic(
                diagnostics,
                resolver,
                workspace_root,
                from,
                item,
                "`self::` prefix refers to the current file, not an external module; \
                 use `parent::` for relative imports"
                    .to_string(),
            );
            continue;
        };
        let (info, symbol) = if item.wildcard || item.path.len() <= 1 {
            match resolve_module_with_prefix(resolver, &import_prefix, &item.path, from) {
                Ok(info) => (info, None),
                Err(err) => {
                    push_import_diagnostic(
                        diagnostics,
                        resolver,
                        workspace_root,
                        from,
                        item,
                        format!("{err}"),
                    );
                    continue;
                }
            }
        } else {
            match resolve_module_with_prefix(resolver, &import_prefix, &item.path, from) {
                Ok(info) => (info, None),
                Err(first_error) => {
                    let parent_path = item.path[..item.path.len() - 1].to_vec();
                    match resolve_module_with_prefix(resolver, &import_prefix, &parent_path, from) {
                        Ok(info) => (info, Some(item.path.last().unwrap().clone())),
                        Err(_) => {
                            push_import_diagnostic(
                                diagnostics,
                                resolver,
                                workspace_root,
                                from,
                                item,
                                format!("{first_error}"),
                            );
                            continue;
                        }
                    }
                }
            }
        };
        let path = info
            .file_path
            .canonicalize()
            .unwrap_or(info.file_path.clone());
        let already_visited = !visited.insert(path.clone());
        if already_visited && symbol.is_none() {
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
        let mut type_items = Vec::new();
        for module_item in &module_items {
            match module_item {
                Item::Function(function) if function.public => {
                    functions.push(function.clone());
                }
                Item::Struct(structure) if structure.public => {
                    types.push(structure.name.clone());
                    type_items.push(module_item.clone());
                }
                Item::TupleStruct(tuple) if tuple.public => {
                    types.push(tuple.name.clone());
                    type_items.push(module_item.clone());
                }
                Item::Enum(enumeration) if enumeration.public => {
                    types.push(enumeration.name.clone());
                    type_items.push(module_item.clone());
                }
                _ => {}
            }
        }
        if let Some(symbol_name) = symbol {
            all_items.extend(type_items.iter().cloned());
            let found = functions
                .iter()
                .find(|function| function.name == symbol_name)
                .cloned()
                .map(Item::Function)
                .or_else(|| {
                    type_items
                        .iter()
                        .find(|type_item| item_name(type_item) == symbol_name)
                        .cloned()
                });
            let Some(injected) = found else {
                push_import_diagnostic(
                    diagnostics,
                    resolver,
                    workspace_root,
                    from,
                    item,
                    format!(
                        "no public symbol `{symbol_name}` in module `{}`",
                        info.import_name
                    ),
                );
                continue;
            };
            if !bare_imported_symbols.insert(symbol_name.clone()) {
                push_import_diagnostic(
                    diagnostics,
                    resolver,
                    workspace_root,
                    from,
                    item,
                    format!("import of `{symbol_name}` conflicts with an existing import"),
                );
                continue;
            }
            all_items.push(injected);
        } else if item.wildcard {
            for function in &functions {
                if !bare_imported_symbols.insert(function.name.clone()) {
                    push_import_diagnostic(
                        diagnostics,
                        resolver,
                        workspace_root,
                        from,
                        item,
                        format!(
                            "import of `{}` conflicts with an existing import",
                            function.name
                        ),
                    );
                    continue;
                }
                all_items.push(Item::Function(function.clone()));
            }
            for type_item in &type_items {
                if !bare_imported_symbols.insert(item_name(type_item).to_string()) {
                    push_import_diagnostic(
                        diagnostics,
                        resolver,
                        workspace_root,
                        from,
                        item,
                        format!(
                            "import of `{}` conflicts with an existing import",
                            item_name(type_item)
                        ),
                    );
                    continue;
                }
                all_items.push(type_item.clone());
            }
        } else {
            for function in &functions {
                let mut imported = function.clone();
                imported.name = format!("{}::{}", info.import_name, imported.name);
                all_items.push(Item::Function(imported));
            }
            for type_item in &type_items {
                all_items.push(type_item.clone());
            }
            for module_item in &module_items {
                if let Item::Enum(enumeration) = module_item {
                    if enumeration.public {
                        continue;
                    }
                    let mut enumeration = enumeration.clone();
                    enumeration.name = format!("{}::{}", info.import_name, enumeration.name);
                    all_items.push(Item::Enum(enumeration));
                }
            }
        }
        if already_visited {
            continue;
        }
        let exports = ModuleExports {
            import_name: info.import_name.clone(),
            import_path: relative_import_path(from, &info.file_path, resolver),
            imported: true,
            functions,
            types,
        };
        module_table.insert(info.import_name.clone(), exports.clone());
        module_table.insert(info.path.join("::"), exports.clone());
        module_table.insert(exports.import_path.clone(), exports);
        collect_modules(
            vfs,
            resolver,
            workspace_root,
            &info.file_path,
            &module_items,
            all_items,
            module_table,
            visited,
            bare_imported_symbols,
            diagnostics,
        );
    }
}
