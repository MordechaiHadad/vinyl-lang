use vinyl_parser::ast::{
    expression::Expression,
    item::Item,
    operator::{BinaryOp, UnaryOp},
    statement::Statement,
};

use super::common;

fn unwrap_paren(expr: &Expression) -> &Expression {
    if let Expression::Paren(inner, _) = expr {
        inner.as_ref()
    } else {
        expr
    }
}

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
fn complex_binary_expression_structure() {
    let items = common::do_lower("fn f(): int32 { (1 + 2 * 3 - 4) // 3 }");
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
        assert_eq!(op, &BinaryOp::FloorDiv);
        if let Expression::Binary {
            left: inner_left,
            op: inner_op,
            right: inner_right,
            ..
        } = unwrap_paren(left.as_ref())
        {
            assert_eq!(inner_op, &BinaryOp::Sub);
            if let Expression::Binary {
                left: innermost_left,
                op: innermost_op,
                right: innermost_right,
                ..
            } = unwrap_paren(inner_left.as_ref())
            {
                assert_eq!(innermost_op, &BinaryOp::Add);
                if let Expression::Int(lv, _) = innermost_left.as_ref() {
                    assert_eq!(*lv, 1);
                } else {
                    panic!("expected int literal as left operand of innermost binary expression");
                }
                if let Expression::Binary {
                    left: mul_left,
                    op: mul_op,
                    right: mul_right,
                    ..
                } = unwrap_paren(innermost_right.as_ref())
                {
                    assert_eq!(mul_op, &BinaryOp::Mul);
                    if let Expression::Int(mul_lv, _) = mul_left.as_ref() {
                        assert_eq!(*mul_lv, 2);
                    } else {
                        panic!("expected int literal as left operand of multiplication");
                    }
                    if let Expression::Int(mul_rv, _) = mul_right.as_ref() {
                        assert_eq!(*mul_rv, 3);
                    } else {
                        panic!("expected int literal as right operand of multiplication");
                    }
                } else {
                    panic!("expected binary expression for multiplication");
                }
            } else {
                panic!("expected binary expression for addition");
            }
            if let Expression::Int(rv, _) = inner_right.as_ref() {
                assert_eq!(*rv, 4);
            } else {
                panic!("expected int literal as right operand of subtraction");
            }
        } else {
            panic!("expected binary expression for subtraction");
        }
        if let Expression::Int(rv, _) = right.as_ref() {
            assert_eq!(*rv, 3);
        } else {
            panic!("expected int literal as right operand of floor division");
        }
    } else {
        panic!("expected binary expression for floor division");
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
