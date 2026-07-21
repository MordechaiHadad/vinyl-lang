mod common;

#[test]
fn let_binding() {
    assert_eq!(common::run("fn main(): int32 { let x = 5; x }").unwrap(), 5);
}

#[test]
fn let_shadow() {
    assert_eq!(
        common::run("fn main(): int32 { let x = 1; let x = 2; x }").unwrap(),
        2
    );
}

#[test]
fn multi_let_block() {
    assert_eq!(
        common::run("fn main(): int32 { let x = 5; let y = 3; x + y }").unwrap(),
        8
    );
}

#[test]
fn int_literal_infers_from_ret_uint32() {
    assert_eq!(common::run("fn main(): uint32 { 42 }").unwrap(), 42);
}

#[test]
fn int_literal_infers_from_ret_isize() {
    assert_eq!(common::run("fn main(): isize { 42 }").unwrap(), 42);
}

#[test]
fn int_literal_infers_from_ret_usize() {
    assert_eq!(common::run("fn main(): usize { 42 }").unwrap(), 42);
}

#[test]
fn no_main_returns_zero() {
    assert_eq!(common::run("fn foo(): int32 { 1 }").unwrap(), 0);
}
