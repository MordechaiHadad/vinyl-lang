use std::fs::File;

use inkwell::{OptimizationLevel, context::Context, targets::{CodeModel, InitializationConfig, RelocMode, Target, TargetTriple}};

use crate::ast::ast::AST;

pub fn test_execute(ast: &Vec<AST>, source: &str) {

    Target::initialize_native(&InitializationConfig::default()).expect("Failed to initialize native target");
    let context = Context::create();
    let module = crate::codegen::llvm::codegen::codegen(&ast, &source, &context);
    let execution_engine = module.create_jit_execution_engine(OptimizationLevel::None).unwrap();
    let target_data = execution_engine.get_target_data();
    let triple = TargetTriple::create("x86_64-pc-win32-unknown");
    let data_layout = target_data.get_data_layout();


    module.set_data_layout(&data_layout);
    module.set_triple(&triple);

}