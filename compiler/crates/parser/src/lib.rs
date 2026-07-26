pub mod ast;
pub mod error;
pub mod lower;

pub use ast::*;
pub use error::ParseError;
use tree_sitter::{Parser, Tree};

unsafe extern "C" {
    fn tree_sitter_vinyl() -> *const tree_sitter::ffi::TSLanguage;
}

fn language() -> tree_sitter::Language {
    unsafe { tree_sitter::Language::from_raw(tree_sitter_vinyl()) }
}

pub fn parse(source: &str) -> Result<Tree, Vec<ParseError>> {
    parse_with_name("<input>", source)
}

pub fn parse_with_name(filename: &str, source: &str) -> Result<Tree, Vec<ParseError>> {
    let mut parser = Parser::new();
    parser
        .set_language(&language())
        .expect("vinyl language should be valid");

    let tree = parser
        .parse(source, None)
        .expect("tree-sitter parse should not fail");

    let errors = error::validate_with_name(filename, &tree, source);
    if errors.is_empty() {
        Ok(tree)
    } else {
        Err(errors)
    }
}
