use vinyl_parser::ast::{
    expression::Expression,
    item::Item,
    statement::Statement,
    types::{Primitive, Type},
};

#[path = "../common/mod.rs"]
mod common;

#[test]
fn tuple_type_in_param() {
    let items = common::do_lower("fn f(x: (int32, float64)) {}");
    let func = match &items[0] {
        Item::Function(f) => f,
        _ => panic!("expected function"),
    };
    assert_eq!(func.params.len(), 1);
    match &func.params[0].type_ {
        Type::Tuple(elements) => {
            assert_eq!(elements.len(), 2);
            assert_eq!(elements[0], Type::Primitive(Primitive::Int32));
            assert_eq!(elements[1], Type::Primitive(Primitive::Float64));
        }
        _ => panic!("expected tuple type"),
    }
}

#[test]
fn tuple_definition_lower() {
    let items = common::do_lower("tuple Pair(int32, float64)");
    assert_eq!(items.len(), 1);
    let t = match &items[0] {
        Item::TupleStruct(t) => t,
        other => panic!("expected tuple struct, got {other:?}"),
    };
    assert_eq!(t.name, "Pair");
    assert_eq!(t.types.len(), 2);
    assert_eq!(t.types[0], Type::Primitive(Primitive::Int32));
    assert_eq!(t.types[1], Type::Primitive(Primitive::Float64));
}

#[test]
fn tuple_empty_lower() {
    let items = common::do_lower("tuple Unit()");
    let t = match &items[0] {
        Item::TupleStruct(t) => t,
        other => panic!("expected tuple struct, got {other:?}"),
    };
    assert_eq!(t.name, "Unit");
    assert!(t.types.is_empty());
}

#[test]
fn tuple_expression_lower() {
    let items = common::do_lower("fn f() { let a = (1, 2); let b = (1,); }");
    let func = match &items[0] {
        Item::Function(f) => f,
        other => panic!("expected function, got {other:?}"),
    };
    match &func.body[0] {
        Statement::Let {
            value: Expression::Tuple(elements, _),
            ..
        } => {
            assert_eq!(elements.len(), 2);
            assert!(matches!(elements[0], Expression::Int(1, _)));
            assert!(matches!(elements[1], Expression::Int(2, _)));
        }
        other => panic!("expected tuple expression, got {other:?}"),
    }
    match &func.body[1] {
        Statement::Let {
            value: Expression::Tuple(elements, _),
            ..
        } => {
            assert_eq!(elements.len(), 1);
            assert!(matches!(elements[0], Expression::Int(1, _)));
        }
        other => panic!("expected tuple expression, got {other:?}"),
    }
}
