// need to disable block comment
#[test]
fn comments() {
    let source = r#"
# line comment
fn main() {
    /* block comment */
    let x = 1; # inline comment
}
"#;
    let result = vinyl_parser::parse(source);
    assert!(result.is_ok(), "parse should succeed: {:?}", result.err());
}
