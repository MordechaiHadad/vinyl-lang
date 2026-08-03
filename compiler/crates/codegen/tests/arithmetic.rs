mod common;

#[test]
fn arithmetic_add() {
    assert_eq!(common::run("fn main(): int32 { 1 + 2 }").unwrap(), 3);
}

#[test]
fn arithmetic_sub() {
    assert_eq!(common::run("fn main(): int32 { 10 - 3 }").unwrap(), 7);
}

#[test]
fn arithmetic_mul() {
    assert_eq!(common::run("fn main(): int32 { 6 * 7 }").unwrap(), 42);
}

#[test]
fn arithmetic_div() {
    assert_eq!(common::run("fn main(): int32 { 10 / 3 }").unwrap(), 3);
}

#[test]
fn arithmetic_rem() {
    assert_eq!(common::run("fn main(): int32 { 10 % 3 }").unwrap(), 1);
}

#[test]
fn arithmetic_precedence() {
    assert_eq!(common::run("fn main(): int32 { 1 + 2 * 3 }").unwrap(), 7);
}

#[test]
fn arithmetic_parens() {
    assert_eq!(common::run("fn main(): int32 { (1 + 2) * 3 }").unwrap(), 9);
}

#[test]
fn floor_div_positive() {
    assert_eq!(common::run("fn main(): int32 { 7 // 3 }").unwrap(), 2);
}

#[test]
fn floor_div_both_neg() {
    assert_eq!(common::run("fn main(): int32 { (-7) // (-3) }").unwrap(), 2);
}

#[test]
fn floor_div_mixed_signs_returns_positive() {
    let r = common::run("fn main(): int32 { (7 // 3) + ((-7) // (-3)) }").unwrap();
    assert_eq!(r, 4);
}

#[test]
fn power_const() {
    assert_eq!(common::run("fn main(): int32 { 2 ** 3 }").unwrap(), 8);
}

#[test]
fn power_zero() {
    assert_eq!(common::run("fn main(): int32 { 2 ** 0 }").unwrap(), 1);
}

#[test]
fn power_one() {
    assert_eq!(common::run("fn main(): int32 { 2 ** 1 }").unwrap(), 2);
}

#[test]
fn power_ten() {
    assert_eq!(common::run("fn main(): int32 { 2 ** 10 }").unwrap(), 1024);
}

#[test]
fn power_large_exponent() {
    assert_eq!(
        common::run("fn main(): int64 { 2 ** 20 }").unwrap(),
        1048576
    );
}

#[test]
fn power_negative_base() {
    assert_eq!(common::run("fn main(): int64 { (-2) ** 3 }").unwrap(), -8);
}

#[test]
fn power_right_associative() {
    assert_eq!(common::run("fn main(): int32 { 2 ** 3 ** 2 }").unwrap(), 512);
}

#[test]
fn power_dynamic_exponent() {
    assert_eq!(
        common::run("fn main(): int32 { let n: int32 = 3; 2 ** n }").unwrap(),
        8
    );
}

#[test]
fn power_dynamic_base_and_exponent() {
    assert_eq!(
        common::run("fn main(): int32 { let b: int32 = 2; let n: int32 = 4; b ** n }").unwrap(),
        16
    );
}

#[test]
fn power_assign() {
    assert_eq!(
        common::run("fn main(): int32 { let mut x: int32 = 2; x **= 3; x }").unwrap(),
        8
    );
}

#[test]
fn power_assign_float() {
    let bits = common::run("fn main(): float64 { let mut x = 2.0; x **= 3.0; x }").unwrap();
    assert_eq!(bits, 8.0f64.to_bits() as i64);
}

#[test]
fn power_float() {
    let bits = common::run("fn main(): float64 { 2.0 ** 3.0 }").unwrap();
    assert_eq!(bits, 8.0f64.to_bits() as i64);
}

#[test]
fn power_float_negative_exponent() {
    let bits = common::run("fn main(): float64 { 2.0 ** -1.0 }").unwrap();
    assert_eq!(bits, 0.5f64.to_bits() as i64);
}

#[test]
fn power_float_whole_exponent() {
    let bits = common::run("fn main(): float64 { 9.0 ** 2.0 }").unwrap();
    assert_eq!(bits, 81.0f64.to_bits() as i64);
}
