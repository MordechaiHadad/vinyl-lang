#[test]
fn match_expression() {
    let source = r#"
fn f(x: int32): int32 {
    match x {
        1 => 10,
        2 => 20,
        _ => 0,
    }
}
"#;
    let result = vinyl_parser::parse(source);
    assert!(result.is_ok(), "parse should succeed: {:?}", result.err());
}

#[test]
fn match_with_block_arms() {
    let source = r#"
fn f(x: int32): int32 {
    match x {
        1 => { let y = 10; y },
        _ => 0,
    }
}
"#;
    let result = vinyl_parser::parse(source);
    assert!(result.is_ok(), "parse should succeed: {:?}", result.err());
}

#[test]
fn match_struct_pattern() {
    let source = r#"
fn f(p: Point): int32 {
    match p {
        Point { x, y } => x + y,
        _ => 0,
    }
}
"#;
    let result = vinyl_parser::parse(source);
    assert!(result.is_ok(), "parse should succeed: {:?}", result.err());
}

#[test]
fn match_tuple_pattern() {
    let source = r#"
fn f(): int32 {
    let pair = (1, 2);
    match pair {
        (a, b) => a + b,
        _ => 0,
    }
}
"#;
    let result = vinyl_parser::parse(source);
    assert!(result.is_ok(), "parse should succeed: {:?}", result.err());
}

#[test]
fn match_multiple_arms() {
    let source = r#"
fn f(x: int32): int32 {
    match x {
        1 => 10,
        2 => 20,
        3 => 30,
        _ => 0,
    }
}
"#;
    let result = vinyl_parser::parse(source);
    assert!(result.is_ok(), "parse should succeed: {:?}", result.err());
}
