#[test]
fn empty_source() {
    let source = "";
    let result = vinyl_parser::parse(source);
    assert!(result.is_ok(), "empty source should parse successfully");
}

#[test]
fn with_error() {
    let source = r#"
fn main() {
    println("hello world"  // missing closing paren
}
"#;
    let result = vinyl_parser::parse(source);
    assert!(result.is_err(), "parse should fail");
}
