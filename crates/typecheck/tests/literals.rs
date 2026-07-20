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
    assert!(items.is_err(), "typeck should fail: int literal cannot be bool");
}

#[test]
fn int_literal_rejects_string() {
    let source = "fn main(): string { 42 }";
    let items = common::compile(source);
    assert!(items.is_err(), "typeck should fail: int literal cannot be string");
}

#[test]
fn int_literal_rejects_char() {
    let source = "fn main(): char { 42 }";
    let items = common::compile(source);
    assert!(items.is_err(), "typeck should fail: int literal cannot be char");
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
    assert!(items.is_err(), "typeck should fail: float literal cannot be int32");
}

#[test]
fn float_literal_rejects_uint32() {
    let source = "fn main(): uint32 { 3.14 }";
    let items = common::compile(source);
    assert!(items.is_err(), "typeck should fail: float literal cannot be uint32");
}

#[test]
fn float_literal_rejects_bool() {
    let source = "fn main(): bool { 3.14 }";
    let items = common::compile(source);
    assert!(items.is_err(), "typeck should fail: float literal cannot be bool");
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
    assert!(items.is_err(), "typeck should fail: char return type mismatch");
}

#[test]
fn char_annotation_mismatch() {
    let source = "fn main() { let c: int32 = 'a'; }";
    let items = common::compile(source);
    assert!(items.is_err(), "typeck should fail: char annotation mismatch");
}
