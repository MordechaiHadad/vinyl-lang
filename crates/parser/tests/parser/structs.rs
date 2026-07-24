#[test]
fn struct_definition() {
    let source = r#"
struct Point {
    x: int32,
    y: int32,
}
"#;
    let result = vinyl_parser::parse(source);
    assert!(result.is_ok(), "parse should succeed: {:?}", result.err());
}

#[test]
fn struct_empty() {
    let source = "struct Empty {}";
    let result = vinyl_parser::parse(source);
    assert!(result.is_ok(), "parse should succeed: {:?}", result.err());
}

#[test]
fn struct_with_comment() {
    let source = r#"
struct Point {
    # comment inside struct
    x: int32,
    y: int32,
}
"#;
    let result = vinyl_parser::parse(source);
    assert!(result.is_ok(), "parse should succeed: {:?}", result.err());
}

#[test]
fn struct_and_fn_together() {
    let source = r#"
struct Point { x: int32, y: int32 }

fn f(p: Point): int32 { p.x }
"#;
    let result = vinyl_parser::parse(source);
    assert!(result.is_ok(), "parse should succeed: {:?}", result.err());
}

#[test]
fn tuple_and_struct_defs() {
    let source = r#"
tuple Pair(int32, int32)
struct Point { x: int32, y: int32 }
"#;
    let result = vinyl_parser::parse(source);
    assert!(result.is_ok(), "parse should succeed: {:?}", result.err());
}

#[test]
fn struct_literal_expression() {
    let source = r#"
struct Point { x: int32, y: int32 }
fn main() {
    let p = Point { x: 1, y: 2 };
}
"#;
    let result = vinyl_parser::parse(source);
    assert!(result.is_ok(), "parse should succeed: {:?}", result.err());
}
