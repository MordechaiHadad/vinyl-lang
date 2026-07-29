#[test]
fn array_expression() {
    let source = r#"
fn main() {
    let a = [1, 2, 3];
    let b = [];
    let c = [[1, 2], [3, 4]];
}
"#;
    let result = vinyl_parser::parse(source);
    assert!(result.is_ok(), "parse should succeed: {:?}", result.err());
}

#[test]
fn index_expression() {
    let source = r#"
fn main() {
    let a = [1, 2, 3];
    let b = a[0];
    let c = a[i];
}
"#;
    let result = vinyl_parser::parse(source);
    assert!(result.is_ok(), "parse should succeed: {:?}", result.err());
}
