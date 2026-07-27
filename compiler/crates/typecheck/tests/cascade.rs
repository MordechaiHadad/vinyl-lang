mod common;

#[test]
fn assign_type_mismatch_no_cascade() {
    let source = "
fn main(): int32 {
    let mut value = 10;
    let view = &value;
    value = 20;
    value = \"\";
    value = 30;
    view + value
}
";
    let result = common::compile(source);
    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert_eq!(
        errors.len(),
        1,
        "assigning string to int var should produce exactly 1 error, got: {:?}",
        errors
    );
}

#[test]
fn deref_assign_type_mismatch_no_cascade() {
    let source = "
fn main(): int32 {
    let mut x = 10;
    let mut r = &x;
    r = \"hello\";
    r = 20;
    x + 5
}
";
    let result = common::compile(source);
    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert_eq!(
        errors.len(),
        1,
        "deref assigning string to &int should produce exactly 1 error, got: {:?}",
        errors
    );
}

#[test]
fn ref_assign_type_mismatch_no_cascade() {
    let source = "
fn main(): int32 {
    let mut x = 10;
    let mut y = 20;
    let mut r = &x;
    r = &\"hello\";
    r = &y;
    x + y
}
";
    let result = common::compile(source);
    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert_eq!(
        errors.len(),
        1,
        "assigning &string to &int var should produce exactly 1 error, got: {:?}",
        errors
    );
}
