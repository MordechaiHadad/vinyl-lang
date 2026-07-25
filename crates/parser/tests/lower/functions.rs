use vinyl_parser::ast::item::Item;

#[path = "../common/mod.rs"]
mod common;

#[test]
fn function_def() {
    let items = common::do_lower("fn add(a: int32, b: int32): int64 { a + b }");
    assert_eq!(items.len(), 1);
    let func = match &items[0] {
        Item::Function(f) => f,
        _ => panic!("expected function"),
    };
    assert_eq!(func.name, "add");
    assert_eq!(func.params.len(), 2);
    assert_eq!(func.params[0].name, "a");
    assert_eq!(func.params[1].name, "b");
    assert!(func.return_type.is_some());
}

#[test]
fn function_no_return_type() {
    let items = common::do_lower("fn main() {}");
    let func = match &items[0] {
        Item::Function(f) => f,
        _ => panic!("expected function"),
    };
    assert!(func.return_type.is_none());
    assert!(func.params.is_empty());
}

#[test]
fn mut_param() {
    let items = common::do_lower("fn inc(mut x: int32) {}");
    let func = match &items[0] {
        Item::Function(f) => f,
        _ => panic!("expected function"),
    };
    assert_eq!(func.params[0].name, "x");
}

#[test]
fn multiple_functions() {
    let items = common::do_lower("fn foo() {} fn bar() {} fn baz() {}");
    assert_eq!(items.len(), 3);
    for (i, name) in ["foo", "bar", "baz"].iter().enumerate() {
        let f = match &items[i] {
            Item::Function(f) => f,
            _ => panic!("expected function"),
        };
        assert_eq!(f.name, *name);
    }
}
