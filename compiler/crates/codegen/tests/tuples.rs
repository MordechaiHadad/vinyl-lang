mod common;

#[test]
fn tuple_first_field() {
    assert_eq!(
        common::run("fn main(): int32 { let t = (10, 20); t.0 }").unwrap(),
        10
    );
}

#[test]
fn tuple_second_field() {
    assert_eq!(
        common::run("fn main(): int32 { let t = (10, 20); t.1 }").unwrap(),
        20
    );
}

#[test]
fn tuple_three_fields() {
    assert_eq!(
        common::run("fn main(): int32 { let t = (1, 2, 3); t.0 + t.2 }").unwrap(),
        4
    );
}

#[test]
fn tuple_expression_direct() {
    assert_eq!(
        common::run("fn main(): int32 { (100, 200).0 }").unwrap(),
        100
    );
}
