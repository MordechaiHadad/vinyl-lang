mod common;

#[test]
fn int_literal_infers_int32_from_annotation() {
    let source = "fn main(): int32 { 42 }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn int_literal_infers_uint32() {
    let source = "fn main(): uint32 { 42 }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn int_literal_infers_isize() {
    let source = "fn main(): isize { 42 }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn int_literal_infers_usize() {
    let source = "fn main(): usize { 42 }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn int_literal_infers_float32() {
    let source = "fn main(): float32 { 42 }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn int_literal_infers_float64() {
    let source = "fn main(): float64 { 42 }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn int_literal_rejects_bool() {
    let source = "fn main(): bool { 42 }";
    let items = common::compile(source);
    assert!(
        items.is_err(),
        "typeck should fail: int literal cannot be bool"
    );
}

#[test]
fn int_literal_rejects_string() {
    let source = "fn main(): string { 42 }";
    let items = common::compile(source);
    assert!(
        items.is_err(),
        "typeck should fail: int literal cannot be string"
    );
}

#[test]
fn int_literal_rejects_char() {
    let source = "fn main(): char { 42 }";
    let items = common::compile(source);
    assert!(
        items.is_err(),
        "typeck should fail: int literal cannot be char"
    );
}

#[test]
fn float_literal_infers_float64() {
    let source = "fn main(): float64 { 3.14 }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn float_literal_infers_float32() {
    let source = "fn main(): float32 { 3.14 }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn float_literal_rejects_int32() {
    let source = "fn main(): int32 { 3.14 }";
    let items = common::compile(source);
    assert!(
        items.is_err(),
        "typeck should fail: float literal cannot be int32"
    );
}

#[test]
fn float_literal_rejects_uint32() {
    let source = "fn main(): uint32 { 3.14 }";
    let items = common::compile(source);
    assert!(
        items.is_err(),
        "typeck should fail: float literal cannot be uint32"
    );
}

#[test]
fn float_literal_rejects_bool() {
    let source = "fn main(): bool { 3.14 }";
    let items = common::compile(source);
    assert!(
        items.is_err(),
        "typeck should fail: float literal cannot be bool"
    );
}

#[test]
fn int32_literal_at_max() {
    let source = "fn main(): int32 { 2147483647 }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn int32_literal_out_of_range_positive() {
    let source = "fn main(): int32 { 2147483648 }";
    let items = common::compile(source);
    assert!(items.is_err(), "typeck should fail: literal overflows int32");
}

#[test]
fn int32_literal_out_of_range_negative() {
    let source = "fn main(): int32 { -2147483649 }";
    let items = common::compile(source);
    assert!(items.is_err(), "typeck should fail: literal underflows int32");
}

#[test]
fn int64_literal_in_range() {
    let source = "fn main(): int64 { 9223372036854775807 }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn int8_literal_out_of_range() {
    let source = "fn main(): int8 { 300 }";
    let items = common::compile(source);
    assert!(items.is_err(), "typeck should fail: literal overflows int8");
}

#[test]
fn uint32_literal_in_range() {
    let source = "fn main(): uint32 { 4294967295 }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn uint32_literal_out_of_range() {
    let source = "fn main(): uint32 { 4294967296 }";
    let items = common::compile(source);
    assert!(items.is_err(), "typeck should fail: literal overflows uint32");
}

#[test]
fn uint32_literal_rejects_negative() {
    let source = "fn main(): uint32 { -5 }";
    let items = common::compile(source);
    assert!(items.is_err(), "typeck should fail: negative literal into uint32");
}

#[test]
fn untyped_int_literal_defaults_int64() {
    let source = "fn main() { let x = 9223372036854775807; }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn float32_literal_out_of_range() {
    let source = "fn main(): float32 { 400000000000000000000000000000000000000.0 }";
    let items = common::compile(source);
    assert!(items.is_err(), "typeck should fail: literal overflows float32");
}

#[test]
fn float64_literal_non_finite() {
    let source = format!("fn main(): float64 {{ {}.0 }}", "1".repeat(400));
    let items = common::compile(&source);
    assert!(items.is_err(), "typeck should fail: non-finite float literal");
}

#[test]
fn char_literal() {
    let source = "fn main(): char { 'a' }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn char_let() {
    let source = "fn main(): char { let c: char = 'z'; c }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn char_let_infer() {
    let source = "fn main(): char { let c = 'x'; c }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn char_return_type_mismatch() {
    let source = "fn main(): int32 { 'a' }";
    let items = common::compile(source);
    assert!(
        items.is_err(),
        "typeck should fail: char return type mismatch"
    );
}

#[test]
fn char_annotation_mismatch() {
    let source = "fn main() { let c: int32 = 'a'; }";
    let items = common::compile(source);
    assert!(
        items.is_err(),
        "typeck should fail: char annotation mismatch"
    );
}
