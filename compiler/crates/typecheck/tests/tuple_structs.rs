mod common;

#[test]
fn tuple_struct_construction() {
    let source = "tuple Pair(int32, int32) fn main(): int32 { let p = Pair(1, 2); p.0 + p.1 }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn tuple_struct_construction_nested_types() {
    let source = "tuple Pair(int32, string) fn main(): int32 { let p = Pair(1, \"a\"); p.0 }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn tuple_struct_construction_arg_type_mismatch() {
    let source = "tuple Pair(int32, int32) fn main(): int32 { let p = Pair(1, \"a\"); p.0 }";
    let items = common::compile(source);
    assert!(items.is_err(), "typeck should fail: arg type mismatch");
}

#[test]
fn tuple_struct_construction_arg_count_mismatch() {
    let source = "tuple Pair(int32, int32) fn main(): int32 { let p = Pair(1, 2, 3); p.0 }";
    let items = common::compile(source);
    assert!(items.is_err(), "typeck should fail: arg count mismatch");
}

#[test]
fn tuple_struct_function_shadowing_still_works() {
    let source =
        "tuple Foo(int32) fn foo(x: int32): int32 { x } fn main(): int32 { let f = foo(7); f }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}
