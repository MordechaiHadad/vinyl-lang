#![allow(unused)]

mod parser;
mod codegen;
mod utilities;
mod analysis;


use std::process::exit;
use tree_sitter::{Language, Node, Parser};
use inkwell::context::Context;
use lasso::Rodeo;
use crate::parser::ast::PrimitiveType;
use crate::parser::ast::LiteralKind;

extern "C" {
    fn tree_sitter_vinyl() -> Language;}

fn main() {
    let language = unsafe { tree_sitter_vinyl() };
    let mut parser = Parser::new();
    let mut rodeo = Rodeo::default();
    parser.set_language(language).unwrap();

    let source_code = std::fs::read_to_string("vendor/tree-sitter-vinyl/test.vnl").unwrap();
    let tree = parser.parse(&source_code, None).unwrap();
    let root = tree.root_node();

    let mut parser_engine = crate::parser::parser::ParserEngine{source: &source_code, rodeo: &mut rodeo};
    let ast = parser_engine.parse_into_ast(&root).unwrap();

    let errors = crate::analysis::type_checker::check_type(&ast);

    let errors_len = errors.len();

    if errors_len > 0 {
        for error in errors {
            println!("{}", error);
        }
        exit(errors_len as i32);
    }

   let context = Context::create();

    let codegen_engine = crate::codegen::llvm::codegen::CodegenEngine{rodeo: &mut rodeo, context: &context, source: &source_code, ast: &ast };

    let module = codegen_engine.codegen();

    // print(&root);
}

fn print(root: &Node) {
    println!("{} {}", root.kind(), root.kind_id());

    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        print(&child);
    }
}
