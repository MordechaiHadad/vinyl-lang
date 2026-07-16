#[test]
fn parse_hello_world() {
    let source = r#"
fn main() {
    println("hello world");
}
"#;
    let result = vinyl_parser::parse(source);
    assert!(result.is_ok(), "parse should succeed: {:?}", result.err());
}

#[test]
fn parse_with_error() {
    let source = r#"
fn main() {
    println("hello world"  // missing closing paren
}
"#;
    let result = vinyl_parser::parse(source);
    assert!(result.is_err(), "parse should fail");
}

#[test]
fn parse_let_decl() {
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
fn parse_if_expression() {
    let source = r#"
fn main() {
    if true {
        let x = 1;
    } else {
        let x = 2;
    }
}
"#;
    let result = vinyl_parser::parse(source);
    assert!(result.is_ok(), "parse should succeed: {:?}", result.err());
}
