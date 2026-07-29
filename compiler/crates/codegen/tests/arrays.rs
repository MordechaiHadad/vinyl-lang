mod common;

#[test]
fn array_index_first() {
    assert_eq!(
        common::run("fn main(): int32 { let arr = [10, 20, 30]; arr[0] }").unwrap(),
        10
    );
}

#[test]
fn array_index_last() {
    assert_eq!(
        common::run("fn main(): int32 { let arr = [10, 20, 30]; arr[2] }").unwrap(),
        30
    );
}

#[test]
fn array_index_middle() {
    assert_eq!(
        common::run("fn main(): int32 { let arr = [1, 2, 3]; arr[1] }").unwrap(),
        2
    );
}

#[test]
fn array_index_expr() {
    assert_eq!(
        common::run("fn main(): int32 { let arr = [5, 10, 15]; arr[1 + 1] }").unwrap(),
        15
    );
}

#[test]
fn array_ops_between_indices() {
    assert_eq!(
        common::run("fn main(): int32 { let arr = [10, 20, 30]; arr[0] + arr[2] }").unwrap(),
        40
    );
}

#[test]
fn array_multi_element() {
    assert_eq!(
        common::run("fn main(): int32 { let arr = [1, 2, 3, 4, 5]; arr[3] }").unwrap(),
        4
    );
}

#[test]
fn array_bool_index() {
    assert_eq!(
        common::run(
            "fn main(): int32 { let arr = [true, false, true]; if arr[0] { 1 } else { 0 } }"
        )
        .unwrap(),
        1
    );
}

#[test]
fn array_bool_index_false() {
    assert_eq!(
        common::run(
            "fn main(): int32 { let arr = [true, false, true]; if arr[1] { 1 } else { 0 } }"
        )
        .unwrap(),
        0
    );
}
