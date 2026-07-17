#[test]
fn typeck_simple_let() {
    let source = "fn main() { let x = 42; }";
    let items = compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn typeck_annotated_let() {
    let source = "fn main() { let x: int32 = 42; }";
    let items = compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn typeck_type_mismatch_annotation() {
    let source = "fn main() { let x: int32 = true; }";
    let items = compile(source);
    assert!(items.is_err(), "typeck should fail: type mismatch");
}

#[test]
fn typeck_binary_op_mismatch() {
    let source = "fn main() { let x = 1 + true; }";
    let items = compile(source);
    assert!(items.is_err(), "typeck should fail: int + bool");
}

#[test]
fn typeck_if_condition_bool() {
    let source = "fn main() { if true { let x = 1; } }";
    let items = compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn typeck_if_condition_not_bool() {
    let source = "fn main() { if 1 { let x = 1; } }";
    let items = compile(source);
    assert!(items.is_err(), "typeck should fail: if condition is int");
}

#[test]
fn typeck_undefined_variable() {
    let source = "fn main() { let x = y; }";
    let items = compile(source);
    assert!(items.is_err(), "typeck should fail: undefined variable");
}

#[test]
fn typeck_function_call_arg_count() {
    let source = "fn add(a: int32, b: int32): int32 { return a + b; } fn main() { add(1); }";
    let items = compile(source);
    assert!(items.is_err(), "typeck should fail: wrong arg count");
}

#[test]
fn typeck_function_call_arg_type() {
    let source = "fn greet(name: string) {} fn main() { greet(42); }";
    let items = compile(source);
    assert!(items.is_err(), "typeck should fail: wrong arg type");
}

#[test]
fn typeck_return_type_match() {
    let source = "fn main() { let x = 42; return x; }";
    let items = compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

fn compile(
    source: &str,
) -> Result<Vec<vinyl_typecheck::hir::HirItem>, Vec<vinyl_typecheck::TypeError>> {
    let tree = vinyl_parser::parse(source).unwrap();
    let items = vinyl_parser::lower::lower(&tree, source, "<test>").unwrap();
    vinyl_typecheck::typeck(&items, source, "<test>")
}
