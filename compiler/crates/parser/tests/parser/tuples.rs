#[test]
fn tuple_definition() {
    let source = "tuple Pair(int32, float64)";
    let result = vinyl_parser::parse(source);
    assert!(result.is_ok(), "parse should succeed: {:?}", result.err());
}

#[test]
fn tuple_empty_parens() {
    let source = "tuple Unit()";
    let result = vinyl_parser::parse(source);
    assert!(result.is_ok(), "parse should succeed: {:?}", result.err());
}

#[test]
fn tuple_type_annotation() {
    let source = r#"
fn f(x: (int32, float64)) {}
"#;
    let result = vinyl_parser::parse(source);
    assert!(result.is_ok(), "parse should succeed: {:?}", result.err());
}

#[test]
fn tuple_expression() {
    let source = r#"
fn f() {
    let a = (1, 2);
    let b = (1,);
}
"#;
    let result = vinyl_parser::parse(source);
    assert!(result.is_ok(), "parse should succeed: {:?}", result.err());
}
