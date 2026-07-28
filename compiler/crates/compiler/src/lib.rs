use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use miette::Diagnostic;
use thiserror::Error;
use vinyl_parser::ast::item::{ImportDef, Item};
use vinyl_resolver::ResolveDiagnostic;
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

fn find_source_root(file_path: &Path) -> PathBuf {
    if let Some(parent) = file_path.parent() {
        let src_candidate = parent.join("src");
        if src_candidate.is_dir() {
            return src_candidate;
        }
        parent.to_path_buf()
    } else {
        let src_candidate = PathBuf::from("src");
        if src_candidate.is_dir() {
            src_candidate
        } else {
            PathBuf::from(".")
        }
    }
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

fn collect_imports(items: &[Item]) -> Vec<Vec<String>> {
    items
        .iter()
        .filter_map(|item| {
            if let Item::Import(ImportDef { path, .. }) = item {
                Some(path.clone())
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
    resolver: &vinyl_resolver::ModuleResolver,
    all_items: &mut Vec<Item>,
    visited: &mut HashSet<PathBuf>,
) -> Result<ModuleTable, Vec<CompileError>> {
    let mut module_table: ModuleTable = HashMap::new();

    for import in collect_imports(items) {
        let module_info = resolver.resolve(&import).map_err(|e| {
            vec![CompileError::Module(ModuleError {
                message: format!("could not resolve import `{}`: {e}", import.join("::")),
            })]
        })?;

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

        let sub_table = resolve_imports(&module_items, resolver, all_items, visited)?;
        module_table.extend(sub_table);
    }

    Ok(module_table)
}

pub fn compile_entry(
    file_path: &Path,
    source_root: Option<&Path>,
) -> Result<(CompiledModule, Vec<TypeDiagnostic>), Vec<CompileError>> {
    let source_root = match source_root {
        Some(root) => root.to_path_buf(),
        None => find_source_root(file_path),
    };

    let resolver = vinyl_resolver::ModuleResolver::new(&source_root)
        .map_err(|e| vec![CompileError::ModResolve(e)])?;

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
    let module_table = resolve_imports(&entry_items, &resolver, &mut all_items, &mut visited)?;

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

    Ok((CompiledModule {
        items: hir,
        module_table,
    }, warnings))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn project(name: &str, files: &[(&str, &str)]) -> PathBuf {
        let root = std::env::temp_dir().join(format!("vinyl_compiler_test_{name}"));
        let _ = fs::remove_dir_all(&root);
        for (file, source) in files {
            let path = root.join(file);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, source).unwrap();
        }
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
        let result = compile_entry(
            &root.join("src/main.vn"),
            Some(&root.join("src")),
        );
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
        let result = compile_entry(
            &root.join("src/main.vn"),
            Some(&root.join("src")),
        );
        assert!(result.is_err());
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
        let result = compile_entry(
            &root.join("src/main.vn"),
            Some(&root.join("src")),
        );
        assert!(result.is_ok(), "{result:?}");
    }
}
