mod common;

#[test]
fn string_index_returns_char() {
    let source = "fn f(): char { \"hello\"[0] }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn string_index_non_int32() {
    let source = "fn f() { let c = \"abc\"[true]; }";
    let items = common::compile(source);
    assert!(items.is_err(), "typeck should fail: index must be an integer");
}

#[test]
fn string_index_int32_var() {
    let source = "fn f(i: int32): char { let s = \"hello\"; s[i] }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn string_index_int_var() {
    let source = "fn f(i: int): char { let s = \"hello\"; s[i] }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn string_index_in_let() {
    let source = "fn f(): char { let s = \"hello\"; s[1] }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}
