use vinyl_parser::ast::{
    expression::Expression, item::ImportDef, item::Item, statement::Statement,
};

use super::common;

#[test]
fn import_no_prefix() {
    let items = common::do_lower("import math;");
    let import = match &items[0] {
        Item::Import(ImportDef { prefix, path, .. }) => (prefix, path),
        _ => panic!("expected import"),
    };
    assert!(import.0.is_empty(), "no prefix expected");
    assert_eq!(import.1, &["math"]);
}

#[test]
fn import_nested_path_no_prefix() {
    let items = common::do_lower("import utils::format;");
    let import = match &items[0] {
        Item::Import(ImportDef { prefix, path, .. }) => (prefix, path),
        _ => panic!("expected import"),
    };
    assert!(import.0.is_empty(), "no prefix expected");
    assert_eq!(import.1, &["utils", "format"]);
}

#[test]
fn import_self_prefix() {
    let items = common::do_lower("import self::foo;");
    let import = match &items[0] {
        Item::Import(ImportDef { prefix, path, .. }) => (prefix, path),
        _ => panic!("expected import"),
    };
    assert_eq!(import.0, &["self"]);
    assert_eq!(import.1, &["foo"]);
}

#[test]
fn import_parent_prefix() {
    let items = common::do_lower("import parent::bar;");
    let import = match &items[0] {
        Item::Import(ImportDef { prefix, path, .. }) => (prefix, path),
        _ => panic!("expected import"),
    };
    assert_eq!(import.0, &["parent"]);
    assert_eq!(import.1, &["bar"]);
}

#[test]
fn import_stacked_parent_prefix() {
    let items = common::do_lower("import parent::parent::baz;");
    let import = match &items[0] {
        Item::Import(ImportDef { prefix, path, .. }) => (prefix, path),
        _ => panic!("expected import"),
    };
    assert_eq!(import.0, &["parent", "parent"]);
    assert_eq!(import.1, &["baz"]);
}

#[test]
fn import_package_prefix() {
    let items = common::do_lower("import package::qux;");
    let import = match &items[0] {
        Item::Import(ImportDef { prefix, path, .. }) => (prefix, path),
        _ => panic!("expected import"),
    };
    assert_eq!(import.0, &["package"]);
    assert_eq!(import.1, &["qux"]);
}

#[test]
fn import_self_nested_path() {
    let items = common::do_lower("import self::module::math;");
    let import = match &items[0] {
        Item::Import(ImportDef { prefix, path, .. }) => (prefix, path),
        _ => panic!("expected import"),
    };
    assert_eq!(import.0, &["self"]);
    assert_eq!(import.1, &["module", "math"]);
}

#[test]
fn import_bare_keyword_errors() {
    let result = vinyl_parser::parse_and_lower("import parent;");
    assert!(result.is_err(), "bare `parent` keyword should error");
    let result = vinyl_parser::parse_and_lower("import self;");
    assert!(result.is_err(), "bare `self` keyword should error");
    let result = vinyl_parser::parse_and_lower("import package;");
    assert!(result.is_err(), "bare `package` keyword should error");
}

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
