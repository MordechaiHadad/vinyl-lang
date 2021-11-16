use crate::ast::ast::{AST};

use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module;

pub fn codegen(ast: Vec<AST>) {
    
    let context = Context::create();
    context.create_builder()

}