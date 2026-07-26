use vinyl_parser::ast::{
    expression::Expression,
    item::Item,
    operator::{BinaryOp, UnaryOp},
    statement::Statement,
};

use super::common;

#[test]
fn binary_expression_structure() {
    let items = common::do_lower("fn f(): int32 { 1 + 2 }");
    let func = match &items[0] {
        Item::Function(f) => f,
        _ => panic!("expected function"),
    };

    let last = func.body.last().unwrap();
    if let Statement::Value(
        Expression::Binary {
            left, op, right, ..
        },
        _,
    ) = last
    {
        assert_eq!(op, &BinaryOp::Add);
        if let Expression::Int(lv, _) = left.as_ref() {
            assert_eq!(*lv, 1);
        } else {
            panic!("expected int literal as left operand");
        }
        if let Expression::Int(rv, _) = right.as_ref() {
            assert_eq!(*rv, 2);
        } else {
            panic!("expected int literal as right operand");
        }
    } else {
        panic!("expected binary expression");
    }
}

#[test]
fn unary_not_bool() {
    let items = common::do_lower("fn f() { let a = !true; let b = not false; }");
    let func = match &items[0] {
        Item::Function(f) => f,
        _ => panic!("expected function"),
    };
    match &func.body[0] {
        Statement::Let {
            value: Expression::Bool(v, _),
            ..
        } => assert!(!*v),
        _ => panic!("expected folded bool literal (!true = false)"),
    }
    match &func.body[1] {
        Statement::Let {
            value: Expression::Bool(v, _),
            ..
        } => assert!(*v),
        _ => panic!("expected folded bool literal (not false = true)"),
    }
}

#[test]
fn unary_neg_folding() {
    let items = common::do_lower("fn f(): int64 { -42 }");
    let func = match &items[0] {
        Item::Function(f) => f,
        _ => panic!("expected function"),
    };
    let last = func.body.last().unwrap();
    match last {
        Statement::Value(Expression::Int(v, _), _) => assert_eq!(*v, -42),
        _ => panic!("expected folded int literal, got {last:?}"),
    }
}

#[test]
fn unary_not_folding_true() {
    let items = common::do_lower("fn f(): bool { !true }");
    let func = match &items[0] {
        Item::Function(f) => f,
        _ => panic!("expected function"),
    };
    let last = func.body.last().unwrap();
    match last {
        Statement::Value(Expression::Bool(v, _), _) => assert!(!*v),
        _ => panic!("expected folded bool literal, got {last:?}"),
    }
}

#[test]
fn unary_not_folding_false() {
    let items = common::do_lower("fn f(): bool { not false }");
    let func = match &items[0] {
        Item::Function(f) => f,
        _ => panic!("expected function"),
    };
    let last = func.body.last().unwrap();
    match last {
        Statement::Value(Expression::Bool(v, _), _) => assert!(*v),
        _ => panic!("expected folded bool literal, got {last:?}"),
    }
}

#[test]
fn unary_neg_variable() {
    let items = common::do_lower("fn f(x: int64): int64 { -x }");
    let func = match &items[0] {
        Item::Function(f) => f,
        _ => panic!("expected function"),
    };
    let last = func.body.last().unwrap();
    match last {
        Statement::Value(
            Expression::Unary {
                op: UnaryOp::Neg,
                operand,
                ..
            },
            _,
        ) => match operand.as_ref() {
            Expression::Ident(name, _) => assert_eq!(name, "x"),
            _ => panic!("expected ident operand"),
        },
        _ => panic!("expected unary neg expression"),
    }
}

#[test]
fn unary_double_not() {
    let items = common::do_lower("fn f(): bool { !!true }");
    let func = match &items[0] {
        Item::Function(f) => f,
        _ => panic!("expected function"),
    };
    let last = func.body.last().unwrap();
    match last {
        Statement::Value(Expression::Bool(v, _), _) => assert!(*v),
        _ => panic!("expected folded bool literal (double not = identity), got {last:?}"),
    }
}

#[test]
fn unary_precedence() {
    let items = common::do_lower("fn f(): int64 { -2 * 3 }");
    let func = match &items[0] {
        Item::Function(f) => f,
        _ => panic!("expected function"),
    };
    let last = func.body.last().unwrap();
    match last {
        Statement::Value(
            Expression::Binary {
                left,
                op: BinaryOp::Mul,
                right,
                ..
            },
            _,
        ) => {
            match left.as_ref() {
                Expression::Int(v, _) => assert_eq!(*v, -2),
                _ => panic!("expected folded int literal -2 as left operand"),
            }
            match right.as_ref() {
                Expression::Int(v, _) => assert_eq!(*v, 3),
                _ => panic!("expected int literal 3 as right operand"),
            }
        }
        _ => {
            panic!("expected binary expression (-2 unfolded to Int(-2) by const folding, then * 3)")
        }
    }
}
