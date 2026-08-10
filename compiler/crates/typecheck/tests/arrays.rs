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
    assert!(
        items.is_err(),
        "typeck should fail: index must be an integer"
    );
}

#[test]
fn array_index_int32_var() {
    let source = "fn f(i: int32): int32 { let arr = [1, 2]; arr[i] }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn array_index_int_var() {
    let source = "fn f(i: int): int { let arr = [1, 2]; arr[i] }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn array_index_float() {
    let source = "fn main() { let arr = [1, 2]; arr[1.5]; }";
    let items = common::compile(source);
    assert!(
        items.is_err(),
        "typeck should fail: index must be an integer"
    );
}

#[test]
fn array_index_string() {
    let source = "fn main() { let arr = [1, 2]; arr[\"hello\"]; }";
    let items = common::compile(source);
    assert!(
        items.is_err(),
        "typeck should fail: index must be an integer"
    );
}

#[test]
fn array_element_type_mismatch() {
    let source = "fn main() { let arr = [1, true]; }";
    let items = common::compile(source);
    assert!(
        items.is_err(),
        "typeck should fail: array element type mismatch"
    );
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
    assert!(
        items.is_err(),
        "typeck should fail: nested array type mismatch"
    );
}

#[test]
fn array_fill_literal() {
    let source = "fn main(): int32 { let arr = [7; 3]; arr[2] }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn array_fill_size_mismatch() {
    let source = "fn main() { let arr: [int32; 2] = [7; 3]; }";
    let items = common::compile(source);
    assert!(items.is_err(), "typeck should fail: array size mismatch");
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
    assert!(
        items.is_err(),
        "typeck should fail: char array type mismatch"
    );
}

#[test]
fn char_array_return_int() {
    let source = "fn main(): int { let arr = ['a', 'b']; arr[0] }";
    let items = common::compile(source);
    assert!(
        items.is_err(),
        "typeck should fail: recieved char from array, expected int"
    );
}

#[test]
fn bool_array_return_int() {
    let source = "fn main(): int { let arr = [true, false]; arr[0] }";
    let items = common::compile(source);
    assert!(
        items.is_err(),
        "typeck should fail: recieved bool from array, expected int"
    );
}
