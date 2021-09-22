use tree_sitter::{Parser, Language};


extern "C" {fn tree_sitter_vinyl() -> Language;}

fn main() {

    let language = unsafe {tree_sitter_vinyl()};
    let mut parser = Parser::new();
    parser.set_language(language).unwrap();

    let source_code = std::fs::read_to_string("vendor/tree-sitter-vinyl/test.vnl").unwrap();
    let tree = parser.parse(&source_code, None).unwrap();
    let root = tree.root_node().to_sexp();

    println!("{}", &source_code);

    println!("{}", root);
    println!("\n");

}
