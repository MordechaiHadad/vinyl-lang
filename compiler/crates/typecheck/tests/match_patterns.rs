mod common;

#[test]
fn exhaustive_enum_match() {
    let source = "enum Shape { Circle(int32), Rect(int32, int32), Empty() }
        fn main(): int32 {
            let s = Shape::Circle(5);
            match s {
                Shape::Circle(r) => r,
                Shape::Rect(w, h) => w + h,
                Shape::Empty() => 0,
            }
        }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn match_with_catch_all() {
    let source = "enum Shape { Circle(int32), Rect(int32, int32), Empty() }
        fn main(): int32 {
            let s = Shape::Circle(5);
            match s {
                Shape::Circle(r) => r,
                _ => 0,
            }
        }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn non_exhaustive_enum_match() {
    let source = "enum Shape { Circle(int32), Rect(int32, int32), Empty() }
        fn main(): int32 {
            let s = Shape::Circle(5);
            match s {
                Shape::Circle(r) => r,
                Shape::Rect(w, h) => w + h,
            }
        }";
    let errors = common::compile(source).expect_err("typeck should fail: not exhaustive");
    assert!(
        errors
            .iter()
            .any(|error| error.to_string().contains("not exhaustive")),
        "expected non-exhaustive match diagnostic, got: {errors:?}"
    );
}

#[test]
fn guarded_arm_does_not_count_for_exhaustiveness() {
    let source = "enum Shape { Circle(int32), Rect(int32, int32), Empty() }
        fn main(): int32 {
            let s = Shape::Circle(5);
            match s {
                Shape::Circle(r) if r > 0 => r,
                Shape::Rect(w, h) => w + h,
            }
        }";
    let errors = common::compile(source).expect_err("typeck should fail: guarded arms don't count");
    assert!(
        errors
            .iter()
            .any(|error| error.to_string().contains("not exhaustive")),
        "expected non-exhaustive match diagnostic, got: {errors:?}"
    );
}

#[test]
fn guarded_arm_with_catch_all() {
    let source = "enum Shape { Circle(int32), Rect(int32, int32), Empty() }
        fn main(): int32 {
            let s = Shape::Circle(5);
            match s {
                Shape::Circle(r) if r > 0 => r,
                _ => 0,
            }
        }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn guard_not_bool() {
    let source = r#"fn main(): int32 {
            let x = 5;
            match x {
                0 if "oops" => 1,
                _ => 2,
            }
        }"#;
    let errors = common::compile(source).expect_err("typeck should fail: guard must be bool");
    assert!(
        errors
            .iter()
            .any(|error| error.to_string().contains("must be a bool")),
        "expected guard-not-bool diagnostic, got: {errors:?}"
    );
}

#[test]
fn unknown_variant_in_pattern() {
    let source = "enum Shape { Circle(int32), Rect(int32, int32), Empty() }
        fn main(): int32 {
            let s = Shape::Circle(5);
            match s {
                Shape::Triangle() => 1,
                _ => 0,
            }
        }";
    let errors = common::compile(source).expect_err("typeck should fail: unknown variant");
    assert!(
        errors
            .iter()
            .any(|error| error.to_string().contains("has no variant")),
        "expected variant-not-found diagnostic, got: {errors:?}"
    );
}

#[test]
fn pattern_arity_mismatch() {
    let source = "enum Shape { Circle(int32), Rect(int32, int32), Empty() }
        fn main(): int32 {
            let s = Shape::Circle(5);
            match s {
                Shape::Circle(x, y) => x + y,
                _ => 0,
            }
        }";
    let errors = common::compile(source).expect_err("typeck should fail: arity mismatch");
    assert!(
        errors
            .iter()
            .any(|error| error.to_string().contains("expects 1 arguments")),
        "expected arity mismatch diagnostic, got: {errors:?}"
    );
}

#[test]
fn arm_type_mismatch() {
    let source = "fn main(): int32 {
            let x = 5;
            match x {
                0 => 1,
                _ => \"oops\",
            }
        }";
    let errors = common::compile(source).expect_err("typeck should fail: arm type mismatch");
    assert!(
        errors.iter().any(|error| error.to_string().contains("type mismatch")),
        "expected mismatch diagnostic, got: {errors:?}"
    );
}

#[test]
fn match_on_int_literals() {
    let source = "fn main(): int32 {
            let x = 3;
            match x {
                0 => 10,
                1 => 20,
                _ => 30,
            }
        }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn match_on_tuple() {
    let source = "fn main(): int32 {
            let pair = (1, 2);
            match pair {
                (a, b) => a + b,
                _ => 0,
            }
        }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn match_on_struct() {
    let source = "struct Point { x: int32, y: int32 }
        fn main(): int32 {
            let p = Point { x: 1, y: 2 };
            match p {
                Point { x, y } => x + y,
                _ => 0,
            }
        }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn scoped_enum_variant_pattern() {
    let source = "enum Shape { Circle(int32), Rect(int32, int32), Empty() }
        fn main(): int32 {
            let s = Shape::Circle(5);
            match s {
                Shape::Circle(r) => r,
                _ => 0,
            }
        }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn match_expression_value() {
    let source = "fn main(): int32 { match 2 { 1 => 100, 2 => 200, _ => 0 } }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn match_bool_non_exhaustive() {
    let source = "fn main(): int32 {
            let b = true;
            match b {
                true => 1,
            }
        }";
    let errors = common::compile(source).expect_err("typeck should fail: non-exhaustive bool");
    assert!(
        errors
            .iter()
            .any(|error| error.to_string().contains("not exhaustive")),
        "expected non-exhaustive diagnostic, got: {errors:?}"
    );
}

#[test]
fn payload_binding_scope_is_arm_local() {
    let source = "enum Shape { Circle(int32), Rect(int32, int32), Empty() }
        fn main(): int32 {
            let s = Shape::Circle(5);
            let r = match s {
                Shape::Circle(r) => r,
                _ => 0,
            };
            r
        }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn pattern_binding_immutable() {
    let source = "fn main(): int32 {
            let x = 5;
            match x {
                0 => 1,
                _ => { let y = 2; y }
            }
        }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}
