#[test]
fn function_no_params() {
    let source = "fn main() {}";
    let result = vinyl_parser::parse(source);
    assert!(result.is_ok(), "parse should succeed: {:?}", result.err());
}

#[test]
fn function_with_return_type() {
    let source = "fn add(a: int32, b: int32): int32 { a + b }";
    let result = vinyl_parser::parse(source);
    assert!(result.is_ok(), "parse should succeed: {:?}", result.err());
}

// #[test] might re-enable
// fn function_mut_param() {
//     let source = "fn inc(mut x: int32) { let y = x + 1; }";
//     let result = vinyl_parser::parse(source);
//     assert!(result.is_ok(), "parse should succeed: {:?}", result.err());
// }

#[test]
fn multiple_functions() {
    let source = r#"
fn foo() {}
fn bar() {}
fn baz() {}
"#;
    let result = vinyl_parser::parse(source);
    assert!(result.is_ok(), "parse should succeed: {:?}", result.err());
}
