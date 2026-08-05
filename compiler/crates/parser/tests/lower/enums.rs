use vinyl_parser::ast::{
    item::{EnumVariantData, Item},
    types::{Primitive, Type},
};

use super::common;

#[test]
fn enum_definition_lower() {
    let items = common::do_lower(
        "enum Option {\n    None,\n    Some(int32),\n    Error { code: int32, message: string },\n}",
    );
    assert_eq!(items.len(), 1);
    let e = match &items[0] {
        Item::Enum(e) => e,
        other => panic!("expected enum, got {other:?}"),
    };
    assert_eq!(e.variants.len(), 3);

    assert_eq!(e.variants[0].name, "None");
    assert!(e.variants[0].data.is_none());

    assert_eq!(e.variants[1].name, "Some");
    match &e.variants[1].data {
        Some(EnumVariantData::Tuple(types)) => {
            assert_eq!(types.len(), 1);
            assert_eq!(types[0], Type::Primitive(Primitive::Int32));
        }
        other => panic!("expected tuple variant, got {other:?}"),
    }

    assert_eq!(e.variants[2].name, "Error");
    match &e.variants[2].data {
        Some(EnumVariantData::Struct(fields)) => {
            assert_eq!(fields.len(), 2);
            assert_eq!(fields[0].name, "code");
            assert_eq!(fields[1].name, "message");
        }
        other => panic!("expected struct variant, got {other:?}"),
    }
}

#[test]
fn enum_variant_public_lower() {
    let items = common::do_lower("enum Shape {\n    public Circle,\n    Square(float64),\n}");
    let e = match &items[0] {
        Item::Enum(e) => e,
        other => panic!("expected enum, got {other:?}"),
    };
    assert!(
        e.variants[0].public,
        "public variant should be marked public"
    );
    assert!(!e.variants[1].public, "bare variant should stay private");
}
