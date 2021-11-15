mod ast;

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

    let ast = ast::parser::parse_into_ast(&root, &source_code);

    for node in ast.unwrap() {
        println!("{:?}", node);
    }

}