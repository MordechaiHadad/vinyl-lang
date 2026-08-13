pub mod error;

use std::path::{Path, PathBuf};

use vinyl_parser::ast::item::Item;
use vinyl_resolver::resolver::Resolver;
use vinyl_typecheck::TypeDiagnostic;
use vinyl_typecheck::module::ModuleTable;

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
        vinyl_resolver::resolver::ResolverMode::Manifest
    ) {
        return Err(vec![CompileError::Module(ModuleError {
            message: "manifest mode is not supported yet; use script mode without vinyl.toml"
                .to_string(),
        })]);
    }
    if matches!(
        resolver.mode(),
        vinyl_resolver::resolver::ResolverMode::Script
    ) {
        let root = resolver.root().to_path_buf();
        register_script_modules(&mut resolver, root);
    }

    let entry_path = if file_path.is_dir() {
        find_entry_file(file_path).ok_or_else(|| {
            vec![CompileError::Module(ModuleError {
                message: format!(
                    "no entry file found in `{}` (looked for main.vn, lib.vn)",
                    file_path.display()
                ),
            })]
        })?
    } else {
        file_path.to_path_buf()
    };
    let entry_path = entry_path
        .canonicalize()
        .map_err(|error| vec![CompileError::Io(error)])?;
    let (entry_source, entry_source_name, entry_items) = parse_file(&entry_path)?;

    if let Err(diagnostic) =
        vinyl_typecheck::validate_main_return_type(&entry_items, &entry_source, &entry_source_name)
    {
        return Err(vec![CompileError::TypeDiagnostic(*diagnostic)]);
    }

    let graph = resolver.build_module_graph(&entry_path, &entry_items, &mut |path| {
        std::fs::read_to_string(path).map_err(|error| error.to_string())
    });
    if graph.issues.iter().any(|issue| !issue.warning) {
        return Err(graph
            .issues
            .into_iter()
            .filter(|issue| !issue.warning)
            .map(|issue| {
                CompileError::Module(ModuleError {
                    message: issue.message,
                })
            })
            .collect());
    }

    let (hir, warnings) = vinyl_typecheck::typeck_with_modules(
        &graph.all_items,
        &entry_source,
        &entry_source_name,
        &graph.module_table,
    )
    .map_err(|errors| {
        errors
            .into_iter()
            .map(CompileError::TypeDiagnostic)
            .collect::<Vec<_>>()
    })?;

    let mut warnings = warnings;
    warnings.extend(vinyl_typecheck::unused_import_warnings(
        &entry_items,
        &entry_source,
        &entry_source_name,
        &graph.module_table,
    ));

    Ok((
        CompiledModule {
            items: hir,
            module_table: graph.module_table,
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
