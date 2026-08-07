mod common;

#[test]
fn simple_let() {
    let source = "fn main() { let x = 42; }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn documentation_attribute_reaches_hir() {
    let items = common::compile("@doc(\"Entry point\")\nfn main() {}").unwrap();
    let function = match &items[0].kind {
        vinyl_typecheck::hir::HirItemKind::Function(function) => function,
        _ => panic!("expected function"),
    };
    assert_eq!(function.documentation.as_deref(), Some("Entry point"));
}

#[test]
fn annotated_let() {
    let source = "fn main() { let x: int32 = 42; }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn type_mismatch_annotation() {
    let source = "fn main() { let x: int32 = true; }";
    let items = common::compile(source);
    assert!(items.is_err(), "typeck should fail: type mismatch");
}

#[test]
fn if_condition_bool() {
    let source = "fn main() { if true { let x = 1; } }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn if_condition_not_bool() {
    let source = "fn main() { if 1 { let x = 1; } }";
    let items = common::compile(source);
    assert!(items.is_err(), "typeck should fail: if condition is int");
}

#[test]
fn if_condition_string_rejected() {
    let source = "fn main() { if \"hello\" { 1 } }";
    let items = common::compile(source);
    assert!(items.is_err(), "typeck should fail: if condition is string");
}

#[test]
fn if_condition_is_comparison() {
    let source = "fn main(): int32 { if 3 < 5 { 1 } else { 0 } }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn if_condition_is_logical() {
    let source = "fn main(): int32 { if true || false { 1 } else { 0 } }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn if_else() {
    let source = "fn main(): int32 { if true { 1 } else { 2 } }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn nested_if() {
    let source = "fn main(): int32 { if true { if false { 0 } else { 1 } } else { 2 } }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn if_else_type_mismatch() {
    let source = "fn f(): int32 { if true { 1 } else { \"hello\" } }";
    let items = common::compile(source);
    assert!(items.is_err(), "typeck should fail: branch types differ");
}

#[test]
fn if_no_else_returns_unit() {
    let source = "fn f(): int32 { if true { 1 } }";
    let items = common::compile(source);
    assert!(
        items.is_err(),
        "typeck should fail: if without else yields unit"
    );
}

#[test]
fn undefined_variable() {
    let source = "fn main() { let x = y; }";
    let items = common::compile(source);
    assert!(items.is_err(), "typeck should fail: undefined variable");
    let errors = items.err().unwrap();
    assert!(
        errors
            .iter()
            .any(|error| error.to_string().contains("undefined variable `y`")),
        "expected undefined variable diagnostic, got: {errors:?}"
    );
}

#[test]
fn undefined_type_in_annotation() {
    let source = "fn main() { let x: Foo = 1; }";
    let items = common::compile(source);
    assert!(items.is_err(), "typeck should fail: undefined type");
    let errors = items.err().unwrap();
    assert!(
        errors
            .iter()
            .any(|error| error.to_string().contains("undefined type `Foo`")),
        "expected undefined type diagnostic, got: {errors:?}"
    );
}

#[test]
fn undefined_type_in_struct_construction() {
    let source = "fn main() { let x = Foo { a: 1 }; }";
    let items = common::compile(source);
    assert!(items.is_err(), "typeck should fail: undefined type");
    let errors = items.err().unwrap();
    assert!(
        errors
            .iter()
            .any(|error| error.to_string().contains("undefined type `Foo`")),
        "expected undefined type diagnostic, got: {errors:?}"
    );
}

#[test]
fn undefined_type_in_enum_variant() {
    let source = "fn main() { let x = Foo::Bar(1); }";
    let items = common::compile(source);
    assert!(items.is_err(), "typeck should fail: undefined type");
    let errors = items.err().unwrap();
    assert!(
        errors
            .iter()
            .any(|error| error.to_string().contains("undefined type `Foo`")),
        "expected undefined type diagnostic, got: {errors:?}"
    );
}

#[test]
fn undefined_module_in_value_path() {
    let source = "fn main() { foo::bar(); }";
    let items = common::compile(source);
    assert!(items.is_err(), "typeck should fail: undefined module");
    let errors = items.err().unwrap();
    assert!(
        errors
            .iter()
            .any(|error| error.to_string().contains("undefined module `foo`")),
        "expected undefined module diagnostic, got: {errors:?}"
    );
}

#[test]
fn let_shadow() {
    let source = "fn main() { let x = 1; let x = 2; }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn let_shadow_different_type() {
    let source = "fn main() { let x = 1; let x = true; }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn multi_let_block() {
    let source = "fn main(): int32 { let x = 5; let y = 3; x + y }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn block_trailing_ident() {
    let source = "fn main(): int32 { let x = 1; x }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn block_trailing_binary() {
    let source = "fn main(): int32 { let x = 1; x + 2 }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn block_returns_unit() {
    let source = "fn main(): int32 { let x = 1; }";
    let items = common::compile(source);
    assert!(
        items.is_ok(),
        "typeck skips trailing check for non-Value stmts"
    );
}

#[test]
fn block_expression_returns_unit() {
    let source = "fn main(): int32 { { 42 } }";
    let items = common::compile(source);
    assert!(
        items.is_err(),
        "typeck should fail: block expression returns unit"
    );
}

#[test]
fn block_expression_unit_mismatch() {
    let source = "fn main(): int32 { {} }";
    let items = common::compile(source);
    assert!(
        items.is_err(),
        "typeck should fail: block expression returns unit"
    );
}

#[test]
fn shadow_in_nested_scope() {
    let source = "fn f(): int32 { let x = 1; if true { let x = true; }; x }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn scope_hides_outer() {
    let source = "fn main() { let x = 1; if true { let y = x; }; let z = y; }";
    let items = common::compile(source);
    assert!(items.is_err(), "typeck should fail: y not in scope");
}

#[test]
fn generic_type_param() {
    let source = "fn f(x: Option<int32>) {}";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn string_type() {
    let source = "fn main(): string { let s: string = \"hello\"; s }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn bool_type() {
    let source = "fn main(): bool { let b: bool = true; b }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn unit_literal() {
    let source = "fn main(): unit { unit }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn struct_def_typeck() {
    let source = "struct Point { x: int32, y: int32 } fn main() {}";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn tuple_struct_def_typeck() {
    let source = "tuple Point(int32, float64) fn main() {}";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn enum_def_typeck() {
    let source = "enum Option { None, Some(int32) } fn main() {}";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn tuple_expr_typeck() {
    let source = "fn main(): int32 { let x = (1, 2); 0 }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn field_access_typeck() {
    let source = "fn main(): int32 { let p: int32 = 0; p.x; 0 }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn int_type_alias() {
    let source = "fn main(): int { 42 }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn float_type_alias() {
    let source = "fn main(): float { 3.14 }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}
