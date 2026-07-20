mod common;

#[test]
fn function_call_arg_count() {
    let source = "fn add(a: int32, b: int32): int32 { return a + b; } fn main() { add(1); }";
    let items = common::compile(source);
    assert!(items.is_err(), "typeck should fail: wrong arg count");
}

#[test]
fn function_call_arg_type() {
    let source = "fn greet(name: string) {} fn main() { greet(42); }";
    let items = common::compile(source);
    assert!(items.is_err(), "typeck should fail: wrong arg type");
}

#[test]
fn function_call_correct_args() {
    let source = "fn add(a: int32, b: int32): int32 { a + b } fn main(): int32 { add(3, 4) }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn function_call_multi_arg_types() {
    let source = "fn greet(greeting: string, n: int32): string { greeting } fn main(): string { greet(\"hi\", 5) }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn function_call_wrong_first_arg_type() {
    let source = "fn greet(greeting: string, n: int32): string { greeting } fn main(): string { greet(5, 5) }";
    let items = common::compile(source);
    assert!(items.is_err(), "typeck should fail: wrong first arg type");
}

#[test]
fn function_call_wrong_second_arg_type() {
    let source = "fn greet(greeting: string, n: int32): string { greeting } fn main(): string { greet(\"hi\", true) }";
    let items = common::compile(source);
    assert!(items.is_err(), "typeck should fail: wrong second arg type");
}

#[test]
fn function_chain() {
    let source = "fn add(a: int32, b: int32): int32 { a + b } fn triple(n: int32): int32 { n * 3 } fn main(): int32 { triple(add(2, 3)) }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn function_identity() {
    let source = "fn id(x: int32): int32 { x } fn main(): int32 { id(99) }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn function_no_args() {
    let source = "fn five(): int32 { 5 } fn main(): int32 { five() }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn explicit_return_type_match() {
    let source = "fn main(): int32 { 42 }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn explicit_return_type_mismatch() {
    let source = "fn main(): int32 { true }";
    let items = common::compile(source);
    assert!(items.is_err(), "typeck should fail: return type mismatch");
}

#[test]
fn explicit_return_type_unit() {
    let source = "fn main(): unit {}";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn return_type_match() {
    let source = "fn main() { let x = 42; return x; }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn return_in_if_branches() {
    let source = "fn f(): int32 { if true { return 1; } else { return 2; } }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn return_in_if_then_fallthrough() {
    let source = "fn f(): int32 { if true { return 1; } 2 }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn return_in_else_fallthrough() {
    let source = "fn f(): int32 { if true { 1 } else { return 2; } }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn return_void_in_typed_fn() {
    let source = "fn f(): int32 { return; }";
    let items = common::compile(source);
    assert!(items.is_err(), "typeck should fail: returning unit in int32 fn");
}

#[test]
fn undefined_function_call() {
    let source = "fn main() { undefined(1); }";
    let items = common::compile(source);
    assert!(items.is_err(), "typeck should fail: undefined function");
}

#[test]
fn mutual_function_calls() {
    let source = "fn a(): int32 { b() } fn b(): int32 { 1 } fn main(): int32 { a() }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn function_call_in_binary() {
    let source = "fn add(a: int32, b: int32): int32 { a + b } fn main(): int32 { add(1, 2) + 3 }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}
