mod common;

#[test]
fn mutable_var_assignment() {
    assert_eq!(
        common::run("fn main(): int32 { let mut x = 5; x = 10; x }").unwrap(),
        10
    );
}

#[test]
fn parameters_are_mutable_by_default() {
    assert_eq!(
        common::run(
            "fn oof(param: int32): int32 { param *= 2; param } fn main(): int32 { oof(10) }"
        )
        .unwrap(),
        20
    );
}

#[test]
fn repeated_assignment() {
    assert_eq!(
        common::run("fn main(): int32 { let mut x = 0; x = 1; x = 2; x }").unwrap(),
        2
    );
}

#[test]
fn compound_add_eq() {
    assert_eq!(
        common::run("fn main(): int32 { let mut x = 5; x += 3; x }").unwrap(),
        8
    );
}

#[test]
fn compound_sub_eq() {
    assert_eq!(
        common::run("fn main(): int32 { let mut x = 10; x -= 3; x }").unwrap(),
        7
    );
}

#[test]
fn compound_mul_eq() {
    assert_eq!(
        common::run("fn main(): int32 { let mut x = 5; x *= 3; x }").unwrap(),
        15
    );
}

#[test]
fn compound_div_eq() {
    assert_eq!(
        common::run("fn main(): int32 { let mut x = 10; x /= 3; x }").unwrap(),
        3
    );
}

#[test]
fn compound_rem_eq() {
    assert_eq!(
        common::run("fn main(): int32 { let mut x = 10; x %= 3; x }").unwrap(),
        1
    );
}

#[test]
fn compound_bitand_eq() {
    assert_eq!(
        common::run("fn main(): int32 { let mut x = 6; x &= 3; x }").unwrap(),
        2
    );
}

#[test]
fn compound_bitor_eq() {
    assert_eq!(
        common::run("fn main(): int32 { let mut x = 4; x |= 3; x }").unwrap(),
        7
    );
}

#[test]
fn compound_shl_eq() {
    assert_eq!(
        common::run("fn main(): int32 { let mut x = 1; x <<= 2; x }").unwrap(),
        4
    );
}

#[test]
fn compound_shr_eq() {
    assert_eq!(
        common::run("fn main(): int32 { let mut x = 8; x >>= 2; x }").unwrap(),
        2
    );
}

#[test]
fn mutable_through_if_branches() {
    assert_eq!(
        common::run("fn main(): int32 { let mut x = 0; if true { x = 42; } x }").unwrap(),
        42
    );
}

#[test]
fn mutable_through_loop() {
    assert_eq!(
        common::run("fn main(): int32 { let mut x = 0; loop { x = 7; break; } x }").unwrap(),
        7
    );
}

#[test]
fn ref_mut_var_changes_seen() {
    assert_eq!(
        common::run("fn main(): int32 { let mut x = 10; let r = &x; x = 69; x }").unwrap(),
        69
    );
}

#[test]
fn ref_immutable_var_allowed() {
    assert_eq!(
        common::run("fn main(): int32 { let x = 10; let r = &x; x }").unwrap(),
        10
    );
}

#[test]
fn write_through_mut_ref() {
    assert_eq!(
        common::run("fn main(): int32 { let mut x = 10; let mut r = &x; r = 20; x }").unwrap(),
        20
    );
}

#[test]
fn value_store_through_ref() {
    assert_eq!(
        common::run(
            "fn main(): int32 { let mut x = 10; let mut z = 69; let mut y = &x; y = z; x }"
        )
        .unwrap(),
        69
    );
}

#[test]
fn rebind_ref_then_write() {
    let r = common::run(
        "fn main(): int32 { let mut x = 10; let mut z = 69; let mut y = &x; y = &z; y = 420; z }",
    );
    if let Err(ref e) = r {
        eprintln!("{e}");
    }
    assert_eq!(r.unwrap(), 420);
}

#[test]
fn write_through_ref_param() {
    assert_eq!(
        common::run("fn foo(p: &int32) { p = 10; } fn main(): int32 { let mut x = 5; foo(&x); x }")
            .unwrap(),
        10
    );
}

#[test]
fn compound_write_through_ref_param() {
    assert_eq!(
        common::run(
            "fn foo(p: &int32) { p *= 2; } fn main(): int32 { let mut x = 10; foo(&x); x }"
        )
        .unwrap(),
        20
    );
}

#[test]
fn compound_bitxor_eq() {
    assert_eq!(
        common::run("fn main(): int32 { let mut x = 6; x ^= 3; x }").unwrap(),
        5
    );
}

#[test]
fn ref_auto_deref_in_return() {
    assert_eq!(
        common::run("fn main(): int32 { let mut x = 10; let r = &x; x = 69; r }").unwrap(),
        69
    );
}

#[test]
fn ref_in_binary_expr() {
    assert_eq!(
        common::run("fn main(): int32 { let mut x = 5; let r = &x; let s = &x; r + s }").unwrap(),
        10
    );
}

#[test]
fn ref_passed_as_value() {
    assert_eq!(
        common::run("fn double(n: int32): int32 { n * 2 } fn main(): int32 { let mut x = 21; let r = &x; double(r) }").unwrap(),
        42
    );
}
