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

#[test]
fn typeck_arithmetic_add() {
    let source = "fn main(): int32 { 1 + 2 }";
    let items = compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn typeck_arithmetic_sub() {
    let source = "fn main(): int32 { 10 - 3 }";
    let items = compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn typeck_arithmetic_mul() {
    let source = "fn main(): int32 { 6 * 7 }";
    let items = compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn typeck_arithmetic_div() {
    let source = "fn main(): int32 { 10 / 3 }";
    let items = compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn typeck_arithmetic_rem() {
    let source = "fn main(): int32 { 10 % 3 }";
    let items = compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn typeck_arithmetic_precedence() {
    let source = "fn main(): int32 { 1 + 2 * 3 }";
    let items = compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn typeck_arithmetic_parens() {
    let source = "fn main(): int32 { (1 + 2) * 3 }";
    let items = compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn typeck_arithmetic_mixed_int_types() {
    let source = "fn main(): int64 { 10 + 20 }";
    let items = compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn typeck_int_literal_infers_int32_from_annotation() {
    let source = "fn main(): int32 { 42 }";
    let items = compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn typeck_int_literal_infers_uint32() {
    let source = "fn main(): uint32 { 42 }";
    let items = compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn typeck_int_literal_infers_isize() {
    let source = "fn main(): isize { 42 }";
    let items = compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn typeck_int_literal_infers_usize() {
    let source = "fn main(): usize { 42 }";
    let items = compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn typeck_int_literal_infers_float32() {
    let source = "fn main(): float32 { 42 }";
    let items = compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn typeck_int_literal_infers_float64() {
    let source = "fn main(): float64 { 42 }";
    let items = compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn typeck_int_literal_rejects_bool() {
    let source = "fn main(): bool { 42 }";
    let items = compile(source);
    assert!(items.is_err(), "typeck should fail: int literal cannot be bool");
}

#[test]
fn typeck_int_literal_rejects_string() {
    let source = "fn main(): string { 42 }";
    let items = compile(source);
    assert!(items.is_err(), "typeck should fail: int literal cannot be string");
}

#[test]
fn typeck_int_literal_rejects_char() {
    let source = "fn main(): char { 42 }";
    let items = compile(source);
    assert!(items.is_err(), "typeck should fail: int literal cannot be char");
}

#[test]
fn typeck_float_literal_infers_float64() {
    let source = "fn main(): float64 { 3.14 }";
    let items = compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn typeck_float_literal_infers_float32() {
    let source = "fn main(): float32 { 3.14 }";
    let items = compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn typeck_float_literal_rejects_int32() {
    let source = "fn main(): int32 { 3.14 }";
    let items = compile(source);
    assert!(items.is_err(), "typeck should fail: float literal cannot be int32");
}

#[test]
fn typeck_float_literal_rejects_uint32() {
    let source = "fn main(): uint32 { 3.14 }";
    let items = compile(source);
    assert!(items.is_err(), "typeck should fail: float literal cannot be uint32");
}

#[test]
fn typeck_float_literal_rejects_bool() {
    let source = "fn main(): bool { 3.14 }";
    let items = compile(source);
    assert!(items.is_err(), "typeck should fail: float literal cannot be bool");
}

#[test]
fn typeck_comparison_eq() {
    let source = "fn main(): int32 { if 3 == 3 { 1 } else { 0 } }";
    let items = compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn typeck_comparison_ne() {
    let source = "fn main(): int32 { if 3 != 4 { 1 } else { 0 } }";
    let items = compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn typeck_comparison_lt() {
    let source = "fn main(): int32 { if 2 < 3 { 1 } else { 0 } }";
    let items = compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn typeck_comparison_gt() {
    let source = "fn main(): int32 { if 5 > 3 { 1 } else { 0 } }";
    let items = compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn typeck_comparison_le() {
    let source = "fn main(): int32 { if 3 <= 3 { 1 } else { 0 } }";
    let items = compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn typeck_comparison_ge() {
    let source = "fn main(): int32 { if 3 >= 3 { 1 } else { 0 } }";
    let items = compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn typeck_logical_and() {
    let source = "fn main(): int32 { if true && true { 1 } else { 0 } }";
    let items = compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn typeck_logical_or() {
    let source = "fn main(): int32 { if false || true { 1 } else { 0 } }";
    let items = compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn typeck_if_else() {
    let source = "fn main(): int32 { if true { 1 } else { 2 } }";
    let items = compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn typeck_nested_if() {
    let source = "fn main(): int32 { if true { if false { 0 } else { 1 } } else { 2 } }";
    let items = compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn typeck_if_condition_is_comparison() {
    let source = "fn main(): int32 { if 3 < 5 { 1 } else { 0 } }";
    let items = compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn typeck_if_condition_is_logical() {
    let source = "fn main(): int32 { if true || false { 1 } else { 0 } }";
    let items = compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn typeck_if_condition_string_rejected() {
    let source = "fn main() { if \"hello\" { 1 } }";
    let items = compile(source);
    assert!(items.is_err(), "typeck should fail: if condition is string");
}

#[test]
fn typeck_function_call_correct_args() {
    let source = "fn add(a: int32, b: int32): int32 { a + b } fn main(): int32 { add(3, 4) }";
    let items = compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn typeck_function_call_multi_arg_types() {
    let source = "fn greet(greeting: string, n: int32): string { greeting } fn main(): string { greet(\"hi\", 5) }";
    let items = compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn typeck_function_call_wrong_first_arg_type() {
    let source = "fn greet(greeting: string, n: int32): string { greeting } fn main(): string { greet(5, 5) }";
    let items = compile(source);
    assert!(items.is_err(), "typeck should fail: wrong first arg type");
}

#[test]
fn typeck_function_call_wrong_second_arg_type() {
    let source = "fn greet(greeting: string, n: int32): string { greeting } fn main(): string { greet(\"hi\", true) }";
    let items = compile(source);
    assert!(items.is_err(), "typeck should fail: wrong second arg type");
}

#[test]
fn typeck_function_chain() {
    let source = "fn add(a: int32, b: int32): int32 { a + b } fn triple(n: int32): int32 { n * 3 } fn main(): int32 { triple(add(2, 3)) }";
    let items = compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn typeck_function_identity() {
    let source = "fn id(x: int32): int32 { x } fn main(): int32 { id(99) }";
    let items = compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn typeck_function_no_args() {
    let source = "fn five(): int32 { 5 } fn main(): int32 { five() }";
    let items = compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn typeck_explicit_return_type_match() {
    let source = "fn main(): int32 { 42 }";
    let items = compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn typeck_explicit_return_type_mismatch() {
    let source = "fn main(): int32 { true }";
    let items = compile(source);
    assert!(items.is_err(), "typeck should fail: return type mismatch");
}

#[test]
fn typeck_explicit_return_type_unit() {
    let source = "fn main(): unit {}";
    let items = compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn typeck_let_shadow() {
    let source = "fn main() { let x = 1; let x = 2; }";
    let items = compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn typeck_let_shadow_different_type() {
    let source = "fn main() { let x = 1; let x = true; }";
    let items = compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn typeck_binary_op_string_plus_int() {
    let source = "fn main() { let x = \"hello\" + 1; }";
    let items = compile(source);
    assert!(items.is_err(), "typeck should fail: string + int");
}

#[test]
fn typeck_binary_op_bool_plus_int() {
    let source = "fn main() { let x = true + 1; }";
    let items = compile(source);
    assert!(items.is_err(), "typeck should fail: bool + int");
}

#[test]
fn typeck_binary_op_string_eq_int() {
    let source = "fn main() { let x = \"hello\" == 1; }";
    let items = compile(source);
    assert!(items.is_err(), "typeck should fail: string == int");
}

#[test]
fn typeck_multi_let_block() {
    let source = "fn main(): int32 { let x = 5; let y = 3; x + y }";
    let items = compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn typeck_string_type() {
    let source = "fn main(): string { let s: string = \"hello\"; s }";
    let items = compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn typeck_bool_type() {
    let source = "fn main(): bool { let b: bool = true; b }";
    let items = compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn typeck_char_literal() {
    let source = "fn main(): char { 'a' }";
    let items = compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn typeck_char_let() {
    let source = "fn main(): char { let c: char = 'z'; c }";
    let items = compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn typeck_char_let_infer() {
    let source = "fn main(): char { let c = 'x'; c }";
    let items = compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn typeck_char_return_type_mismatch() {
    let source = "fn main(): int32 { 'a' }";
    let items = compile(source);
    assert!(items.is_err(), "typeck should fail: char return type mismatch");
}

#[test]
fn typeck_char_annotation_mismatch() {
    let source = "fn main() { let c: int32 = 'a'; }";
    let items = compile(source);
    assert!(items.is_err(), "typeck should fail: char annotation mismatch");
}

#[test]
fn typeck_char_in_array() {
    let source = "fn main(): char { let arr = ['a', 'b', 'c']; arr[1] }";
    let items = compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn typeck_char_array_type_mismatch() {
    let source = "fn main() { let arr: [int32; 2] = ['a', 'b']; }";
    let items = compile(source);
    assert!(items.is_err(), "typeck should fail: char array type mismatch");
}

#[test]
fn typeck_array_literal() {
    let source = "fn main(): int32 { let arr = [1, 2, 3]; arr[0] }";
    let items = compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn typeck_array_literal_uniform_types() {
    let source = "fn main(): int32 { let arr = [1, 2, 3]; arr[1] }";
    let items = compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn typeck_array_typed_annotation_match() {
    let source = "fn main(): int32 { let arr: [int32; 3] = [1, 2, 3]; arr[0] }";
    let items = compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn typeck_array_typed_size_mismatch() {
    let source = "fn main() { let arr: [int32; 3] = [1, 2]; }";
    let items = compile(source);
    assert!(items.is_err(), "typeck should fail: array size mismatch");
}

#[test]
fn typeck_array_index_int32() {
    let source = "fn main(): int32 { let arr = [10, 20]; arr[0] }";
    let items = compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn typeck_array_index_non_int32() {
    let source = "fn main() { let arr = [1, 2]; arr[true]; }";
    let items = compile(source);
    assert!(items.is_err(), "typeck should fail: index must be int32");
}

#[test]
fn typeck_array_element_type_mismatch() {
    let source = "fn main() { let arr = [1, true]; }";
    let items = compile(source);
    assert!(items.is_err(), "typeck should fail: array element type mismatch");
}

#[test]
fn typeck_array_index_non_array() {
    let source = "fn main() { let x = 42; x[0]; }";
    let items = compile(source);
    assert!(items.is_err(), "typeck should fail: cannot index non-array");
}

#[test]
fn typeck_array_of_arrays() {
    let source = "fn main(): int32 { let arr = [[1, 2], [3, 4]]; arr[0][1] }";
    let items = compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn typeck_array_nested_type_mismatch() {
    let source = "fn main() { let arr = [[1, 2], [true, false]]; }";
    let items = compile(source);
    assert!(items.is_err(), "typeck should fail: nested array type mismatch");
}

fn compile(
    source: &str,
) -> Result<Vec<vinyl_typecheck::hir::HirItem>, Vec<vinyl_typecheck::TypeError>> {
    let tree = vinyl_parser::parse(source).unwrap();
    let items = vinyl_parser::lower::lower(&tree, source, "<test>").unwrap();
    vinyl_typecheck::typeck(&items, source, "<test>")
}
