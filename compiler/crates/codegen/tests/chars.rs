mod common;

#[test]
fn char_literal() {
    assert_eq!(common::run("fn main(): char { 'a' }").unwrap(), 'a' as i64);
}

#[test]
fn char_let() {
    assert_eq!(
        common::run("fn main(): char { let c = 'z'; c }").unwrap(),
        'z' as i64
    );
}

#[test]
fn char_code_point() {
    assert_eq!(common::run("fn main(): char { '!' }").unwrap(), '!' as i64);
}

#[test]
fn char_digit() {
    assert_eq!(common::run("fn main(): char { '7' }").unwrap(), '7' as i64);
}

#[test]
fn char_array_index() {
    assert_eq!(
        common::run("fn main(): char { let arr = ['a', 'b', 'c']; arr[1] }").unwrap(),
        'b' as i64
    );
}

#[test]
fn char_array_first() {
    assert_eq!(
        common::run("fn main(): char { let arr = ['x', 'y', 'z']; arr[0] }").unwrap(),
        'x' as i64
    );
}

#[test]
fn char_array_last() {
    assert_eq!(
        common::run("fn main(): char { let arr = ['x', 'y', 'z']; arr[2] }").unwrap(),
        'z' as i64
    );
}
