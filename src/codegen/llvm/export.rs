use std::fs::File;

use inkwell::{OptimizationLevel, context::Context, targets::{CodeModel, InitializationConfig, RelocMode, Target, TargetTriple}};

use crate::parser::ast::AST;

pub fn test_execute(ast: &Vec<AST>, source: &str) {

    Target::initialize_native(&InitializationConfig::default()).expect("Failed to initialize native target");
    let context = Context::create();
}