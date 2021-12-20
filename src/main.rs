#![allow(unused)]

mod parser;
mod codegen;
mod analysis;
mod utilities;


use std::process::exit;
use ariadne::{Label, Report, ReportKind, Source};
use tree_sitter::{Language, Node, Parser};
use inkwell::context::Context;
use lasso::Rodeo;
use crate::analysis::AnalysisEngine;
use crate::parser::ast::PrimitiveType;
use crate::parser::ast::LiteralKind;

extern "C" {
    fn tree_sitter_vinyl() -> Language;}

fn main() {
    let language = unsafe { tree_sitter_vinyl() };
    let mut parser = Parser::new();
    let mut rodeo = Rodeo::default();
    parser.set_language(language).unwrap();

    let source_code = include_str!("../vendor/tree-sitter-vinyl/test.vnl");
    let tree = parser.parse(&source_code, None).unwrap();
    let root = tree.root_node();

    let mut parser_engine = crate::parser::parser::ParserEngine{source: &source_code, rodeo: &mut rodeo};
    let ast = match parser_engine.parse_into_ast(&root) {
        Ok(result) => result,
        Err(errors) => {
            for error in &errors {
                println!("{}", error);
            }
            exit(errors.len() as i32);
        }
    };

    for var in &ast.namespaces {
        println!("{:#?}", var);
    }

    let mut analyzer = AnalysisEngine::new(&ast, &mut rodeo, &source_code);
    analyzer.start();

   let context = Context::create();

    let mut codegen = codegen::llvm::codegen::CodegenEngine { rodeo: &mut rodeo, context: &context, source: &source_code, ast: &ast };
    let module = codegen.codegen();


    // print(&root);
}

fn print(root: &Node) {
    println!("{} {}", root.kind(), root.kind_id());

    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        print(&child);
    }
}
