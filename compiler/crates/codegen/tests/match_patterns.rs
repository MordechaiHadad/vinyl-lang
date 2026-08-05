mod common;

#[test]
fn match_enum_payload_extraction() {
    assert_eq!(
        common::run("enum Shape { Circle(int32), Empty() }
            fn main(): int32 {
                let s = Shape::Circle(7);
                match s {
                    Shape::Circle(r) => r,
                    _ => 0,
                }
            }")
        .unwrap(),
        7
    );
}

#[test]
fn match_multi_variant_payload() {
    assert_eq!(
        common::run("enum Shape { Circle(int32), Rect(int32, int32), Empty() }
            fn main(): int32 {
                let s = Shape::Rect(3, 4);
                match s {
                    Shape::Circle(r) => r,
                    Shape::Rect(w, h) => w * h,
                    Shape::Empty() => 0,
                }
            }")
        .unwrap(),
        12
    );
}

#[test]
fn match_wildcard_catch_all() {
    assert_eq!(
        common::run("enum Color { Red, Green, Blue }
            fn main(): int32 {
                let c = Color::Blue();
                match c {
                    Color::Red() => 1,
                    _ => 2,
                }
            }")
        .unwrap(),
        2
    );
}

#[test]
fn match_int_literals() {
    assert_eq!(
        common::run("fn main(): int32 {
                let x = 3;
                match x {
                    0 => 10,
                    1 => 20,
                    _ => 30,
                }
            }")
        .unwrap(),
        30
    );
}

#[test]
fn match_with_guard() {
    assert_eq!(
        common::run("fn main(): int32 {
                let x = 5;
                match x {
                    0 => 1,
                    v if v > 3 => 2,
                    _ => 3,
                }
            }")
        .unwrap(),
        2
    );
}

#[test]
fn match_guard_falls_through_to_catch_all() {
    assert_eq!(
        common::run("fn main(): int32 {
                let x = 2;
                match x {
                    0 => 1,
                    v if v > 3 => 2,
                    _ => 3,
                }
            }")
        .unwrap(),
        3
    );
}

#[test]
fn match_tuple_pattern() {
    assert_eq!(
        common::run("fn main(): int32 {
                let pair = (1, 2);
                match pair {
                    (a, b) => a + b,
                    _ => 0,
                }
            }")
        .unwrap(),
        3
    );
}

#[test]
fn match_struct_pattern() {
    assert_eq!(
        common::run("struct Point { x: int32, y: int32 }
            fn main(): int32 {
                let p = Point { x: 4, y: 5 };
                match p {
                    Point { x, y } => x * y,
                    _ => 0,
                }
            }")
        .unwrap(),
        20
    );
}

#[test]
fn match_expression_value() {
    assert_eq!(
        common::run("fn main(): int32 {
                let result = match 2 { 1 => 100, 2 => 200, _ => 0 };
                result + 1
            }")
        .unwrap(),
        201
    );
}

#[test]
fn match_bool_literals() {
    assert_eq!(
        common::run("fn main(): int32 {
                let b = true;
                match b {
                    true => 1,
                    false => 0,
                }
            }")
        .unwrap(),
        1
    );
}

#[test]
fn match_guarded_enum_payload() {
    assert_eq!(
        common::run("enum Shape { Circle(int32), Empty() }
            fn main(): int32 {
                let s = Shape::Circle(10);
                match s {
                    Shape::Circle(r) if r > 5 => r * 2,
                    _ => 0,
                }
            }")
        .unwrap(),
        20
    );
}
