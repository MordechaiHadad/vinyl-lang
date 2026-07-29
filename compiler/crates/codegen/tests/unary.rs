mod common;

#[test]
fn not_true() {
    assert_eq!(common::run("fn main(): bool { not true }").unwrap(), 0);
}

#[test]
fn not_false() {
    assert_eq!(common::run("fn main(): bool { not false }").unwrap(), 1);
}

#[test]
fn not_variable() {
    assert_eq!(
        common::run("fn main(): bool { let x = false; not x }").unwrap(),
        1
    );
}

#[test]
fn neg_int() {
    assert_eq!(common::run("fn main(): int64 { -42 }").unwrap(), -42);
}

#[test]
fn neg_variable() {
    assert_eq!(
        common::run("fn main(): int64 { let x = 5; -x }").unwrap(),
        -5
    );
}

#[test]
fn double_neg() {
    assert_eq!(common::run("fn main(): int64 { -(-5) }").unwrap(), 5);
}

#[test]
fn double_not() {
    assert_eq!(common::run("fn main(): bool { !(!true) }").unwrap(), 1);
}

#[test]
fn not_in_if_condition() {
    assert_eq!(
        common::run("fn main(): int64 { let t = true; if not t { 10 } else { 69 } }").unwrap(),
        69
    );
}
