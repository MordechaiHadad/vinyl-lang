mod ast;

use ast::{Function, AST};
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module;

use tree_sitter::{Parser, Language, Node};


extern "C" {fn tree_sitter_vinyl() -> Language;}

fn main() {

    let language = unsafe {tree_sitter_vinyl()};
    let mut parser = Parser::new();
    parser.set_language(language).unwrap();

    let source_code = std::fs::read_to_string("vendor/tree-sitter-vinyl/test.vnl").unwrap();
    let tree = parser.parse(&source_code, None).unwrap();
    let root = tree.root_node();
}

// NOTE: Loop through parsed smybols.
fn parser_into_ast(node: &Node, source: &String) -> Option<Vec<AST>> {
    let mut cursor = node.walk();
    let mut ast = Vec::new();
    if node.child_count() > 0 {
        for child in node.children(&mut cursor.clone()) {
            match child.kind() {
                "function_declaration" => println!("hello"),
                "variable_declaration" => {
                    for child in child.children(&mut cursor) {
                        println!("{}", child.kind());
                    }
                }
                _ => continue,
            }
        }
    }

    Some(ast)
}