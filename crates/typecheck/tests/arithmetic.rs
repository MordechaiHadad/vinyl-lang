mod common;

#[test]
fn add() {
    let source = "fn main(): int32 { 1 + 2 }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn sub() {
    let source = "fn main(): int32 { 10 - 3 }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn mul() {
    let source = "fn main(): int32 { 6 * 7 }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn div() {
    let source = "fn main(): int32 { 10 / 3 }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn rem() {
    let source = "fn main(): int32 { 10 % 3 }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn precedence() {
    let source = "fn main(): int32 { 1 + 2 * 3 }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn parens() {
    let source = "fn main(): int32 { (1 + 2) * 3 }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn mixed_int_types() {
    let source = "fn main(): int64 { 10 + 20 }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn binary_op_mismatch() {
    let source = "fn main() { let x = 1 + true; }";
    let items = common::compile(source);
    assert!(items.is_err(), "typeck should fail: int + bool");
}

#[test]
fn power() {
    let source = "fn main(): int32 { 2 ** 3 }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn floor_div() {
    let source = "fn main(): int32 { 10 // 3 }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn bitwise_and() {
    let source = "fn main(): int32 { 1 & 3 }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn bitwise_or() {
    let source = "fn main(): int32 { 1 | 2 }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn bitwise_xor() {
    let source = "fn main(): int32 { 1 ^ 3 }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn shift_left() {
    let source = "fn main(): int32 { 1 << 2 }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn shift_right() {
    let source = "fn main(): int32 { 8 >> 1 }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn range() {
    let source = "fn main() { let r = 1..10; }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn range_inclusive() {
    let source = "fn main() { let r = 1..=10; }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn bitwise_type_mismatch() {
    let source = "fn main() { let x = 1 & true; }";
    let items = common::compile(source);
    assert!(items.is_err(), "typeck should fail: int & bool");
}

#[test]
fn shift_type_mismatch() {
    let source = "fn main() { let x = 1 << true; }";
    let items = common::compile(source);
    assert!(items.is_err(), "typeck should fail: int << bool");
}

#[test]
fn comparison_eq() {
    let source = "fn main(): int32 { if 3 == 3 { 1 } else { 0 } }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn comparison_ne() {
    let source = "fn main(): int32 { if 3 != 4 { 1 } else { 0 } }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn comparison_lt() {
    let source = "fn main(): int32 { if 2 < 3 { 1 } else { 0 } }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn comparison_gt() {
    let source = "fn main(): int32 { if 5 > 3 { 1 } else { 0 } }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn comparison_le() {
    let source = "fn main(): int32 { if 3 <= 3 { 1 } else { 0 } }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn comparison_ge() {
    let source = "fn main(): int32 { if 3 >= 3 { 1 } else { 0 } }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn comparison_returns_bool() {
    let source = "fn f(): bool { 1 == 2 }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn comparison_result_not_bool() {
    let source = "fn f(): int32 { 1 == 2 }";
    let items = common::compile(source);
    assert!(items.is_err(), "typeck should fail: comparison returns bool, not int32");
}

#[test]
fn logical_and() {
    let source = "fn main(): int32 { if true && true { 1 } else { 0 } }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn logical_or() {
    let source = "fn main(): int32 { if false || true { 1 } else { 0 } }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn logical_and_returns_bool() {
    let source = "fn f(): bool { true && false }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn logical_or_returns_bool() {
    let source = "fn f(): bool { true || false }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn logical_result_not_bool() {
    let source = "fn f(): int32 { true && false }";
    let items = common::compile(source);
    assert!(items.is_err(), "typeck should fail: logical returns bool, not int32");
}

#[test]
fn mixed_comparison_types() {
    let source = "fn f(): bool { 1.5 == 1 }";
    let items = common::compile(source);
    assert!(items.is_err(), "typeck should fail: float == int");
}

#[test]
fn string_equality() {
    let source = "fn f(): bool { \"a\" == \"b\" }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn binary_op_string_plus_int() {
    let source = "fn main() { let x = \"hello\" + 1; }";
    let items = common::compile(source);
    assert!(items.is_err(), "typeck should fail: string + int");
}

#[test]
fn binary_op_bool_plus_int() {
    let source = "fn main() { let x = true + 1; }";
    let items = common::compile(source);
    assert!(items.is_err(), "typeck should fail: bool + int");
}

#[test]
fn binary_op_string_eq_int() {
    let source = "fn main() { let x = \"hello\" == 1; }";
    let items = common::compile(source);
    assert!(items.is_err(), "typeck should fail: string == int");
}
