use vinyl_parser::ast::{
    item::Item,
    types::{Primitive, Type},
};

use super::common;

#[test]
fn primitive_types() {
    let items = common::do_lower(
        "fn f(x: int8, y: uint16, z: float32, w: string, c: char, b: bool, u: unit) {}",
    );
    let func = match &items[0] {
        Item::Function(f) => f,
        _ => panic!("expected function"),
    };

    let expected = [
        (Primitive::Int8, "int8"),
        (Primitive::UInt16, "uint16"),
        (Primitive::Float32, "float32"),
        (Primitive::String, "string"),
        (Primitive::Char, "char"),
        (Primitive::Bool, "bool"),
        (Primitive::Unit, "unit"),
    ];

    for (i, (expected_prim, expected_name)) in expected.iter().enumerate() {
        let param_type = &func.params[i].type_;
        assert_eq!(
            *param_type,
            Type::Primitive(expected_prim.clone()),
            "expected {expected_name} at param {i}, got {param_type:?}"
        );
    }
}

#[test]
fn array_type() {
    let items = common::do_lower("fn f(a: [int32; 5]) {}");
    let func = match &items[0] {
        Item::Function(f) => f,
        _ => panic!("expected function"),
    };
    match &func.params[0].type_ {
        Type::Array { element, size } => {
            assert_eq!(**element, Type::Primitive(Primitive::Int32));
            assert_eq!(*size, 5);
        }
        other => panic!("expected array type, got {other:?}"),
    }
}

#[test]
fn generic_type() {
    let items = common::do_lower("fn f(a: Option<int32>) {}");
    let func = match &items[0] {
        Item::Function(f) => f,
        _ => panic!("expected function"),
    };
    match &func.params[0].type_ {
        Type::Generic { name, args } => {
            assert_eq!(name, "Option");
            assert_eq!(args.len(), 1);
            assert_eq!(args[0], Type::Primitive(Primitive::Int32));
        }
        other => panic!("expected generic type, got {other:?}"),
    }
}

#[test]
fn int_float_type_aliases() {
    let items = common::do_lower("fn f(x: int, y: float) {}");
    let func = match &items[0] {
        Item::Function(f) => f,
        _ => panic!("expected function"),
    };
    assert_eq!(func.params[0].type_, Type::Primitive(Primitive::Int64));
    assert_eq!(func.params[1].type_, Type::Primitive(Primitive::Float64));
}

#[test]
fn scoped_type() {
    let items = common::do_lower("fn f(s: math::Shape) {}");
    let func = match &items[0] {
        Item::Function(f) => f,
        _ => panic!("expected function"),
    };
    assert_eq!(
        func.params[0].type_,
        Type::Named("math::Shape".to_string())
    );
}
