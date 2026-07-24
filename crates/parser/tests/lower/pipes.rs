use vinyl_parser::ast::{expression::Expression, item::Item, statement::Statement};

#[path = "../common/mod.rs"]
mod common;

fn assert_call(source: &str, expected_function: &str, check_args: impl FnOnce(&[Expression])) {
    let items = common::do_lower(source);
    let function = match &items[0] {
        Item::Function(function) => function,
        other => panic!("expected function, got {other:?}"),
    };
    match function.body.last().unwrap() {
        Statement::Value(Expression::Call { function, args, .. }, _) => {
            assert!(
                matches!(function.as_ref(), Expression::Ident(name, _) if name == expected_function)
            );
            check_args(args);
        }
        other => panic!("expected call expression, got {other:?}"),
    }
}

#[test]
fn pipe_first_arg_lower() {
    assert_call(
        "fn f(x: int32): int32 { x |> double() }",
        "double",
        |args| {
            assert!(matches!(args, [Expression::Ident(name, _)] if name == "x"));
        },
    );
}

#[test]
fn pipe_last_arg_lower() {
    assert_call(
        "fn f(x: int32): int32 { x |>> double() }",
        "double",
        |args| {
            assert!(matches!(args, [Expression::Ident(name, _)] if name == "x"));
        },
    );
}

#[test]
fn pipe_with_existing_args_lower() {
    assert_call("fn f(x: int32): int32 { x |> add(1, 2) }", "add", |args| {
        assert!(
            matches!(args, [Expression::Ident(name, _), Expression::Int(1, _), Expression::Int(2, _)] if name == "x")
        );
    });
}

#[test]
fn pipe_last_with_existing_args_lower() {
    assert_call("fn f(x: int32): int32 { x |>> add(1, 2) }", "add", |args| {
        assert!(
            matches!(args, [Expression::Int(1, _), Expression::Int(2, _), Expression::Ident(name, _)] if name == "x")
        );
    });
}

#[test]
fn pipe_bare_ident_lower() {
    assert_call("fn f(x: int32): int32 { x |> double }", "double", |args| {
        assert!(matches!(args, [Expression::Ident(name, _)] if name == "x"));
    });
}

#[test]
fn pipe_chain_lower() {
    assert_call(
        "fn f(x: int32): int32 { x |> double |> triple }",
        "triple",
        |args| {
            assert!(matches!(args, [Expression::Call { .. }]));
        },
    );
}

#[test]
fn pipe_literal_lower() {
    assert_call("fn f(): int32 { 5 |> double() }", "double", |args| {
        assert!(matches!(args, [Expression::Int(5, _)]));
    });
}
