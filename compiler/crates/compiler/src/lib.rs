pub mod error;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use vinyl_parser::ast::item::{ImportDef, Item};
use vinyl_resolver::resolver::{ImportPrefix, Resolver};
use vinyl_typecheck::TypeDiagnostic;
use vinyl_typecheck::module::{ModuleExports, ModuleTable};

use crate::error::{CompileError, ModuleError};

#[derive(Debug)]
pub struct CompiledModule {
    pub items: Vec<vinyl_typecheck::hir::HirItem>,
    pub module_table: ModuleTable,
}

fn parse_file(path: &Path) -> Result<(String, String, Vec<Item>), Vec<CompileError>> {
    let source = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => return Err(vec![CompileError::Io(e)]),
    };
    let name = path.to_string_lossy().to_string();
    let items = match vinyl_parser::parse_and_lower_with_name(&name, &source) {
        Ok(items) => items,
        Err(errors) => {
            return Err(errors
                .into_iter()
                .map(CompileError::Parse)
                .collect::<Vec<CompileError>>());
        }
    };
    Ok((source, name, items))
}

struct CollectedImport {
    prefix: Vec<String>,
    path: Vec<String>,
}

fn collect_imports(items: &[Item]) -> Vec<CollectedImport> {
    items
        .iter()
        .filter_map(|item| {
            if let Item::Import(ImportDef { prefix, path, .. }) = item {
                Some(CollectedImport {
                    prefix: prefix.clone(),
                    path: path.clone(),
                })
            } else {
                None
            }
        })
        .collect()
}

fn find_entry_file(source_root: &Path) -> Option<PathBuf> {
    let candidates = ["main.vn", "lib.vn"];
    for name in &candidates {
        let path = source_root.join(name);
        if path.exists() {
            return Some(path);
        }
    }
    None
}

fn resolve_imports(
    items: &[Item],
    from: &Path,
    resolver: &mut Resolver,
    all_items: &mut Vec<Item>,
    visited: &mut HashSet<PathBuf>,
) -> Result<ModuleTable, Vec<CompileError>> {
    let mut module_table: ModuleTable = HashMap::new();

    for import in collect_imports(items) {
        let import_display = {
            let mut s = String::new();
            if !import.prefix.is_empty() {
                s.push_str(&import.prefix.join("::"));
                s.push_str("::");
            }
            s.push_str(&import.path.join("::"));
            s
        };

        let module_info = if import.prefix.is_empty() {
            resolver.resolve_module_path(&import.path).map_err(|e| {
                vec![CompileError::Module(ModuleError {
                    message: format!("could not resolve import `{import_display}`: {e}"),
                })]
            })?
        } else {
            let self_count = import
                .prefix
                .iter()
                .filter(|s| s.as_str() == "self")
                .count();
            let package_count = import
                .prefix
                .iter()
                .filter(|s| s.as_str() == "package")
                .count();
            let parent_count = import
                .prefix
                .iter()
                .filter(|s| s.as_str() == "parent")
                .count();
            let total_known = self_count + package_count + parent_count;
            let total = import.prefix.len();

            if total_known != total {
                return Err(vec![CompileError::Module(ModuleError {
                    message: format!("unknown import prefix in `{import_display}`"),
                })]);
            }

            if self_count > 0 {
                return Err(vec![CompileError::Module(ModuleError {
                    message: "`self::` prefix refers to the current file, not an external module; \
                         use `parent::` for relative imports"
                        .to_string(),
                })]);
            }
            if package_count > 1 {
                return Err(vec![CompileError::Module(ModuleError {
                    message: format!("`package` prefix can only appear once in `{import_display}`"),
                })]);
            }
            if package_count > 0 && parent_count > 0 {
                return Err(vec![CompileError::Module(ModuleError {
                    message: format!(
                        "cannot combine `package` and `parent` prefixes in `{import_display}`"
                    ),
                })]);
            }

            if parent_count >= 4 {
                eprintln!(
                    "warning: import `{import_display}` uses {parent_count} levels of `parent::`; \
                     consider moving to a manifest-based project"
                );
            }

            let prefix = if package_count > 0 {
                ImportPrefix::Package
            } else if parent_count == 1 {
                ImportPrefix::Self_
            } else {
                ImportPrefix::Parent(parent_count - 1)
            };

            let path_strs: Vec<&str> = import.path.iter().map(|s| s.as_str()).collect();
            resolver.resolve(&prefix, &path_strs, from).map_err(|e| {
                vec![CompileError::Module(ModuleError {
                    message: format!("could not resolve import `{import_display}`: {e}"),
                })]
            })?
        };

        let canonical = module_info.file_path.canonicalize().map_err(|e| {
            vec![CompileError::Module(ModuleError {
                message: format!(
                    "could not canonicalize path `{}`: {e}",
                    module_info.file_path.display()
                ),
            })]
        })?;

        if !visited.insert(canonical) {
            continue;
        }

        let (_, _, module_items) = parse_file(&module_info.file_path)?;

        let import_name = module_info.import_name.clone();

        let mut public_functions = Vec::new();
        let mut public_types = Vec::new();

        for item in &module_items {
            let is_public = match item {
                Item::Function(f) => {
                    if f.public {
                        public_functions.push(f.clone());
                    }
                    f.public
                }
                Item::Struct(s) => {
                    if s.public {
                        public_types.push(s.name.clone());
                    }
                    s.public
                }
                Item::TupleStruct(t) => {
                    if t.public {
                        public_types.push(t.name.clone());
                    }
                    t.public
                }
                Item::Enum(e) => {
                    if e.public {
                        public_types.push(e.name.clone());
                    }
                    e.public
                }
                Item::Import(_) => true,
            };

            if is_public {
                let item = match item {
                    Item::Function(function) => {
                        let mut function = function.clone();
                        function.name = format!("{}::{}", import_name, function.name);
                        Item::Function(function)
                    }
                    _ => item.clone(),
                };
                all_items.push(item);
            }
        }

        module_table.insert(
            import_name.clone(),
            ModuleExports {
                import_name,
                import_path: import_display,
                imported: true,
                functions: public_functions,
                types: public_types,
            },
        );

        let sub_table = resolve_imports(
            &module_items,
            &module_info.file_path,
            resolver,
            all_items,
            visited,
        )?;
        module_table.extend(sub_table);
    }

    Ok(module_table)
}

fn add_resolved_modules(
    module_table: &mut ModuleTable,
    resolver: &Resolver,
    from: &Path,
) -> Result<(), Vec<CompileError>> {
    for info in resolver.all_modules().values() {
        if module_table.contains_key(&info.import_name) {
            continue;
        }
        let (_, _, items) = parse_file(&info.file_path)?;
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
        module_table.insert(
            info.import_name.clone(),
            ModuleExports {
                import_name: info.import_name.clone(),
                import_path: relative_import_path(from, &info.file_path, resolver),
                imported: false,
                functions,
                types,
            },
        );
    }
    Ok(())
}

fn relative_import_path(from: &Path, to: &Path, resolver: &Resolver) -> String {
    let stem = to.file_stem().unwrap_or_default().to_string_lossy();
    let source_root = match resolver.mode() {
        vinyl_resolver::resolver::ResolverMode::Manifest => resolver.root().join("src"),
        vinyl_resolver::resolver::ResolverMode::Script => resolver.root().to_path_buf(),
    };
    if let Ok(relative) = to.strip_prefix(&source_root) {
        let relative = relative
            .with_extension("")
            .to_string_lossy()
            .replace(['/', '\\'], "::");
        if from
            .parent()
            .and_then(|parent| to.parent().map(|target| parent == target))
            == Some(true)
        {
            return format!("parent::{relative}");
        }
        return format!("package::{relative}");
    }
    stem.into_owned()
}

pub fn compile_entry(
    file_path: &Path,
    project_root: Option<&Path>,
) -> Result<(CompiledModule, Vec<TypeDiagnostic>), Vec<CompileError>> {
    let mut resolver = if let Some(root) = project_root {
        vinyl_resolver::resolver::Resolver::for_manifest(root)
            .map_err(|e| vec![CompileError::ModResolve(e)])?
    } else {
        vinyl_resolver::resolver::Resolver::detect(file_path)
            .map_err(|e| vec![CompileError::ModResolve(e)])?
    };
    if matches!(
        resolver.mode(),
        vinyl_resolver::resolver::ResolverMode::Script
    ) {
        let root = resolver.root().to_path_buf();
        register_script_modules(&mut resolver, root);
    }

    let (entry_source, entry_source_name, mut all_items) = if file_path.is_dir() {
        let entry = find_entry_file(file_path).ok_or_else(|| {
            vec![CompileError::Module(ModuleError {
                message: format!(
                    "no entry file found in `{}` (looked for main.vn, lib.vn)",
                    file_path.display()
                ),
            })]
        })?;
        parse_file(&entry)?
    } else {
        parse_file(file_path)?
    };

    let mut visited = HashSet::new();
    if let Ok(canonical) = file_path.canonicalize() {
        visited.insert(canonical);
    }

    let entry_items = all_items.clone();
    let module_table = resolve_imports(
        &entry_items,
        file_path,
        &mut resolver,
        &mut all_items,
        &mut visited,
    )?;
    let mut module_table = module_table;
    add_resolved_modules(&mut module_table, &resolver, file_path)?;

    let (hir, warnings) = vinyl_typecheck::typeck_with_modules(
        &all_items,
        &entry_source,
        &entry_source_name,
        &module_table,
    )
    .map_err(|errors| {
        errors
            .into_iter()
            .map(CompileError::TypeDiagnostic)
            .collect::<Vec<_>>()
    })?;

    Ok((
        CompiledModule {
            items: hir,
            module_table,
        },
        warnings,
    ))
}

fn register_script_modules(resolver: &mut Resolver, directory: PathBuf) {
    let Ok(entries) = std::fs::read_dir(&directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            register_script_modules(resolver, path);
        } else if path.extension().is_some_and(|extension| extension == "vn") {
            resolver.register_module(&path);
        }
    }
}
