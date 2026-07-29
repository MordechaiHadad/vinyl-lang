use vinyl_parser::ast::{expression::Expression, item::Item, statement::Statement};

use super::common;

#[test]
fn literal_values() {
    let items = common::do_lower(
        "fn f() { let a = 42; let b = 3.14; let c = true; let d = 'x'; let e = \"hello\"; }",
    );
    let func = match &items[0] {
        Item::Function(f) => f,
        _ => panic!("expected function"),
    };

    let check_int = |idx| {
        if let Statement::Let {
            value: Expression::Int(v, _),
            ..
        } = &func.body[idx]
        {
            assert_eq!(*v, 42);
        } else {
            panic!("expected int literal at {idx}");
        }
    };

    let check_float = |idx| {
        if let Statement::Let {
            value: Expression::Float(v, _),
            ..
        } = &func.body[idx]
        {
            assert!((*v - (314.0_f64 / 100.0)).abs() < 1e-10);
        } else {
            panic!("expected float literal at {idx}");
        }
    };

    let check_bool = |idx, expected| {
        if let Statement::Let {
            value: Expression::Bool(v, _),
            ..
        } = &func.body[idx]
        {
            assert_eq!(*v, expected);
        } else {
            panic!("expected bool literal at {idx}");
        }
    };

    let check_char = |idx| {
        if let Statement::Let {
            value: Expression::Char(v, _),
            ..
        } = &func.body[idx]
        {
            assert_eq!(*v, 'x');
        } else {
            panic!("expected char literal at {idx}");
        }
    };

    let check_string = |idx| {
        if let Statement::Let {
            value: Expression::String(v, _),
            ..
        } = &func.body[idx]
        {
            assert_eq!(v, "hello");
        } else {
            panic!("expected string literal at {idx}");
        }
    };

    check_int(0);
    check_float(1);
    check_bool(2, true);
    check_char(3);
    check_string(4);
}

#[test]
fn hex_int_literal() {
    let items = common::do_lower("fn f() { let a = 0xFF; }");
    let func = match &items[0] {
        Item::Function(f) => f,
        _ => panic!("expected function"),
    };
    if let Statement::Let {
        value: Expression::Int(v, _),
        ..
    } = &func.body[0]
    {
        assert_eq!(*v, 255);
    } else {
        panic!("expected int literal");
    }
}

#[test]
fn binary_int_literal() {
    let items = common::do_lower("fn f() { let a = 0b1010; }");
    let func = match &items[0] {
        Item::Function(f) => f,
        _ => panic!("expected function"),
    };
    if let Statement::Let {
        value: Expression::Int(v, _),
        ..
    } = &func.body[0]
    {
        assert_eq!(*v, 10);
    } else {
        panic!("expected int literal");
    }
}

#[test]
fn negative_int_literal() {
    let items = common::do_lower("fn f() { let a = -42; }");
    let func = match &items[0] {
        Item::Function(f) => f,
        _ => panic!("expected function"),
    };
    if let Statement::Let {
        value: Expression::Int(v, _),
        ..
    } = &func.body[0]
    {
        assert_eq!(*v, -42);
    } else {
        panic!("expected int literal (constant folded from -42)");
    }
}

#[test]
fn raw_string() {
    let items = common::do_lower("fn f() { let a = r\"hello\\nworld\"; }");
    let func = match &items[0] {
        Item::Function(f) => f,
        _ => panic!("expected function"),
    };
    if let Statement::Let {
        value: Expression::String(v, _),
        ..
    } = &func.body[0]
    {
        assert_eq!(v, "hello\\nworld");
    } else {
        panic!("expected string literal");
    }
}

#[test]
fn char_literal() {
    let items = common::do_lower("fn f() { let a = 'a'; }");
    let func = match &items[0] {
        Item::Function(f) => f,
        _ => panic!("expected function"),
    };
    if let Statement::Let {
        value: Expression::Char(v, _),
        ..
    } = &func.body[0]
    {
        assert_eq!(*v, 'a');
    } else {
        panic!("expected char literal");
    }
}

#[test]
fn unit_literal_expression() {
    let items = common::do_lower("fn f() { let x = unit; }");
    let func = match &items[0] {
        Item::Function(f) => f,
        _ => panic!("expected function"),
    };
    assert!(matches!(
        func.body[0],
        Statement::Let {
            value: Expression::Unit(_),
            ..
        }
    ));
}
