mod common;

#[test]
fn private_field_access_same_module_allowed() {
    let source = "struct Point { x: int32 } fn main(): int32 { let p = Point { x: 1 }; p.x }";
    let items = common::compile(source);
    assert!(
        items.is_ok(),
        "same-module private field access should be allowed: {:?}",
        items.err()
    );
}

#[test]
fn private_field_construction_same_module_allowed() {
    let source =
        "struct Point { x: int32, y: int32 } fn main(): unit { let p = Point { x: 1, y: 2 }; }";
    let items = common::compile(source);
    assert!(
        items.is_ok(),
        "same-module private field construction should be allowed: {:?}",
        items.err()
    );
}

#[test]
fn private_variant_same_module_allowed() {
    let source = "enum Shape { Circle } fn main(): unit { let s = Shape::Circle; }";
    let items = common::compile(source);
    assert!(
        items.is_ok(),
        "same-module private variant should be allowed: {:?}",
        items.err()
    );
}
