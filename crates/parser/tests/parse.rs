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
fn with_error() {
    let source = r#"
fn main() {
    println("hello world"  // missing closing paren
}
"#;
    let result = vinyl_parser::parse(source);
    assert!(result.is_err(), "parse should fail");
}

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

#[test]
fn function_mut_param() {
    let source = "fn inc(mut x: int32) { let y = x + 1; }";
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

#[test]
fn all_binary_ops() {
    let source = r#"
fn main() {
    let _a = 1 + 2;
    let _b = 3 - 4;
    let _c = 5 * 6;
    let _d = 7 / 8;
    let _e = 9 % 10;
    let _f = 2 ** 3;
    let _g = 10 // 3;
    let _h = 1 << 2;
    let _i = 8 >> 1;
    let _j = 1 & 3;
    let _k = 1 | 2;
    let _l = 1 ^ 3;
    let _m = 1 == 1;
    let _n = 1 != 2;
    let _o = 1 < 2;
    let _p = 2 > 1;
    let _q = 1 <= 1;
    let _r = 2 >= 2;
    let _s = true && false;
    let _t = true || false;
    let _u = true and false;
    let _v = true or false;
    let _w = 1..10;
    let _x = 1..=10;
}
"#;
    let result = vinyl_parser::parse(source);
    assert!(result.is_ok(), "parse should succeed: {:?}", result.err());
}

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
fn block_with_trailing_expr() {
    let source = "fn f(): int32 { let x = 1; let y = 2; x + y }";
    let result = vinyl_parser::parse(source);
    assert!(result.is_ok(), "parse should succeed: {:?}", result.err());
}

#[test]
fn attribute_on_function() {
    let source = r#"
@deprecated
@inline(always)
fn old_function() {}
"#;
    let result = vinyl_parser::parse(source);
    assert!(result.is_ok(), "parse should succeed: {:?}", result.err());
}

#[test]
fn comments() {
    let source = r#"
# line comment
fn main() {
    /* block comment */
    let x = 1; # inline comment
}
"#;
    let result = vinyl_parser::parse(source);
    assert!(result.is_ok(), "parse should succeed: {:?}", result.err());
}

#[test]
fn paren_expression() {
    let source = "fn f(): int32 { (1 + 2) * 3 }";
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
fn enum_definition() {
    let source = r#"
enum Option {
    None,
    Some(int32),
    Error { code: int32, message: string },
}
"#;
    let result = vinyl_parser::parse(source);
    assert!(result.is_ok(), "parse should succeed: {:?}", result.err());
}

#[test]
fn enum_empty() {
    let source = "enum Empty {}";
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

#[test]
fn field_access_expression() {
    let source = r#"
fn f(p: Point): int32 {
    p.x
}
"#;
    let result = vinyl_parser::parse(source);
    assert!(result.is_ok(), "parse should succeed: {:?}", result.err());
}

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
fn empty_source() {
    let source = "";
    let result = vinyl_parser::parse(source);
    assert!(result.is_ok(), "empty source should parse successfully");
}

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
