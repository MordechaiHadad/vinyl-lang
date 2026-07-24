#[test]
fn attribute_on_function() {
    let source = r#"
@deprecated
@inline(always)
fn old_function() {}
"#;
    let result = vinyl_parser::parse(source);
    assert!(result.is_ok(), "parse should succeed: {:?}", result.err());
}
