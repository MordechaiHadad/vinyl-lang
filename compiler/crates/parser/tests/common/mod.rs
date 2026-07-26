use vinyl_parser::{ast::item::Item, lower::lower};

pub fn do_lower(source: &str) -> Vec<Item> {
    let tree = vinyl_parser::parse(source).unwrap();
    lower(&tree, source, "<test>").unwrap()
}
