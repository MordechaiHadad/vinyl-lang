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

#[test]
fn unit_return_unit() {
    assert_eq!(common::run("fn main(): unit { unit }").unwrap(), 0);
}

#[test]
fn int_default_literal() {
    assert_eq!(common::run("fn main(): int { 42 }").unwrap(), 42);
}

#[test]
fn pipe_first_arg() {
    assert_eq!(
        common::run("fn add(a: int32, b: int32): int32 { a + b } fn main(): int32 { 5 |> add(3) }")
            .unwrap(),
        8
    );
}

#[test]
fn pipe_last_arg() {
    assert_eq!(
        common::run("fn add(a: int32, b: int32): int32 { a + b } fn main(): int32 { 3 |>> add(5) }")
            .unwrap(),
        8
    );
}

#[test]
fn pipe_chain() {
    let src = r#"
        fn add(a: int32, b: int32): int32 { a + b }
        fn triple(n: int32): int32 { n * 3 }
        fn main(): int32 { 2 |> add(3) |> triple() }
    "#;
    assert_eq!(common::run(src).unwrap(), 15);
}

#[test]
fn pipe_bare_ident() {
    assert_eq!(
        common::run("fn double(n: int32): int32 { n * 2 } fn main(): int32 { 7 |> double }")
            .unwrap(),
        14
    );
}
