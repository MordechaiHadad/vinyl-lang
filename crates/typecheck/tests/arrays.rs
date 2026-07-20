mod common;

#[test]
fn array_literal() {
    let source = "fn main(): int32 { let arr = [1, 2, 3]; arr[0] }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn array_literal_uniform_types() {
    let source = "fn main(): int32 { let arr = [1, 2, 3]; arr[1] }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn array_typed_annotation_match() {
    let source = "fn main(): int32 { let arr: [int32; 3] = [1, 2, 3]; arr[0] }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn array_typed_size_mismatch() {
    let source = "fn main() { let arr: [int32; 3] = [1, 2]; }";
    let items = common::compile(source);
    assert!(items.is_err(), "typeck should fail: array size mismatch");
}

#[test]
fn array_index_int32() {
    let source = "fn main(): int32 { let arr = [10, 20]; arr[0] }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn array_index_non_int32() {
    let source = "fn main() { let arr = [1, 2]; arr[true]; }";
    let items = common::compile(source);
    assert!(items.is_err(), "typeck should fail: index must be int32");
}

#[test]
fn array_element_type_mismatch() {
    let source = "fn main() { let arr = [1, true]; }";
    let items = common::compile(source);
    assert!(items.is_err(), "typeck should fail: array element type mismatch");
}

#[test]
fn array_index_non_array() {
    let source = "fn main() { let x = 42; x[0]; }";
    let items = common::compile(source);
    assert!(items.is_err(), "typeck should fail: cannot index non-array");
}

#[test]
fn array_of_arrays() {
    let source = "fn main(): int32 { let arr = [[1, 2], [3, 4]]; arr[0][1] }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn array_nested_type_mismatch() {
    let source = "fn main() { let arr = [[1, 2], [true, false]]; }";
    let items = common::compile(source);
    assert!(items.is_err(), "typeck should fail: nested array type mismatch");
}

#[test]
fn char_in_array() {
    let source = "fn main(): char { let arr = ['a', 'b', 'c']; arr[1] }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn char_array_type_mismatch() {
    let source = "fn main() { let arr: [int32; 2] = ['a', 'b']; }";
    let items = common::compile(source);
    assert!(items.is_err(), "typeck should fail: char array type mismatch");
}
