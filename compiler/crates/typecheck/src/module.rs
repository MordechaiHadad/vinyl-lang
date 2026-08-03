use std::collections::HashMap;

use vinyl_parser::ast::item::FunctionDef;

#[derive(Debug, Clone)]
pub struct ModuleExports {
    pub import_name: String,
    pub import_path: String,
    pub imported: bool,
    pub functions: Vec<FunctionDef>,
    pub types: Vec<String>,
}

pub type ModuleTable = HashMap<String, ModuleExports>;
