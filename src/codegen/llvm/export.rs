use std::fs::File;

use inkwell::{
    context::Context,
    targets::{CodeModel, InitializationConfig, RelocMode, Target, TargetTriple},
    OptimizationLevel,
};

use crate::parser::ast::AST;

pub fn test_execute(ast: &[AST], source: &str) {
    Target::initialize_native(&InitializationConfig::default())
        .expect("Failed to initialize native target");
    let context = Context::create();
}
