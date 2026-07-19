use vinyl_codegen::cranelift::CraneliftBackend;
use vinyl_codegen::CodegenBackend;

fn run(source: &str) -> Result<i64, String> {
    let tree = vinyl_parser::parse(source).map_err(|_| "parse error")?;
    let items = vinyl_parser::lower::lower(&tree, source, "<test>")
        .map_err(|e| format!("lower error: {e:?}"))?;
    let hir = vinyl_typecheck::typeck(&items, source, "<test>")
        .map_err(|_| "type error")?;
    let mut backend =
        CraneliftBackend::new().map_err(|e| format!("backend error: {e}"))?;
    backend
        .compile(&hir)
        .map_err(|e| format!("compile error: {e}"))?;
    backend.run().map_err(|e| format!("run error: {e}"))
}

// -- arithmetic -- //

#[test]
fn arithmetic_add() {
    assert_eq!(run("fn main(): int32 { 1 + 2 }").unwrap(), 3);
}

#[test]
fn arithmetic_sub() {
    assert_eq!(run("fn main(): int32 { 10 - 3 }").unwrap(), 7);
}

#[test]
fn arithmetic_mul() {
    assert_eq!(run("fn main(): int32 { 6 * 7 }").unwrap(), 42);
}

#[test]
fn arithmetic_div() {
    assert_eq!(run("fn main(): int32 { 10 / 3 }").unwrap(), 3);
}

#[test]
fn arithmetic_rem() {
    assert_eq!(run("fn main(): int32 { 10 % 3 }").unwrap(), 1);
}

#[test]
fn arithmetic_precedence() {
    assert_eq!(run("fn main(): int32 { 1 + 2 * 3 }").unwrap(), 7);
}

#[test]
fn arithmetic_parens() {
    assert_eq!(run("fn main(): int32 { (1 + 2) * 3 }").unwrap(), 9);
}

// -- floor division -- //

#[test]
fn floor_div_positive() {
    assert_eq!(run("fn main(): int32 { 7 // 3 }").unwrap(), 2);
}

#[test]
fn floor_div_both_neg() {
    assert_eq!(run("fn main(): int32 { (-7) // (-3) }").unwrap(), 2);
}

#[test]
fn floor_div_mixed_signs_returns_positive() {
    let r = run("fn main(): int32 { (7 // 3) + ((-7) // (-3)) }").unwrap();
    assert_eq!(r, 4);
}

// -- let bindings -- //

#[test]
fn let_binding() {
    assert_eq!(run("fn main(): int32 { let x = 5; x }").unwrap(), 5);
}

#[test]
fn let_shadow() {
    assert_eq!(
        run("fn main(): int32 { let x = 1; let x = 2; x }").unwrap(),
        2
    );
}

#[test]
fn multi_let_block() {
    assert_eq!(
        run("fn main(): int32 { let x = 5; let y = 3; x + y }").unwrap(),
        8
    );
}

// -- if / else -- //

#[test]
fn if_true() {
    assert_eq!(
        run("fn main(): int32 { if true { return 1; } 0 }").unwrap(),
        1
    );
}

#[test]
fn if_false() {
    assert_eq!(
        run("fn main(): int32 { if false { return 1; } 0 }").unwrap(),
        0
    );
}

#[test]
fn if_else_true() {
    assert_eq!(
        run("fn main(): int32 { if true { return 1; } else { return 2; } 0 }")
            .unwrap(),
        1
    );
}

#[test]
fn if_else_false() {
    assert_eq!(
        run("fn main(): int32 { if false { return 1; } else { return 2; } 0 }")
            .unwrap(),
        2
    );
}

#[test]
fn nested_if() {
    let src = r#"
        fn main(): int32 {
            if true {
                if false { return 0; }
            }
            42
        }
    "#;
    assert_eq!(run(src).unwrap(), 42);
}

#[test]
fn nested_if_else() {
    let src = r#"
        fn main(): int32 {
            if true {
                if true { return 10; } else { return 20; }
            }
            0
        }
    "#;
    assert_eq!(run(src).unwrap(), 10);
}

// -- function calls -- //

#[test]
fn func_no_args() {
    assert_eq!(
        run("fn five(): int32 { 5 } fn main(): int32 { five() }").unwrap(),
        5
    );
}

#[test]
fn func_args() {
    assert_eq!(
        run("fn add(a: int32, b: int32): int32 { a + b } fn main(): int32 { add(3, 4) }")
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
    assert_eq!(run(src).unwrap(), 15);
}

#[test]
fn func_inc_chain() {
    let src = r#"
        fn inc(x: int32): int32 { x + 1 }
        fn add(a: int32, b: int32): int32 { a + b }
        fn main(): int32 { inc(add(inc(3), inc(4))) }
    "#;
    assert_eq!(run(src).unwrap(), 10);
}

#[test]
fn func_identity() {
    assert_eq!(
        run("fn id(x: int32): int32 { x } fn main(): int32 { id(99) }").unwrap(),
        99
    );
}

// -- implicit return -- //

#[test]
fn implicit_return_literal() {
    assert_eq!(run("fn main(): int32 { 42 }").unwrap(), 42);
}

#[test]
fn implicit_return_ident() {
    assert_eq!(
        run("fn main(): int32 { let x = 7; let y = 6; x * y }").unwrap(),
        42
    );
}

#[test]
fn implicit_return_binop() {
    assert_eq!(
        run("fn square(n: int32): int32 { n * n } fn main(): int32 { square(6) }")
            .unwrap(),
        36
    );
}

// -- bitwise -- //

#[test]
fn bitwise_and() {
    assert_eq!(run("fn main(): int32 { 6 & 3 }").unwrap(), 2);
}

#[test]
fn bitwise_or() {
    assert_eq!(run("fn main(): int32 { 2 | 4 }").unwrap(), 6);
}

#[test]
fn bitwise_xor() {
    assert_eq!(run("fn main(): int32 { 5 ^ 3 }").unwrap(), 6);
}

#[test]
fn shift_left() {
    assert_eq!(run("fn main(): int32 { 3 << 2 }").unwrap(), 12);
}

#[test]
fn shift_right() {
    assert_eq!(run("fn main(): int32 { 16 >> 2 }").unwrap(), 4);
}

// -- comparison -- //

#[test]
fn cmp_eq_true() {
    assert_eq!(
        run("fn main(): int32 { if 3 == 3 { return 1; } 0 }").unwrap(),
        1
    );
}

#[test]
fn cmp_eq_false() {
    assert_eq!(
        run("fn main(): int32 { if 3 == 4 { return 1; } 0 }").unwrap(),
        0
    );
}

#[test]
fn cmp_ne() {
    assert_eq!(
        run("fn main(): int32 { if 3 != 4 { return 1; } 0 }").unwrap(),
        1
    );
}

#[test]
fn cmp_lt() {
    assert_eq!(
        run("fn main(): int32 { if 2 < 3 { return 1; } 0 }").unwrap(),
        1
    );
}

#[test]
fn cmp_gt() {
    assert_eq!(
        run("fn main(): int32 { if 5 > 3 { return 1; } 0 }").unwrap(),
        1
    );
}

#[test]
fn cmp_le() {
    assert_eq!(
        run("fn main(): int32 { if 3 <= 3 { return 1; } 0 }").unwrap(),
        1
    );
}

#[test]
fn cmp_ge() {
    assert_eq!(
        run("fn main(): int32 { if 3 >= 3 { return 1; } 0 }").unwrap(),
        1
    );
}

// -- logical -- //

#[test]
fn and_both_true() {
    assert_eq!(
        run("fn main(): int32 { if true && true { return 1; } 0 }").unwrap(),
        1
    );
}

#[test]
fn and_left_false() {
    assert_eq!(
        run("fn main(): int32 { if false && true { return 1; } 0 }").unwrap(),
        0
    );
}

#[test]
fn or_both_false() {
    assert_eq!(
        run("fn main(): int32 { if false || false { return 1; } 0 }").unwrap(),
        0
    );
}

#[test]
fn or_left_true() {
    assert_eq!(
        run("fn main(): int32 { if true || false { return 1; } 0 }").unwrap(),
        1
    );
}

// -- literals with type inference -- //

#[test]
fn int_literal_infers_from_ret_uint32() {
    assert_eq!(
        run("fn main(): uint32 { 42 }").unwrap(),
        42
    );
}

#[test]
fn int_literal_infers_from_ret_isize() {
    assert_eq!(
        run("fn main(): isize { 42 }").unwrap(),
        42
    );
}

#[test]
fn int_literal_infers_from_ret_usize() {
    assert_eq!(
        run("fn main(): usize { 42 }").unwrap(),
        42
    );
}

// -- no main -- //

#[test]
fn no_main_returns_zero() {
    assert_eq!(run("fn foo(): int32 { 1 }").unwrap(), 0);
}
