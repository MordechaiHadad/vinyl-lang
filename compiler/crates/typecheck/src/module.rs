use std::collections::HashMap;

use vinyl_parser::ast::item::FunctionDef;

#[derive(Debug, Clone)]
pub struct ModuleExports {
    pub import_name: String,
    pub import_path: String,
    pub imported: bool,
    pub functions: Vec<FunctionDef>,
    pub types: Vec<String>,
    /// Every user-defined symbol name declared by the module, public or not,
    /// used to distinguish "exists but is private" from "does not exist".
    pub all_symbols: Vec<String>,
}

impl ModuleExports {
    pub fn declares(&self, name: &str) -> bool {
        self.all_symbols.iter().any(|symbol| symbol == name)
    }
}

pub type ModuleTable = HashMap<String, ModuleExports>;

pub fn resolve_module<'a>(
    modules: &'a ModuleTable,
    segments: &[String],
) -> Option<(usize, &'a ModuleExports)> {
    (1..segments.len()).rev().find_map(|length| {
        modules
            .get(&segments[..length].join("::"))
            .map(|exports| (length, exports))
    })
}
