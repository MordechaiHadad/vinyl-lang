use vinyl_parser::ast::{
    expression::Expression,
    item::Item,
    operator::AssignOp,
    statement::{AssignTarget, Statement},
    types::{Primitive, Type},
};

mod common;

#[test]
fn reference_type_in_param() {
    let items = common::do_lower("fn f(p: &int32) {}");
    let func = match &items[0] {
        Item::Function(f) => f,
        _ => panic!("expected function"),
    };
    assert_eq!(func.params.len(), 1);
    assert_eq!(func.params[0].name, "p");
    assert_eq!(
        func.params[0].type_,
        Type::Ref(Box::new(Type::Primitive(Primitive::Int32)))
    );
}

#[test]
fn reference_type_in_let() {
    let items = common::do_lower("fn f() { let x: &int32 = &y; }");
    let func = match &items[0] {
        Item::Function(f) => f,
        _ => panic!("expected function"),
    };
    if let Statement::Let {
        type_: Some(t),
        value,
        ..
    } = &func.body[0]
    {
        assert_eq!(*t, Type::Ref(Box::new(Type::Primitive(Primitive::Int32))));
        assert!(matches!(value, Expression::Ref { .. }));
    } else {
        panic!("expected let with ref type and ref expr");
    }
}

#[test]
fn simple_assignment() {
    let items = common::do_lower("fn f() { let mut x = 5; x = 10; }");
    let func = match &items[0] {
        Item::Function(f) => f,
        _ => panic!("expected function"),
    };
    assert_eq!(func.body.len(), 2);
    match &func.body[1] {
        Statement::Assign {
            target: AssignTarget::Ident(name, _),
            op: AssignOp::Eq,
            value,
            ..
        } => {
            assert_eq!(name, "x");
            assert!(matches!(value.as_ref(), Expression::Int(v, _) if *v == 10));
        }
        other => panic!("expected assign to ident, got {other:?}"),
    }
}

#[test]
fn compound_assign_add_eq() {
    let items = common::do_lower("fn f() { let mut x = 5; x += 3; }");
    let func = match &items[0] {
        Item::Function(f) => f,
        _ => panic!("expected function"),
    };
    match &func.body[1] {
        Statement::Assign {
            target: AssignTarget::Ident(name, _),
            op: AssignOp::AddEq,
            ..
        } => {
            assert_eq!(name, "x");
        }
        other => panic!("expected += assign, got {other:?}"),
    }
}

#[test]
fn ref_expression() {
    let items = common::do_lower("fn f() { let x = 5; let y = &x; }");
    let func = match &items[0] {
        Item::Function(f) => f,
        _ => panic!("expected function"),
    };
    match &func.body[1] {
        Statement::Let { value, .. } => {
            assert!(
                matches!(value, Expression::Ref { operand, .. } if matches!(&**operand, Expression::Ident(n, _) if n == "x"))
            );
        }
        other => panic!("expected let with ref expr, got {other:?}"),
    }
}

#[test]
fn assign_op_equality() {
    for (source, expected_op) in [
        ("x = 1", AssignOp::Eq),
        ("x += 1", AssignOp::AddEq),
        ("x -= 1", AssignOp::SubEq),
        ("x *= 1", AssignOp::MulEq),
        ("x /= 1", AssignOp::DivEq),
        ("x %= 1", AssignOp::RemEq),
        ("x &= 1", AssignOp::BitAndEq),
        ("x |= 1", AssignOp::BitOrEq),
        ("x ^= 1", AssignOp::BitXorEq),
        ("x <<= 1", AssignOp::ShlEq),
        ("x >>= 1", AssignOp::ShrEq),
    ] {
        let items = common::do_lower(&format!("fn f() {{ let mut x = 0; {source}; }}"));
        let func = match &items[0] {
            Item::Function(f) => f,
            _ => panic!("expected function"),
        };
        match &func.body[1] {
            Statement::Assign { op, .. } => {
                assert_eq!(*op, expected_op, "op mismatch for {source}")
            }
            other => panic!("expected assign for {source}, got {other:?}"),
        }
    }
}

#[test]
fn assign_target_index() {
    let items = common::do_lower("fn f() { let arr = [1, 2, 3]; arr[0] = 42; }");
    let func = match &items[0] {
        Item::Function(f) => f,
        _ => panic!("expected function"),
    };
    match &func.body[1] {
        Statement::Assign {
            target:
                AssignTarget::Index {
                    span: _,
                    array,
                    index,
                },
            op: AssignOp::Eq,
            ..
        } => {
            assert!(matches!(array.as_ref(), Expression::Ident(name, _) if name == "arr"));
            assert!(matches!(index.as_ref(), Expression::Int(v, _) if *v == 0));
        }
        other => panic!("expected index assign, got {other:?}"),
    }
}
