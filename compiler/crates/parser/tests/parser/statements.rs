#[test]
fn let_decl() {
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
fn return_with_expression() {
    let source = "fn f(): int32 { return 42; }";
    let result = vinyl_parser::parse(source);
    assert!(result.is_ok(), "parse should succeed: {:?}", result.err());
}

#[test]
fn return_void() {
    let source = "fn f() { return; }";
    let result = vinyl_parser::parse(source);
    assert!(result.is_ok(), "parse should succeed: {:?}", result.err());
}

#[test]
fn block_with_trailing_expr() {
    let source = "fn f(): int32 { let x = 1; let y = 2; x + y }";
    let result = vinyl_parser::parse(source);
    assert!(result.is_ok(), "parse should succeed: {:?}", result.err());
}

#[test]
fn missing_semicolon() {
    let source = "fn main() { let x = 1 }";
    let result = vinyl_parser::parse(source);
    assert!(result.is_err(), "parse should fail: missing semicolon");
}

#[test]
fn missing_closing_brace() {
    let source = "fn main() { let x = 1; ";
    let result = vinyl_parser::parse(source);
    assert!(result.is_err(), "parse should fail: missing closing brace");
}

#[test]
fn let_without_equals() {
    let source = "fn main() { let x int32; }";
    let result = vinyl_parser::parse(source);
    assert!(result.is_err(), "parse should fail: let without =");
}

#[test]
fn import_self_prefix() {
    let result = vinyl_parser::parse("import self::foo;");
    assert!(
        result.is_ok(),
        "self:: prefix should parse: {:?}",
        result.err()
    );
}

#[test]
fn import_parent_prefix() {
    let result = vinyl_parser::parse("import parent::bar;");
    assert!(
        result.is_ok(),
        "parent:: prefix should parse: {:?}",
        result.err()
    );
}

#[test]
fn import_stacked_parent_prefix() {
    let result = vinyl_parser::parse("import parent::parent::baz;");
    assert!(
        result.is_ok(),
        "stacked parent:: prefix should parse: {:?}",
        result.err()
    );
}

#[test]
fn import_package_prefix() {
    let result = vinyl_parser::parse("import package::qux;");
    assert!(
        result.is_ok(),
        "package:: prefix should parse: {:?}",
        result.err()
    );
}

#[test]
fn import_self_nested_path() {
    let result = vinyl_parser::parse("import self::module::math;");
    assert!(
        result.is_ok(),
        "self:: with nested path should parse: {:?}",
        result.err()
    );
}

#[test]
fn import_bare_parent_errors() {
    let result = vinyl_parser::parse("import parent;");
    assert!(result.is_err(), "bare `parent` keyword should error");
}

#[test]
fn import_bare_self_errors() {
    let result = vinyl_parser::parse("import self;");
    assert!(result.is_err(), "bare `self` keyword should error");
}

#[test]
fn import_bare_package_errors() {
    let result = vinyl_parser::parse("import package;");
    assert!(result.is_err(), "bare `package` keyword should error");
}
