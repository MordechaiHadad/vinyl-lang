mod common;

#[test]
fn if_true() {
    assert_eq!(
        common::run("fn main(): int32 { if true { return 1; } 0 }").unwrap(),
        1
    );
}

#[test]
fn if_false() {
    assert_eq!(
        common::run("fn main(): int32 { if false { return 1; } 0 }").unwrap(),
        0
    );
}

#[test]
fn if_else_true() {
    assert_eq!(
        common::run("fn main(): int32 { if true { return 1; } else { return 2; } 0 }").unwrap(),
        1
    );
}

#[test]
fn if_else_false() {
    assert_eq!(
        common::run("fn main(): int32 { if false { return 1; } else { return 2; } 0 }").unwrap(),
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
    assert_eq!(common::run(src).unwrap(), 42);
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
    assert_eq!(common::run(src).unwrap(), 10);
}

#[test]
fn cmp_eq_true() {
    assert_eq!(
        common::run("fn main(): int32 { if 3 == 3 { return 1; } 0 }").unwrap(),
        1
    );
}

#[test]
fn cmp_eq_false() {
    assert_eq!(
        common::run("fn main(): int32 { if 3 == 4 { return 1; } 0 }").unwrap(),
        0
    );
}

#[test]
fn cmp_ne() {
    assert_eq!(
        common::run("fn main(): int32 { if 3 != 4 { return 1; } 0 }").unwrap(),
        1
    );
}

#[test]
fn cmp_lt() {
    assert_eq!(
        common::run("fn main(): int32 { if 2 < 3 { return 1; } 0 }").unwrap(),
        1
    );
}

#[test]
fn cmp_gt() {
    assert_eq!(
        common::run("fn main(): int32 { if 5 > 3 { return 1; } 0 }").unwrap(),
        1
    );
}

#[test]
fn cmp_le() {
    assert_eq!(
        common::run("fn main(): int32 { if 3 <= 3 { return 1; } 0 }").unwrap(),
        1
    );
}

#[test]
fn cmp_ge() {
    assert_eq!(
        common::run("fn main(): int32 { if 3 >= 3 { return 1; } 0 }").unwrap(),
        1
    );
}

#[test]
fn and_both_true() {
    assert_eq!(
        common::run("fn main(): int32 { if true && true { return 1; } 0 }").unwrap(),
        1
    );
}

#[test]
fn and_left_false() {
    assert_eq!(
        common::run("fn main(): int32 { if false && true { return 1; } 0 }").unwrap(),
        0
    );
}

#[test]
fn or_both_false() {
    assert_eq!(
        common::run("fn main(): int32 { if false || false { return 1; } 0 }").unwrap(),
        0
    );
}

#[test]
fn or_left_true() {
    assert_eq!(
        common::run("fn main(): int32 { if true || false { return 1; } 0 }").unwrap(),
        1
    );
}
