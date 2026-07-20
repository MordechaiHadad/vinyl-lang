use vinyl_parser::ast::Item;
use vinyl_parser::lower::lower;

pub fn do_lower(source: &str) -> Vec<Item> {
    let tree = vinyl_parser::parse(source).unwrap();
    lower(&tree, source, "<test>").unwrap()
}
