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
