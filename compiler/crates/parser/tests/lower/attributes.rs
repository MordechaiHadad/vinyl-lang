use vinyl_parser::ast::item::Item;

use super::common;

#[test]
fn attributes() {
    let items = common::do_lower("@deprecated\n@inline(always)\nfn old_function() {}");
    let func = match &items[0] {
        Item::Function(f) => f,
        _ => panic!("expected function"),
    };
    assert_eq!(func.attrs.len(), 2);
    assert_eq!(func.attrs[0].name, "deprecated");
    assert_eq!(func.attrs[1].name, "inline");
}

#[test]
fn documentation_attribute() {
    let items = common::do_lower("@doc(\"hello\n# Header 1\n## Header 2\")\npublic fn add() {}");
    let func = match &items[0] {
        Item::Function(function) => function,
        _ => panic!("expected function"),
    };
    assert!(func.public);
    assert_eq!(func.attrs[0].name, "doc");
    assert!(matches!(
        &func.attrs[0].args[0],
        vinyl_parser::ast::expression::Expression::String(value, _)
            if value == "hello\n# Header 1\n## Header 2"
    ));
}
