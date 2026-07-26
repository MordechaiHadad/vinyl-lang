use vinyl_parser::ast::{expression::Expression, item::Item, statement::Statement};

use super::common;

#[test]
fn let_statements() {
    let items =
        common::do_lower("fn f() { let x: int32 = 42; let y = 10; let mut z: float64 = 3.14; }");
    let func = match &items[0] {
        Item::Function(f) => f,
        _ => panic!("expected function"),
    };
    assert_eq!(func.body.len(), 3);

    if let Statement::Let {
        name,
        mutable,
        type_,
        ..
    } = &func.body[0]
    {
        assert_eq!(name, "x");
        assert!(!mutable);
        assert!(type_.is_some());
    } else {
        panic!("expected let statement");
    }

    if let Statement::Let {
        name,
        mutable,
        type_,
        ..
    } = &func.body[1]
    {
        assert_eq!(name, "y");
        assert!(!mutable);
        assert!(type_.is_none());
    } else {
        panic!("expected let statement");
    }

    if let Statement::Let {
        name,
        mutable,
        type_,
        ..
    } = &func.body[2]
    {
        assert_eq!(name, "z");
        assert!(*mutable);
        assert!(type_.is_some());
    } else {
        panic!("expected let statement");
    }
}

#[test]
fn return_statement() {
    let items = common::do_lower("fn f(): int32 { return 42; }");
    let func = match &items[0] {
        Item::Function(f) => f,
        _ => panic!("expected function"),
    };
    match &func.body[0] {
        Statement::Return(Some(Expression::Int(v, _)), _) => {
            assert_eq!(*v, 42);
        }
        other => panic!("expected return with int, got {:?}", other),
    }
}

#[test]
fn return_void() {
    let items = common::do_lower("fn f() { return; }");
    let func = match &items[0] {
        Item::Function(f) => f,
        _ => panic!("expected function"),
    };
    match &func.body[0] {
        Statement::Return(None, _) => {}
        other => panic!("expected return without value, got {:?}", other),
    }
}
