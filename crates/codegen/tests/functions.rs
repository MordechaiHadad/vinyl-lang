mod common;

#[test]
fn implicit_return_literal() {
    assert_eq!(common::run("fn main(): int32 { 42 }").unwrap(), 42);
}

#[test]
fn implicit_return_ident() {
    assert_eq!(
        common::run("fn main(): int32 { let x = 7; let y = 6; x * y }").unwrap(),
        42
    );
}

#[test]
fn implicit_return_binop() {
    assert_eq!(
        common::run("fn square(n: int32): int32 { n * n } fn main(): int32 { square(6) }").unwrap(),
        36
    );
}

#[test]
fn func_no_args() {
    assert_eq!(
        common::run("fn five(): int32 { 5 } fn main(): int32 { five() }").unwrap(),
        5
    );
}

#[test]
fn func_args() {
    assert_eq!(
        common::run("fn add(a: int32, b: int32): int32 { a + b } fn main(): int32 { add(3, 4) }")
            .unwrap(),
        7
    );
}

#[test]
fn func_chain() {
    let src = r#"
        fn add(a: int32, b: int32): int32 { a + b }
        fn triple(n: int32): int32 { n * 3 }
        fn main(): int32 { triple(add(2, 3)) }
    "#;
    assert_eq!(common::run(src).unwrap(), 15);
}

#[test]
fn func_inc_chain() {
    let src = r#"
        fn inc(x: int32): int32 { x + 1 }
        fn add(a: int32, b: int32): int32 { a + b }
        fn main(): int32 { inc(add(inc(3), inc(4))) }
    "#;
    assert_eq!(common::run(src).unwrap(), 10);
}

#[test]
fn func_identity() {
    assert_eq!(
        common::run("fn id(x: int32): int32 { x } fn main(): int32 { id(99) }").unwrap(),
        99
    );
}
