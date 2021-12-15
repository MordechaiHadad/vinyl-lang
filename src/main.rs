#![allow(unused)]

mod parser;
mod codegen;
mod utilities;
mod type_checker;


use std::process::exit;
use tree_sitter::{Language, Node, Parser};
use inkwell::context::Context;
use crate::parser::ast::PrimitiveType;
use crate::parser::ast::LiteralKind;

extern "C" {
    fn tree_sitter_vinyl() -> Language;}

fn main() {
    let language = unsafe { tree_sitter_vinyl() };
    let mut parser = Parser::new();
    parser.set_language(language).unwrap();

    let source_code = std::fs::read_to_string("vendor/tree-sitter-vinyl/test.vnl").unwrap();
    let tree = parser.parse(&source_code, None).unwrap();
    let root = tree.root_node();

    let ast = parser::parser::parse_into_ast(&root, &source_code).unwrap();

    let errors = crate::type_checker::type_checker::check_type(&ast);

    let errors_len = errors.len();

    if errors_len > 0 {
        for error in errors {
            println!("{}", error);
        }

        exit(errors_len as i32);
    }



    // utilities::utilities::print_ast(&parser, &source_code);

    codegen::llvm::export::test_execute(&ast, &source_code);

    // print(&root);
}

fn print(root: &Node) {
    println!("{} {}", root.kind(), root.kind_id());

    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        print(&child);
    }
}
