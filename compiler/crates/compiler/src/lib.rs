use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use miette::Diagnostic;
use thiserror::Error;
use vinyl_parser::ast::item::{ImportDef, Item};
use vinyl_resolver::{ImportPrefix, ResolveDiagnostic};
use vinyl_typecheck::TypeDiagnostic;
use vinyl_typecheck::module::{ModuleExports, ModuleTable};

#[derive(Debug, Error, Diagnostic)]
pub enum CompileError {
    #[error(transparent)]
    #[diagnostic(transparent)]
    Parse(#[from] vinyl_parser::ParserDiagnostic),
    #[error(transparent)]
    #[diagnostic(transparent)]
    TypeDiagnostic(#[from] TypeDiagnostic),
    #[error("io error: {0}")]
    #[diagnostic(code(compiler::io_error))]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    #[diagnostic(code(compiler::module_error))]
    Module(#[from] ModuleError),
    #[error("module resolution error: {0}")]
    #[diagnostic(code(compiler::module_resolution_error))]
    ModResolve(#[from] ResolveDiagnostic),
}

#[derive(Debug, Error, Diagnostic)]
#[error("{message}")]
#[diagnostic(
    help("check the module path and file structure"),
    code(compiler::module_error)
)]
pub struct ModuleError {
    pub message: String,
}

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
    resolver: &mut vinyl_resolver::Resolver,
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
                         use `parent::` for relative imports".to_string(),
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

pub fn compile_entry(
    file_path: &Path,
    project_root: Option<&Path>,
) -> Result<(CompiledModule, Vec<TypeDiagnostic>), Vec<CompileError>> {
    let mut resolver = if let Some(root) = project_root {
        vinyl_resolver::Resolver::for_manifest(root)
            .map_err(|e| vec![CompileError::ModResolve(e)])?
    } else {
        vinyl_resolver::Resolver::detect(file_path)
            .map_err(|e| vec![CompileError::ModResolve(e)])?
    };

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn script_project(name: &str, files: &[(&str, &str)]) -> PathBuf {
        let root = std::env::temp_dir().join(format!("vinyl_compiler_script_{name}"));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        for (file, source) in files {
            let path = root.join(file);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, source).unwrap();
        }
        root
    }

    fn project(name: &str, files: &[(&str, &str)]) -> PathBuf {
        let root = std::env::temp_dir().join(format!("vinyl_compiler_test_{name}"));
        let _ = fs::remove_dir_all(&root);
        for (file, source) in files {
            let path = root.join(file);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, source).unwrap();
        }
        fs::write(root.join("vinyl.toml"), "").unwrap();
        root
    }

    #[test]
    fn compiles_public_import() {
        let root = project(
            "public_import",
            &[
                (
                    "src/main.vn",
                    "import math; fn main(): int { math::answer() }",
                ),
                ("src/math.vn", "public fn answer(): int { 42 }"),
            ],
        );
        let result = compile_entry(&root.join("src/main.vn"), Some(&root));
        assert!(result.is_ok(), "{result:?}");
    }

    #[test]
    fn rejects_private_import() {
        let root = project(
            "private_import",
            &[
                (
                    "src/main.vn",
                    "import math; fn main(): int { math::answer() }",
                ),
                ("src/math.vn", "fn answer(): int { 42 }"),
            ],
        );
        let result = compile_entry(&root.join("src/main.vn"), Some(&root));
        assert!(result.is_err());
    }

    #[test]
    fn import_not_found_errors() {
        let root = project(
            "import_not_found",
            &[("src/main.vn", "import math; fn main(): int { 0 }")],
        );
        let result = compile_entry(&root.join("src/main.vn"), Some(&root));
        assert!(result.is_err());
    }

    #[test]
    fn nested_module_import() {
        let root = project(
            "nested_import",
            &[
                (
                    "src/main.vn",
                    "import utils::format; fn main(): string { format::greet() }",
                ),
                (
                    "src/utils/format.vn",
                    "public fn greet(): string { \"hi\" }",
                ),
            ],
        );
        let result = compile_entry(&root.join("src/main.vn"), Some(&root));
        assert!(result.is_ok(), "{result:?}");
    }

    #[test]
    fn entry_without_main_or_lib() {
        let root = project("no_entry", &[("src/foo.vn", "fn foo(): int { 1 }")]);
        let result = compile_entry(&root, Some(&root));
        assert!(result.is_err());
    }

    #[test]
    fn compiles_script_project_with_import() {
        let root = script_project(
            "script_import",
            &[
                (
                    "main.vn",
                    "import math; fn main(): int { math::double(21) }",
                ),
                ("math.vn", "public fn double(n: int): int { n * 2 }"),
            ],
        );
        let result = compile_entry(&root.join("main.vn"), None);
        assert!(result.is_ok(), "{result:?}");
    }

    #[test]
    fn compiles_manifest_via_detect() {
        let root = project(
            "manifest_detect",
            &[
                (
                    "src/main.vn",
                    "import math; fn main(): int { math::answer() }",
                ),
                ("src/math.vn", "public fn answer(): int { 42 }"),
            ],
        );
        let result = compile_entry(&root.join("src/main.vn"), None);
        assert!(result.is_ok(), "{result:?}");
    }

    #[test]
    fn resolves_directory_module() {
        let root = project(
            "directory_module",
            &[
                (
                    "src/main.vn",
                    "import math; fn main(): int { math::answer() }",
                ),
                ("src/math/math.vn", "public fn answer(): int { 42 }"),
            ],
        );
        let result = compile_entry(&root.join("src/main.vn"), Some(&root));
        assert!(result.is_ok(), "{result:?}");
    }

    #[test]
    fn script_self_prefix_errors() {
        let root = script_project(
            "script_self_errors",
            &[("main.vn", "import self::helper; fn main(): int { 0 }")],
        );
        let result = compile_entry(&root.join("main.vn"), None);
        assert!(result.is_err(), "self:: should error in imports");
    }

    #[test]
    fn script_parent_prefix_same_dir() {
        let root = script_project(
            "script_parent_same_dir",
            &[
                (
                    "sub/main.vn",
                    "import parent::helper; fn main(): int { helper::answer() }",
                ),
                ("sub/helper.vn", "public fn answer(): int { 42 }"),
            ],
        );
        let result = compile_entry(&root.join("sub/main.vn"), None);
        assert!(result.is_ok(), "{result:?}");
    }

    #[test]
    fn script_parent_parent_prefix() {
        let root = script_project(
            "script_parent_parent",
            &[
                (
                    "sub/main.vn",
                    "import parent::parent::helper; fn main(): int { helper::answer() }",
                ),
                ("helper.vn", "public fn answer(): int { 42 }"),
            ],
        );
        let result = compile_entry(&root.join("sub/main.vn"), None);
        assert!(result.is_ok(), "{result:?}");
    }

    #[test]
    fn script_package_prefix_rejected() {
        let root = script_project(
            "script_package_rejected",
            &[
                ("main.vn", "import package::helper; fn main(): int { 0 }"),
                ("helper.vn", "public fn answer(): int { 42 }"),
            ],
        );
        let result = compile_entry(&root.join("main.vn"), None);
        assert!(
            result.is_err(),
            "package:: should be rejected in script mode"
        );
    }

    #[test]
    fn manifest_self_prefix_errors() {
        let root = project(
            "manifest_self_errors",
            &[("src/main.vn", "import self::helper; fn main(): int { 0 }")],
        );
        let result = compile_entry(&root.join("src/main.vn"), Some(&root));
        assert!(result.is_err(), "self:: should error in imports");
    }

    #[test]
    fn manifest_parent_prefix_same_dir() {
        let root = project(
            "manifest_parent_same_dir",
            &[
                (
                    "src/sub/main.vn",
                    "import parent::helper; fn main(): int { helper::answer() }",
                ),
                ("src/sub/helper.vn", "public fn answer(): int { 42 }"),
            ],
        );
        let result = compile_entry(&root.join("src/sub/main.vn"), Some(&root));
        assert!(result.is_ok(), "{result:?}");
    }

    #[test]
    fn manifest_parent_parent_prefix() {
        let root = project(
            "manifest_parent_parent",
            &[
                (
                    "src/sub/main.vn",
                    "import parent::parent::helper; fn main(): int { helper::answer() }",
                ),
                ("src/helper.vn", "public fn answer(): int { 42 }"),
            ],
        );
        let result = compile_entry(&root.join("src/sub/main.vn"), Some(&root));
        assert!(result.is_ok(), "{result:?}");
    }

    #[test]
    fn manifest_package_prefix() {
        let root = project(
            "manifest_package_prefix",
            &[
                (
                    "src/main.vn",
                    "import package::helper; fn main(): int { helper::answer() }",
                ),
                ("src/helper.vn", "public fn answer(): int { 42 }"),
            ],
        );
        let result = compile_entry(&root.join("src/main.vn"), Some(&root));
        assert!(result.is_ok(), "{result:?}");
    }
}
