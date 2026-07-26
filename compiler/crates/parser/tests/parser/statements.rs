#[test]
fn let_decl() {
    let source = r#"
fn main() {
    let x: int32 = 42;
    let y = 10;
    let mut z: float64 = 3.14;
    return x + y;
}
"#;
    let result = vinyl_parser::parse(source);
    assert!(result.is_ok(), "parse should succeed: {:?}", result.err());
}

#[test]
fn return_with_expression() {
    let source = "fn f(): int32 { return 42; }";
    let result = vinyl_parser::parse(source);
    assert!(result.is_ok(), "parse should succeed: {:?}", result.err());
}

#[test]
fn return_void() {
    let source = "fn f() { return; }";
    let result = vinyl_parser::parse(source);
    assert!(result.is_ok(), "parse should succeed: {:?}", result.err());
}

#[test]
fn block_with_trailing_expr() {
    let source = "fn f(): int32 { let x = 1; let y = 2; x + y }";
    let result = vinyl_parser::parse(source);
    assert!(result.is_ok(), "parse should succeed: {:?}", result.err());
}

#[test]
fn missing_semicolon() {
    let source = "fn main() { let x = 1 }";
    let result = vinyl_parser::parse(source);
    assert!(result.is_err(), "parse should fail: missing semicolon");
}

#[test]
fn missing_closing_brace() {
    let source = "fn main() { let x = 1; ";
    let result = vinyl_parser::parse(source);
    assert!(result.is_err(), "parse should fail: missing closing brace");
}

#[test]
fn let_without_equals() {
    let source = "fn main() { let x int32; }";
    let result = vinyl_parser::parse(source);
    assert!(result.is_err(), "parse should fail: let without =");
}
