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
    assert_eq!(
        common::run("fn pow_check(): int32 { let mut x = 2.0; x **= 3.0; if x == 8.0 { 1 } else { 0 } } fn main(): int32 { pow_check() }")
            .unwrap(),
        1
    );
}

#[test]
fn power_float() {
    assert_eq!(
        common::run("fn pow_check(): int32 { if 2.0 ** 3.0 == 8.0 { 1 } else { 0 } } fn main(): int32 { pow_check() }")
            .unwrap(),
        1
    );
}

#[test]
fn power_float_negative_exponent() {
    assert_eq!(
        common::run("fn pow_check(): int32 { if 2.0 ** -1.0 == 0.5 { 1 } else { 0 } } fn main(): int32 { pow_check() }")
            .unwrap(),
        1
    );
}

#[test]
fn power_float_whole_exponent() {
    assert_eq!(
        common::run("fn pow_check(): int32 { if 9.0 ** 2.0 == 81.0 { 1 } else { 0 } } fn main(): int32 { pow_check() }")
            .unwrap(),
        1
    );
}

#[test]
fn float32_arithmetic() {
    assert_eq!(
        common::run("fn check(): int32 { let a: float32 = 1.5; let b: float32 = 2.5; if (a + b == 4.0) && (b - a == 1.0) && (a * b == 3.75) && (b / a > 1.6) { 1 } else { 0 } } fn main(): int32 { check() }")
            .unwrap(),
        1
    );
}

#[test]
fn float32_pow() {
    assert_eq!(
        common::run("fn check(): int32 { let x: float32 = 2.0; if x ** 3.0 == 8.0 && 2.0 ** -1.0 == 0.5 { 1 } else { 0 } } fn main(): int32 { check() }")
            .unwrap(),
        1
    );
}

#[test]
fn float32_comparison() {
    assert_eq!(
        common::run("fn check(): int32 { if 1.5 < 2.5 && 3.0 >= 3.0 && 1.0 != 2.0 { 1 } else { 0 } } fn main(): int32 { check() }")
            .unwrap(),
        1
    );
}

#[test]
fn float32_int_literal_coerces() {
    assert_eq!(
        common::run("fn check(): int32 { let x: float32 = 5; if x == 5.0 { 1 } else { 0 } } fn main(): int32 { check() }")
            .unwrap(),
        1
    );
}

#[test]
fn float32_function_boundary() {
    assert_eq!(
        common::run("fn half(x: float32): float32 { x * 2.0 } fn check(): int32 { if half(21.0) == 42.0 { 1 } else { 0 } } fn main(): int32 { check() }")
            .unwrap(),
        1
    );
}

#[test]
fn float32_struct_fields() {
    assert_eq!(
        common::run("struct P { a: float32, b: float32 } fn check(): int32 { let p = P { a: 1.5, b: 2.5 }; if p.a + p.b == 4.0 { 1 } else { 0 } } fn main(): int32 { check() }")
            .unwrap(),
        1
    );
}

#[test]
fn float32_array() {
    assert_eq!(
        common::run("fn check(): int32 { let a: [float32; 2] = [1.5, 2.5]; if a[0] + a[1] == 4.0 { 1 } else { 0 } } fn main(): int32 { check() }")
            .unwrap(),
        1
    );
}

#[test]
fn float32_mutable_var_in_loop() {
    assert_eq!(
        common::run("fn check(): int32 { let mut acc: float32 = 0.0; let mut i = 0; while i < 3 { acc = acc + 1.0; i = i + 1; } if acc == 3.0 { 1 } else { 0 } } fn main(): int32 { check() }")
            .unwrap(),
        1
    );
}

#[test]
fn float32_if_expression_result() {
    assert_eq!(
        common::run("fn check(): int32 { let c = true; let x: float32 = if c { 1.5 } else { 2.5 }; if x == 1.5 { 1 } else { 0 } } fn main(): int32 { check() }")
            .unwrap(),
        1
    );
}

#[test]
fn int128_arithmetic() {
    assert_eq!(
        common::run("fn check(): int32 { let a: int128 = 170141183460469231731687303715884105727; if a - 1 == 170141183460469231731687303715884105726 && a + 1 == -170141183460469231731687303715884105728 { 1 } else { 0 } } fn main(): int32 { check() }")
            .unwrap(),
        1
    );
}

#[test]
fn int128_multiplication() {
    assert_eq!(
        common::run("fn check(): int32 { let a: int128 = 1000000000000000000; if a * a == 1000000000000000000000000000000000000 { 1 } else { 0 } } fn main(): int32 { check() }")
            .unwrap(),
        1
    );
}

#[test]
fn int128_division_and_remainder() {
    assert_eq!(
        common::run("fn check(): int32 { let a: int128 = 1000000000000000000000000000000000000; if a / 2 == 500000000000000000000000000000000000 && a % 3 == 1 && a % 7 == 1 { 1 } else { 0 } } fn main(): int32 { check() }")
            .unwrap(),
        1
    );
}

#[test]
fn int128_negative_division_remainder_sign() {
    assert_eq!(
        common::run("fn check(): int32 { let a: int128 = -7; if a / 2 == -3 && a % 2 == -1 && a % 7 == 0 { 1 } else { 0 } } fn main(): int32 { check() }")
            .unwrap(),
        1
    );
}

#[test]
fn int128_floor_division() {
    assert_eq!(
        common::run("fn check(): int32 { let a: int128 = -7; if a // 3 == -3 && a // 2 == -4 { 1 } else { 0 } } fn main(): int32 { check() }")
            .unwrap(),
        1
    );
}

#[test]
fn int128_shifts_and_comparisons() {
    assert_eq!(
        common::run("fn check(): int32 { let a: int128 = 1; let b: int128 = -8; if (a << 100) >> 100 == 1 && b >> 2 == -2 && b < 0 && a > b { 1 } else { 0 } } fn main(): int32 { check() }")
            .unwrap(),
        1
    );
}

#[test]
fn int128_bitwise_ops() {
    assert_eq!(
        common::run("fn check(): int32 { let a: int128 = 170141183460469231731687303715884105727; if (a & 255) == 255 && (a ^ 170141183460469231731687303715884105726) == 1 && (a | 0) == a { 1 } else { 0 } } fn main(): int32 { check() }")
            .unwrap(),
        1
    );
}

#[test]
fn int128_pow() {
    assert_eq!(
        common::run("fn check(): int32 { let a: int128 = 2; if a ** 100 == 1267650600228229401496703205376 { 1 } else { 0 } } fn main(): int32 { check() }")
            .unwrap(),
        1
    );
}

#[test]
fn int128_function_boundary() {
    assert_eq!(
        common::run("fn add(a: int128, b: int128): int128 { a + b } fn check(): int32 { if add(170141183460469231731687303715884105727, 1) == -170141183460469231731687303715884105728 { 1 } else { 0 } } fn main(): int32 { check() }")
            .unwrap(),
        1
    );
}

#[test]
fn int128_compound_assignment() {
    assert_eq!(
        common::run("fn check(): int32 { let mut a: int128 = 100000000000000000000; a += 2; a *= 3; a -= 4; if a == 300000000000000000002 { 1 } else { 0 } } fn main(): int32 { check() }")
            .unwrap(),
        1
    );
}

#[test]
fn int128_min_literal_negation() {
    assert_eq!(
        common::run("fn check(): int32 { let a: int128 = -170141183460469231731687303715884105728; if a < 0 && a + 1 == -170141183460469231731687303715884105727 { 1 } else { 0 } } fn main(): int32 { check() }")
            .unwrap(),
        1
    );
}

#[test]
fn uint128_literal_max() {
    assert_eq!(
        common::run("fn check(): int32 { let a: uint128 = 340282366920938463463374607431768211455; if a == 340282366920938463463374607431768211455 { 1 } else { 0 } } fn main(): int32 { check() }")
            .unwrap(),
        1
    );
}

#[test]
fn uint128_division_and_logical_shift() {
    assert_eq!(
        common::run("fn check(): int32 { let a: uint128 = 340282366920938463463374607431768211455; if a / 2 == 170141183460469231731687303715884105727 && a % 2 == 1 && a >> 64 == 18446744073709551615 { 1 } else { 0 } } fn main(): int32 { check() }")
            .unwrap(),
        1
    );
}
