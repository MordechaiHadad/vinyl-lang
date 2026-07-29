#[test]
fn enum_definition() {
    let source = r#"
enum Option {
    None,
    Some(int32),
    Error { code: int32, message: string },
}
"#;
    let result = vinyl_parser::parse(source);
    assert!(result.is_ok(), "parse should succeed: {:?}", result.err());
}

#[test]
fn enum_empty() {
    let source = "enum Empty {}";
    let result = vinyl_parser::parse(source);
    assert!(result.is_ok(), "parse should succeed: {:?}", result.err());
}
