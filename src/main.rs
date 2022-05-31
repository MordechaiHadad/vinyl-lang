#![allow(unused)]

mod analysis;
mod codegen;
mod parser;
mod utilities;

use analysis::AnalysisEngine;
use ariadne::{Label, Report, ReportKind, Source};
use inkwell::context::Context;
use lasso::Rodeo;
use parser::ast::LiteralKind;
use parser::ast::PrimitiveType;
use std::process::exit;
use tree_sitter::{Language, Node, Parser};

extern "C" {
    fn tree_sitter_vinyl() -> Language;
}

fn main() {
    let language = unsafe { tree_sitter_vinyl() };
    let mut parser = Parser::new();
    let mut rodeo = Rodeo::default();
    parser.set_language(language).unwrap();

    let source_code = include_str!("../vendor/tree-sitter-vinyl/test.vnl");
    let tree = parser.parse(&source_code, None).unwrap();
    let root = tree.root_node();

    let mut parser_engine = parser::ParserEngine::new(&mut rodeo, source_code);
    let ast = parser_engine.parse_into_ast(&root);

    println!("{:#?}", ast);
}
