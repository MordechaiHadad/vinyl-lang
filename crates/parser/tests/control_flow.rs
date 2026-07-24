#[test]
fn if_expression() {
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

#[test]
fn if_else_if_else() {
    let source = r#"
fn main() {
    if x < 0 {
        "negative"
    } else if x == 0 {
        "zero"
    } else {
        "positive"
    }
}
"#;
    let result = vinyl_parser::parse(source);
    assert!(result.is_ok(), "parse should succeed: {:?}", result.err());
}

#[test]
fn while_loop() {
    let source = r#"
fn main() {
    let mut x = 0;
    while x < 10 {
        let y = x + 1;
    }
}
"#;
    let result = vinyl_parser::parse(source);
    assert!(result.is_ok(), "parse should succeed: {:?}", result.err());
}

#[test]
fn loop_statement() {
    let source = r#"
fn main() {
    loop {
        if true { break; }
        break;
    }
}
"#;
    let result = vinyl_parser::parse(source);
    assert!(result.is_ok(), "parse should succeed: {:?}", result.err());
}

#[test]
fn break_continue() {
    let source = r#"
fn main() {
    loop {
        continue;
        break;
    }
}
"#;
    let result = vinyl_parser::parse(source);
    assert!(result.is_ok(), "parse should succeed: {:?}", result.err());
}
