#[test]
fn type_annotations() {
    let source = r#"
fn f(x: int32, y: float64, z: string, w: bool, c: char): uint8 {
    let a: int64 = 1;
    let b: [int32; 5] = [1, 2, 3, 4, 5];
    let d: Option<int32> = 0;
}
"#;
    let result = vinyl_parser::parse(source);
    assert!(result.is_ok(), "parse should succeed: {:?}", result.err());
}
