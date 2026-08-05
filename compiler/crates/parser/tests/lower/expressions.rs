use vinyl_parser::ast::{
    expression::Expression,
    item::Item,
    operator::BinaryOp,
    pattern::{LiteralPattern, Pattern},
    statement::Statement,
};

use super::common;

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
fn module_function_call_is_not_enum_variant() {
    let items = common::do_lower("fn f(): int32 { math::double(5) }");
    let function = match &items[0] {
        Item::Function(function) => function,
        other => panic!("expected function, got {other:?}"),
    };
    match &function.body[0] {
        Statement::Value(Expression::Call { function, args, .. }, _) => {
            assert!(
                matches!(function.as_ref(), Expression::ValuePath { segments, .. } if segments == &["math", "double"])
            );
            assert!(matches!(args.as_slice(), [Expression::Int(5, _)]));
        }
        other => panic!("expected module function call, got {other:?}"),
    }
}

#[test]
fn module_qualified_enum_variant() {
    let items = common::do_lower("fn f(): unit { math::Shape::Circle }");
    let function = match &items[0] {
        Item::Function(function) => function,
        other => panic!("expected function, got {other:?}"),
    };
    match &function.body[0] {
        Statement::Value(
            Expression::EnumVariant {
                type_name, variant_name, args, ..
            },
            _,
        ) => {
            assert_eq!(type_name, "math::Shape");
            assert_eq!(variant_name, "Circle");
            assert!(args.is_empty());
        }
        other => panic!("expected enum variant, got {other:?}"),
    }
}

#[test]
fn module_qualified_enum_variant_with_args() {
    let items = common::do_lower("fn f(): unit { math::Shape::Square(2.0) }");
    let function = match &items[0] {
        Item::Function(function) => function,
        other => panic!("expected function, got {other:?}"),
    };
    match &function.body[0] {
        Statement::Value(
            Expression::EnumVariant {
                type_name,
                variant_name,
                args,
                ..
            },
            _,
        ) => {
            assert_eq!(type_name, "math::Shape");
            assert_eq!(variant_name, "Square");
            assert!(matches!(args.as_slice(), [Expression::Float(2.0, _)]));
        }
        other => panic!("expected enum variant, got {other:?}"),
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

#[test]
fn match_arm_guard_lower() {
    let items = common::do_lower("fn f(x: int32): int32 { match x { 1 if x > 0 => 10, _ => 0 } }");
    let func = match &items[0] {
        Item::Function(f) => f,
        other => panic!("expected function, got {other:?}"),
    };
    let last = func.body.last().unwrap();
    match last {
        Statement::Value(Expression::Match { arms, .. }, _) => {
            assert_eq!(arms.len(), 2);
            match &arms[0].guard {
                Some(guard) => match guard.as_ref() {
                    Expression::Binary { left, op: BinaryOp::Gt, right, .. } => {
                        match left.as_ref() {
                            Expression::Ident(n, _) => assert_eq!(n, "x"),
                            other => panic!("expected ident, got {other:?}"),
                        }
                        match right.as_ref() {
                            Expression::Int(0, _) => {}
                            other => panic!("expected int 0, got {other:?}"),
                        }
                    }
                    other => panic!("expected greater comparison, got {other:?}"),
                },
                None => panic!("expected guard"),
            }
            match &arms[1].guard {
                None => {}
                Some(_) => panic!("expected no guard on wildcard arm"),
            }
        }
        other => panic!("expected match expression, got {other:?}"),
    }
}

#[test]
fn match_enum_variant_pattern_lower() {
    let items = common::do_lower(
        "fn area(s: Shape): int32 {\n\
         match s {\n\
         Shape::Circle(r) if r > 0 => r,\n\
         Shape::Rect(w, h) => w * h,\n\
         Shape::Empty() => 0,\n\
         }\n\
         }",
    );
    let func = match &items[0] {
        Item::Function(f) => f,
        other => panic!("expected function, got {other:?}"),
    };
    let last = func.body.last().unwrap();
    match last {
        Statement::Value(Expression::Match { arms, .. }, _) => {
            assert_eq!(arms.len(), 3);

            match &arms[0].pattern {
                Pattern::EnumVariant { type_path, variant_name, patterns, .. } => {
                    assert_eq!(type_path, "Shape");
                    assert_eq!(variant_name, "Circle");
                    assert_eq!(patterns.len(), 1);
                    match &patterns[0] {
                        Pattern::Ident(n, _) => assert_eq!(n, "r"),
                        other => panic!("expected ident pattern, got {other:?}"),
                    }
                }
                other => panic!("expected enum variant pattern, got {other:?}"),
            }

            match &arms[1].pattern {
                Pattern::EnumVariant { type_path, variant_name, patterns, .. } => {
                    assert_eq!(type_path, "Shape");
                    assert_eq!(variant_name, "Rect");
                    assert_eq!(patterns.len(), 2);
                }
                other => panic!("expected enum variant pattern, got {other:?}"),
            }

            match &arms[2].pattern {
                Pattern::EnumVariant { type_path, variant_name, patterns, .. } => {
                    assert_eq!(type_path, "Shape");
                    assert_eq!(variant_name, "Empty");
                    assert!(patterns.is_empty());
                }
                other => panic!("expected enum variant pattern, got {other:?}"),
            }
        }
        other => panic!("expected match expression, got {other:?}"),
    }
}

