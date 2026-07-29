#[test]
fn hello_world() {
    let source = r#"
fn main() {
    println("hello world");
}
"#;
    let result = vinyl_parser::parse(source);
    assert!(result.is_ok(), "parse should succeed: {:?}", result.err());
}

#[test]
fn all_literals() {
    let source = r#"
fn main() {
    let a = 42;
    let b = 0xFF;
    let c = 0o77;
    let d = 0b1010;
    let e = -1;
    let f = 3.14;
    let g = 0.5;
    let h = "hello";
    let i = r"raw";
    let j = f"interpolated";
    let k = 'x';
    let l = true;
    let m = false;
}
"#;
    let result = vinyl_parser::parse(source);
    assert!(result.is_ok(), "parse should succeed: {:?}", result.err());
}
