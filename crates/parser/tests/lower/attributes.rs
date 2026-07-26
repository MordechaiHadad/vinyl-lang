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
