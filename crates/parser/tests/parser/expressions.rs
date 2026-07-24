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
fn paren_expression() {
    let source = "fn f(): int32 { (1 + 2) * 3 }";
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
