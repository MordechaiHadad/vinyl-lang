use vinyl_parser::ast::item::{Item, TypeAliasDef};
use vinyl_parser::ast::types::Type;

use super::common;

#[test]
fn type_alias_lower() {
    let items = common::do_lower("type PointAlias = Point;");
    assert_eq!(items.len(), 1);
    match &items[0] {
        Item::TypeAlias(a) => {
            assert_eq!(a.name, "PointAlias");
            assert_eq!(a.type_, Type::Named("Point".into()));
        }
        other => panic!("expected type alias, got {other:?}"),
    }
}

#[test]
fn type_alias_private_by_default() {
    let items = common::do_lower("type PrivateAlias = int32;");
    match &items[0] {
        Item::TypeAlias(a) => assert!(!a.public),
        other => panic!("expected type alias, got {other:?}"),
    }
}

#[test]
fn type_alias_public_prefix() {
    let items = common::do_lower("public type PublicAlias = int32;");
    match &items[0] {
        Item::TypeAlias(a) => assert!(a.public),
        other => panic!("expected type alias, got {other:?}"),
    }
}

#[test]
fn type_alias_primitive_target() {
    let items = common::do_lower("type Int = int32;");
    match &items[0] {
        Item::TypeAlias(a) => {
            assert_eq!(a.name, "Int");
            assert_eq!(a.type_, Type::Primitive(vinyl_parser::ast::types::Primitive::Int32));
        }
        other => panic!("expected type alias, got {other:?}"),
    }
}

#[test]
fn type_alias_after_attrs() {
    let items = common::do_lower("@foo\ntype Aliased = int32;");
    match &items[0] {
        Item::TypeAlias(a) => {
            assert_eq!(a.attrs.len(), 1);
            assert_eq!(a.attrs[0].name, "foo");
        }
        other => panic!("expected type alias, got {other:?}"),
    }
}
