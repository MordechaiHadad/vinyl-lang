use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use vinyl_parser::ast::item::{ImportDef, Item};
use vinyl_parser::ast::statement::Statement;
use vinyl_typecheck::module::{ModuleExports, ModuleTable};

use crate::resolver::{ModuleInfo, Resolver, ResolverMode};
use crate::{ResolveDiagnostic, resolver::ImportPrefix};

/// A diagnostic attached to a specific file and source span during module graph
/// building. `warning` distinguishes the `parent::`-depth hint from real errors;
/// the compiler fails on non-warning issues, the LSP surfaces them per file.
#[derive(Debug, Clone)]
pub struct ModuleIssue {
    pub file: PathBuf,
    pub offset: usize,
    pub length: usize,
    pub message: String,
    pub warning: bool,
}

/// The result of resolving a file and its (transitive) imports into the
/// typechecker inputs: the module table, the flattened item list, the parsed
/// module sources, and any resolution/parse issues encountered along the way.
#[derive(Debug)]
pub struct ModuleGraph {
    pub module_table: ModuleTable,
    pub all_items: Vec<Item>,
    pub files: Vec<(PathBuf, String, Vec<Item>)>,
    pub issues: Vec<ModuleIssue>,
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

fn all_symbol_names(items: &[Item]) -> Vec<String> {
    items
        .iter()
        .filter_map(|item| match item {
            Item::Import(_) => None,
            _ => Some(item_name(item).to_string()),
        })
        .collect()
}

fn import_display(import: &ImportDef) -> String {
    let mut display = String::new();
    if !import.prefix.is_empty() {
        display.push_str(&import.prefix.join("::"));
        display.push_str("::");
    }
    display.push_str(&import.path.join("::"));
    display
}

/// Validates an import prefix against the allowed `package`/`parent` segments.
/// Mirrors the compiler's richer validation: `self::` is rejected outright,
/// repeated or mixed `package` prefixes are rejected, and deep `parent::` chains
/// produce a warning (the caller continues resolution).
pub fn import_prefix(import: &ImportDef) -> Result<Option<ImportPrefix>, String> {
    if import.prefix.is_empty() {
        return Ok(None);
    }
    let display = import_display(import);
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

    if total_known != import.prefix.len() {
        return Err(format!("unknown import prefix in `{display}`"));
    }
    if self_count > 0 {
        return Err(
            "`self::` prefix refers to the current file, not an external module; \
             use `parent::` for relative imports"
                .to_string(),
        );
    }
    if package_count > 1 {
        return Err(format!(
            "`package` prefix can only appear once in `{display}`"
        ));
    }
    if package_count > 0 && parent_count > 0 {
        return Err(format!(
            "cannot combine `package` and `parent` prefixes in `{display}`"
        ));
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

fn parent_depth_warning(import: &ImportDef) -> Option<String> {
    let parent_count = import
        .prefix
        .iter()
        .filter(|s| s.as_str() == "parent")
        .count();
    (parent_count >= 4).then(|| {
        format!(
            "import `{}` uses {parent_count} levels of `parent::`; \
             consider moving to a manifest-based project",
            import_display(import)
        )
    })
}

struct Collector<'a> {
    resolver: &'a mut Resolver,
    all_items: &'a mut Vec<Item>,
    module_table: &'a mut ModuleTable,
    visited: &'a mut HashSet<PathBuf>,
    files: &'a mut Vec<(PathBuf, String, Vec<Item>)>,
    files_seen: &'a mut HashSet<PathBuf>,
    issues: &'a mut Vec<ModuleIssue>,
    bare_imported_symbols: &'a mut HashSet<String>,
    read_source: &'a mut dyn FnMut(&Path) -> Result<String, String>,
}

impl Resolver {
    /// Builds the module graph for `from`, following all of its imports and
    /// registering the remaining known modules, with LSP semantics as canonical:
    /// bare `math::` references require an explicit `import math;`, while
    /// `parent::math::` qualified references work without one.
    pub fn build_module_graph(
        &mut self,
        from: &Path,
        items: &[Item],
        read_source: &mut dyn FnMut(&Path) -> Result<String, String>,
    ) -> ModuleGraph {
        let mut all_items = items.to_vec();
        let mut module_table = ModuleTable::new();
        let mut visited = HashSet::new();
        let mut files = Vec::new();
        let mut files_seen = HashSet::new();
        let mut issues = Vec::new();
        let mut bare_imported_symbols = HashSet::new();

        if let Ok(canonical) = from.canonicalize() {
            visited.insert(canonical);
        }

        {
            let mut collector = Collector {
                resolver: self,
                all_items: &mut all_items,
                module_table: &mut module_table,
                visited: &mut visited,
                files: &mut files,
                files_seen: &mut files_seen,
                issues: &mut issues,
                bare_imported_symbols: &mut bare_imported_symbols,
                read_source,
            };
            collector.collect_modules(from, items);
            collector.add_resolved_modules(from);
        }

        ModuleGraph {
            module_table,
            all_items,
            files,
            issues,
        }
    }

    /// Import path used to address `to` from `from`, e.g. `parent::math` for a
    /// sibling module. Both paths are canonicalized before comparing so that the
    /// Windows `\\?\`-prefixed canonical entry path never diverges from the plain
    /// resolver paths (the original Windows-only resolution bug).
    pub fn relative_import_path(&self, from_file: &Path, to_module: &Path) -> String {
        let to_stem = to_module
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        if let (Ok(from_canon), Ok(to_canon)) = (
            from_file.parent().unwrap_or(Path::new("")).canonicalize(),
            to_module.canonicalize(),
        ) && from_canon == to_canon.parent().unwrap_or(Path::new(""))
        {
            return format!("parent::{to_stem}");
        }
        let source_root = match self.mode() {
            ResolverMode::Manifest => self.root().join("src"),
            ResolverMode::Script => self.root().to_path_buf(),
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
}

impl<'a> Collector<'a> {
    fn collect_modules(&mut self, from: &Path, items: &[Item]) {
        let imports: Vec<(ImportDef, bool)> = imports_in_items(items)
            .into_iter()
            .flat_map(|(import, local)| {
                if import.symbols.is_empty() {
                    return vec![(import.clone(), local)];
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
                            path_spans: Vec::new(),
                            symbols: Vec::new(),
                            symbol_spans: Vec::new(),
                            wildcard: false,
                        }
                    })
                    .map(|import| (import, local))
                    .collect()
            })
            .collect();

        for (item, local) in &imports {
            if let Some(warning) = parent_depth_warning(item) {
                self.push_issue(from, item, warning, true);
            }
            let prefix = match import_prefix(item) {
                Ok(prefix) => prefix,
                Err(message) => {
                    self.push_issue(from, item, message, false);
                    continue;
                }
            };
            let (info, symbol) = if item.wildcard || item.path.len() <= 1 {
                match self.resolve_module_with_prefix(&prefix, &item.path, from) {
                    Ok(info) => (info, None),
                    Err(diagnostic) => {
                        self.push_resolve_issue(from, item, diagnostic);
                        continue;
                    }
                }
            } else {
                match self.resolve_module_with_prefix(&prefix, &item.path, from) {
                    Ok(info) => (info, None),
                    Err(first_error) => {
                        let parent_path = item.path[..item.path.len() - 1].to_vec();
                        match self.resolve_module_with_prefix(&prefix, &parent_path, from) {
                            Ok(info) => (info, Some(item.path.last().unwrap().clone())),
                            Err(_) => {
                                self.push_resolve_issue(from, item, first_error);
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
            let already_visited = !self.visited.insert(path.clone());
            if already_visited && symbol.is_none() {
                continue;
            }
            let (module_source, module_items) = match self.parse_file(&path) {
                Ok(result) => result,
                Err(module_issues) => {
                    self.issues.extend(module_issues);
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
                    Item::TypeAlias(alias) if alias.public => {
                        types.push(alias.name.clone());
                        type_items.push(module_item.clone());
                    }
                    _ => {}
                }
            }

            if let Some(symbol_name) = symbol {
                if !*local {
                    self.all_items.extend(type_items.iter().cloned());
                }
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
                    if *local {
                        continue;
                    }
                    self.push_issue(
                        from,
                        item,
                        format!(
                            "no public symbol `{symbol_name}` in module `{}`",
                            info.import_name
                        ),
                        false,
                    );
                    continue;
                };
                if !*local {
                    if !self.bare_imported_symbols.insert(symbol_name.clone()) {
                        self.push_issue(
                            from,
                            item,
                            format!("import of `{symbol_name}` conflicts with an existing import"),
                            false,
                        );
                        continue;
                    }
                    self.all_items.push(injected);
                }
            } else if item.wildcard && !*local {
                for function in &functions {
                    if !self.bare_imported_symbols.insert(function.name.clone()) {
                        self.push_issue(
                            from,
                            item,
                            format!(
                                "import of `{}` conflicts with an existing import",
                                function.name
                            ),
                            false,
                        );
                        continue;
                    }
                    self.all_items.push(Item::Function(function.clone()));
                }
                for type_item in &type_items {
                    if !self
                        .bare_imported_symbols
                        .insert(item_name(type_item).to_string())
                    {
                        self.push_issue(
                            from,
                            item,
                            format!(
                                "import of `{}` conflicts with an existing import",
                                item_name(type_item)
                            ),
                            false,
                        );
                        continue;
                    }
                    self.all_items.push(type_item.clone());
                }
            } else if !*local {
                for function in &functions {
                    let mut imported = function.clone();
                    imported.name = format!("{}::{}", info.import_name, imported.name);
                    self.all_items.push(Item::Function(imported));
                }
                for type_item in &type_items {
                    self.all_items.push(type_item.clone());
                }
                for module_item in &module_items {
                    if let Item::Enum(enumeration) = module_item {
                        if enumeration.public {
                            continue;
                        }
                        let mut enumeration = enumeration.clone();
                        enumeration.name = format!("{}::{}", info.import_name, enumeration.name);
                        self.all_items.push(Item::Enum(enumeration));
                    }
                }
            }

            if already_visited {
                continue;
            }
            let all_symbols = all_symbol_names(&module_items);
            let exports = ModuleExports {
                import_name: info.import_name.clone(),
                import_path: self.resolver.relative_import_path(from, &info.file_path),
                imported: true,
                functions,
                types,
                all_symbols,
            };
            self.module_table
                .insert(info.import_name.clone(), exports.clone());
            self.module_table
                .insert(info.path.join("::"), exports.clone());
            self.module_table
                .insert(exports.import_path.clone(), exports);
            self.push_file(&path, module_source, module_items.clone());
            self.collect_modules(&info.file_path, &module_items);
        }
    }

    fn add_resolved_modules(&mut self, from: &Path) {
        let all_modules: Vec<ModuleInfo> = self.resolver.all_modules().values().cloned().collect();
        for info in all_modules {
            if self.module_table.contains_key(&info.import_name) {
                continue;
            }
            let (module_source, module_items) = match self.parse_file(&info.file_path) {
                Ok(result) => result,
                Err(module_issues) => {
                    self.issues.extend(module_issues);
                    continue;
                }
            };
            self.push_file(&info.file_path, module_source, module_items.clone());
            self.all_items
                .extend(module_items.iter().filter_map(|item| {
                    let mut item = item.clone();
                    let is_public = match &mut item {
                        Item::Struct(structure) => structure.public,
                        Item::TupleStruct(tuple) => tuple.public,
                        Item::Enum(enumeration) => enumeration.public,
                        Item::TypeAlias(alias) => alias.public,
                        _ => return None,
                    };
                    if !is_public {
                        return None;
                    }
                    match &mut item {
                        Item::Struct(structure) => {
                            structure.name = format!("{}::{}", info.import_name, structure.name)
                        }
                        Item::TupleStruct(tuple) => {
                            tuple.name = format!("{}::{}", info.import_name, tuple.name)
                        }
                        Item::Enum(enumeration) => {
                            enumeration.name = format!("{}::{}", info.import_name, enumeration.name)
                        }
                        Item::TypeAlias(alias) => {
                            alias.name = format!("{}::{}", info.import_name, alias.name)
                        }
                        _ => unreachable!(),
                    }
                    Some(item)
                }));
            self.all_items
                .extend(module_items.iter().filter_map(|item| {
                    let Item::Enum(enumeration) = item else {
                        return None;
                    };
                    if enumeration.public {
                        return None;
                    }
                    let mut enumeration = enumeration.clone();
                    enumeration.name = format!("{}::{}", info.import_name, enumeration.name);
                    Some(Item::Enum(enumeration))
                }));
            let functions = module_items
                .iter()
                .filter_map(|item| match item {
                    Item::Function(function) if function.public => Some(function.clone()),
                    _ => None,
                })
                .collect();
            let types = module_items
                .iter()
                .filter_map(|item| match item {
                    Item::Struct(structure) if structure.public => Some(structure.name.clone()),
                    Item::TupleStruct(tuple) if tuple.public => Some(tuple.name.clone()),
                    Item::Enum(enumeration) if enumeration.public => Some(enumeration.name.clone()),
                    Item::TypeAlias(alias) if alias.public => Some(alias.name.clone()),
                    _ => None,
                })
                .collect();
            let all_symbols = all_symbol_names(&module_items);
            let exports = ModuleExports {
                import_name: info.import_name.clone(),
                import_path: self.resolver.relative_import_path(from, &info.file_path),
                imported: false,
                functions,
                types,
                all_symbols,
            };
            self.module_table
                .insert(info.import_name.clone(), exports.clone());
            self.module_table
                .insert(info.path.join("::"), exports.clone());
            let inline_exports = ModuleExports {
                imported: true,
                ..exports.clone()
            };
            self.module_table
                .insert(exports.import_path.clone(), inline_exports);
        }
    }

    fn resolve_module_with_prefix(
        &mut self,
        prefix: &Option<ImportPrefix>,
        path: &[String],
        from: &Path,
    ) -> Result<ModuleInfo, ResolveDiagnostic> {
        match prefix {
            None => self.resolver.resolve_module_path(path),
            Some(prefix) => {
                let path_strs: Vec<&str> = path.iter().map(|segment| segment.as_str()).collect();
                self.resolver.resolve(prefix, &path_strs, from)
            }
        }
    }

    fn parse_file(&mut self, path: &Path) -> Result<(String, Vec<Item>), Vec<ModuleIssue>> {
        let source = match (self.read_source)(path) {
            Ok(source) => source,
            Err(message) => {
                return Err(vec![ModuleIssue {
                    file: path.to_path_buf(),
                    offset: 0,
                    length: 0,
                    message,
                    warning: false,
                }]);
            }
        };
        let name = path.to_string_lossy().to_string();
        let items = match vinyl_parser::parse_and_lower_with_name(&name, &source) {
            Ok(items) => items,
            Err(errors) => {
                return Err(errors
                    .into_iter()
                    .map(|error| ModuleIssue {
                        file: path.to_path_buf(),
                        offset: error.span.offset(),
                        length: error.span.len(),
                        message: format!("{error}"),
                        warning: false,
                    })
                    .collect());
            }
        };
        Ok((source, items))
    }

    fn push_resolve_issue(
        &mut self,
        from: &Path,
        import: &ImportDef,
        diagnostic: ResolveDiagnostic,
    ) {
        self.push_issue(
            from,
            import,
            format!(
                "could not resolve import `{}`: {diagnostic}",
                import_display(import)
            ),
            false,
        );
    }

    fn push_issue(&mut self, from: &Path, import: &ImportDef, message: String, warning: bool) {
        self.issues.push(ModuleIssue {
            file: from.to_path_buf(),
            offset: import.span.offset(),
            length: import.span.len(),
            message,
            warning,
        });
    }

    fn push_file(&mut self, path: &Path, source: String, items: Vec<Item>) {
        let key = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        if self.files_seen.insert(key) {
            self.files.push((path.to_path_buf(), source, items));
        }
    }
}

fn imports_in_items(items: &[Item]) -> Vec<(&ImportDef, bool)> {
    let mut imports = Vec::new();
    for item in items {
        match item {
            Item::Import(import) => imports.push((import, false)),
            Item::Function(function) => imports_in_statements(&function.body, &mut imports),
            _ => {}
        }
    }
    imports
}

fn imports_in_statements<'a>(
    statements: &'a [Statement],
    imports: &mut Vec<(&'a ImportDef, bool)>,
) {
    for statement in statements {
        match statement {
            Statement::Import(import) => imports.push((import, true)),
            Statement::Loop { body, .. }
            | Statement::If {
                then_block: body, ..
            }
            | Statement::While { body, .. } => imports_in_statements(body, imports),
            Statement::Expression(expression) | Statement::Value(expression, _) => {
                imports_in_expression(expression, imports)
            }
            Statement::Let { value, .. } => imports_in_expression(value, imports),
            Statement::Return(Some(expression), _) => imports_in_expression(expression, imports),
            Statement::Assign { value, .. } => imports_in_expression(value, imports),
            Statement::Return(None, _) | Statement::Break(_) | Statement::Continue(_) => {}
        }
    }
}

fn imports_in_expression<'a>(
    expression: &'a vinyl_parser::ast::expression::Expression,
    imports: &mut Vec<(&'a ImportDef, bool)>,
) {
    use vinyl_parser::ast::expression::Expression;
    match expression {
        Expression::Block(statements, _) => imports_in_statements(statements, imports),
        Expression::If {
            then_block,
            else_if,
            else_block,
            ..
        } => {
            imports_in_statements(then_block, imports);
            for (_, block) in else_if {
                imports_in_statements(block, imports);
            }
            if let Some(block) = else_block {
                imports_in_statements(block, imports);
            }
        }
        _ => {}
    }
}
