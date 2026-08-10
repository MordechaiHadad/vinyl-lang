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

#[test]
fn array_fill_small_clean() {
    let source = "fn main(): int32 { let arr = [0; 10]; arr[0] }";
    let (items, warnings) = common::compile_with_warnings(source);
    assert!(items.is_ok(), "small fill should compile: {:?}", items.err());
    assert_eq!(warnings.len(), 0, "small array should not warn");
}

#[test]
fn array_fill_below_warning_boundary_clean() {
    let source = "fn main(): int32 { let arr = [0; 4095]; arr[0] }";
    let (items, warnings) = common::compile_with_warnings(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
    assert_eq!(warnings.len(), 0, "just under 32 KiB should not warn");
}

#[test]
fn array_fill_large_warns() {
    let source = "fn main(): int32 { let arr = [0; 4096]; arr[0] }";
    let (items, warnings) = common::compile_with_warnings(source);
    assert!(items.is_ok(), "32 KiB fill should compile: {:?}", items.err());
    assert_eq!(warnings.len(), 1, "exactly 32 KiB should warn");
    assert!(
        warnings[0].to_string().contains("large"),
        "warning should mention the large array"
    );
}

#[test]
fn array_fill_just_below_error_boundary_warns() {
    let source = "fn main(): int32 { let arr = [0; 131071]; arr[0] }";
    let (items, warnings) = common::compile_with_warnings(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
    assert_eq!(warnings.len(), 1, "just under 1 MiB should warn");
}

#[test]
fn array_fill_error_boundary_errors() {
    let source = "fn main(): int32 { let arr = [0; 131072]; arr[0] }";
    let items = common::compile(source);
    assert!(
        items.is_err(),
        "exactly 1 MiB should fail with array_too_large"
    );
}

#[test]
fn array_fill_huge_errors() {
    let source = "fn main(): int32 { let arr = [0; 10000000]; arr[0] }";
    let items = common::compile(source);
    assert!(items.is_err(), "10M-element fill should fail");
}

#[test]
fn allow_large_array_suppresses_error_and_warning() {
    let source =
        "@allow(large_array) fn main(): int32 { let arr = [0; 10000000]; arr[0] }";
    let (items, warnings) = common::compile_with_warnings(source);
    assert!(items.is_ok(), "suppressed fill should compile: {:?}", items.err());
    assert_eq!(warnings.len(), 0, "suppressed fill should not warn");
}

#[test]
fn array_fill_named_element_not_checked() {
    let source = "struct Point { x: int32, y: int32 }
fn main(): int32 { let arr = [Point { x: 1, y: 2 }; 1000000]; arr[0].x }";
    let (items, warnings) = common::compile_with_warnings(source);
    // todo(sized): no `Sized` bound yet, named element sizes are not checked
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
    assert_eq!(warnings.len(), 0);
}
