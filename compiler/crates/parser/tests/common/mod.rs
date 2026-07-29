use vinyl_parser::{ast::item::Item, parse_and_lower};

pub fn do_lower(source: &str) -> Vec<Item> {
    parse_and_lower(source).unwrap()
}
