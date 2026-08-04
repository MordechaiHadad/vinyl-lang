use vinyl_parser::ast::{
    item::Item,
    types::{Primitive, Type},
};

use super::common;

#[test]
fn struct_definition_lower() {
    let items = common::do_lower("struct Point {\n    x: int32,\n    y: float64,\n}");
    assert_eq!(items.len(), 1);
    let s = match &items[0] {
        Item::Struct(s) => s,
        other => panic!("expected struct, got {other:?}"),
    };
    assert_eq!(s.name, "Point");
    assert_eq!(s.fields.len(), 2);
    assert_eq!(s.fields[0].name, "x");
    assert_eq!(s.fields[0].type_, Type::Primitive(Primitive::Int32));
    assert_eq!(s.fields[1].name, "y");
    assert_eq!(s.fields[1].type_, Type::Primitive(Primitive::Float64));
}

#[test]
fn struct_empty_lower() {
    let items = common::do_lower("struct Empty {}");
    let s = match &items[0] {
        Item::Struct(s) => s,
        other => panic!("expected struct, got {other:?}"),
    };
    assert_eq!(s.name, "Empty");
    assert!(s.fields.is_empty());
}

#[test]
fn struct_field_public_lower() {
    let items = common::do_lower("struct Point {\n    public x: int32,\n    y: float64,\n}");
    let s = match &items[0] {
        Item::Struct(s) => s,
        other => panic!("expected struct, got {other:?}"),
    };
    assert!(s.fields[0].public, "public field should be marked public");
    assert!(!s.fields[1].public, "bare field should stay private");
}
