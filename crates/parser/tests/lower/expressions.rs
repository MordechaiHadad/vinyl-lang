use vinyl_parser::ast::{
    expression::Expression,
    item::Item,
    pattern::{LiteralPattern, Pattern},
    statement::Statement,
};

#[path = "../common/mod.rs"]
mod common;

#[test]
fn if_expression() {
    let items = common::do_lower("fn f(): int32 { if true { 1 } else { 2 } }");
    let func = match &items[0] {
        Item::Function(f) => f,
        _ => panic!("expected function"),
    };

    let last = func.body.last().unwrap();
    let expr = match last {
        Statement::Value(e, _) => e,
        _ => panic!("expected value statement"),
    };
    match expr {
        Expression::If {
            condition,
            then_block,
            else_if,
            else_block,
            ..
        } => {
            assert!(matches!(condition.as_ref(), Expression::Bool(true, _)));
            assert!(!then_block.is_empty());
            assert!(else_if.is_empty());
            assert!(else_block.is_some());
        }
        _ => panic!("expected if expression"),
    }
}

#[test]
fn array_expression() {
    let items = common::do_lower("fn f() { let a = [1, 2, 3]; }");
    let func = match &items[0] {
        Item::Function(f) => f,
        _ => panic!("expected function"),
    };
    if let Statement::Let {
        value: Expression::Array(elements, _),
        ..
    } = &func.body[0]
    {
        assert_eq!(elements.len(), 3);
    } else {
        panic!("expected array expression");
    }
}

#[test]
fn field_access_lower() {
    let items = common::do_lower("fn f(p: Point): int32 { p.x }");
    let func = match &items[0] {
        Item::Function(f) => f,
        other => panic!("expected function, got {other:?}"),
    };
    let last = func.body.last().unwrap();
    match last {
        Statement::Value(Expression::Field { object, name, .. }, _) => {
            assert_eq!(name, "x");
            match object.as_ref() {
                Expression::Ident(n, _) => assert_eq!(n, "p"),
                other => panic!("expected ident, got {other:?}"),
            }
        }
        other => panic!("expected field access, got {other:?}"),
    }
}

#[test]
fn match_expression_lower() {
    let items = common::do_lower("fn f(x: int32): int32 { match x { 1 => 10, _ => 0 } }");
    let func = match &items[0] {
        Item::Function(f) => f,
        other => panic!("expected function, got {other:?}"),
    };
    let last = func.body.last().unwrap();
    match last {
        Statement::Value(Expression::Match { value, arms, .. }, _) => {
            match value.as_ref() {
                Expression::Ident(n, _) => assert_eq!(n, "x"),
                other => panic!("expected ident, got {other:?}"),
            }
            assert_eq!(arms.len(), 2);

            match &arms[0].pattern {
                Pattern::Literal(LiteralPattern::Int(1), _) => {}
                other => panic!("expected int literal pattern, got {other:?}"),
            }
            match &arms[0].body.as_ref() {
                Expression::Int(10, _) => {}
                other => panic!("expected int body, got {other:?}"),
            }

            match &arms[1].pattern {
                Pattern::Wildcard(_) => {}
                other => panic!("expected wildcard pattern, got {other:?}"),
            }
        }
        other => panic!("expected match expression, got {other:?}"),
    }
}
