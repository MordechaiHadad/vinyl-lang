mod common;

#[test]
fn small_struct_field_access() {
    assert_eq!(
        common::run("struct Point { x: int32, y: int32 } fn main(): int32 { let p = Point { x: 10, y: 20 }; p.x }")
            .unwrap(),
        10
    );
}

#[test]
fn small_struct_both_fields() {
    assert_eq!(
        common::run("struct Point { x: int32, y: int32 } fn main(): int32 { let p = Point { x: 10, y: 20 }; p.x + p.y }")
            .unwrap(),
        30
    );
}

#[test]
fn small_struct_field_assignment() {
    assert_eq!(
        common::run("struct Point { x: int32, y: int32 } fn main(): int32 { let mut p = Point { x: 10, y: 20 }; p.x = 30; p.x }")
            .unwrap(),
        30
    );
}

#[test]
fn small_struct_param() {
    assert_eq!(
        common::run("struct Point { x: int32, y: int32 } fn id(p: Point): Point { p } fn main(): int32 { let p = Point { x: 7, y: 3 }; id(p).x }")
            .unwrap(),
        7
    );
}

#[test]
fn small_struct_return() {
    assert_eq!(
        common::run("struct Point { x: int32, y: int32 } fn make(x: int32, y: int32): Point { Point { x: x, y: y } } fn main(): int32 { let p = make(5, 6); p.y }")
            .unwrap(),
        6
    );
}

#[test]
fn large_struct_param_before_scalar_param() {
    assert_eq!(
        common::run("struct Test { a: int32, b: int32, c: int32 } fn test(value: Test, extra: int32): int32 { value.a + value.b + value.c + extra } fn main(): int32 { let value = Test { a: 10, b: 20, c: 30 }; test(value, 9) }")
            .unwrap(),
        69
    );
}

#[test]
fn large_struct_field_access() {
    assert_eq!(
        common::run("struct Triple { a: int32, b: int32, c: int32 } fn main(): int32 { let t = Triple { a: 1, b: 2, c: 3 }; t.a + t.b + t.c }")
            .unwrap(),
        6
    );
}

#[test]
fn large_tuple_field_access() {
    assert_eq!(
        common::run("fn main(): int32 { let t = (1, 2, 3); t.0 + t.1 + t.2 }").unwrap(),
        6
    );
}

#[test]
fn nested_struct_field_access() {
    assert_eq!(
        common::run("struct Inner { value: int32 } struct Outer { inner: Inner, extra: int32 } fn main(): int32 { let o = Outer { inner: Inner { value: 42 }, extra: 10 }; o.inner.value + o.extra }")
            .unwrap(),
        52
    );
}

#[test]
fn small_struct_whole_assignment() {
    assert_eq!(
        common::run("struct Point { x: int32, y: int32 } fn main(): int32 { let mut a = Point { x: 10, y: 20 }; let mut b = Point { x: 1, y: 2 }; b = a; b.x + b.y }")
            .unwrap(),
        30
    );
}

#[test]
fn two_chunk_struct_whole_assignment() {
    assert_eq!(
        common::run("struct Quad { a: int32, b: int32, c: int32, d: int32 } fn main(): int32 { let mut a = Quad { a: 1, b: 2, c: 3, d: 4 }; let mut b = Quad { a: 9, b: 9, c: 9, d: 9 }; b = a; b.a + b.b + b.c + b.d }")
            .unwrap(),
        10
    );
}

#[test]
fn large_struct_whole_assignment() {
    assert_eq!(
        common::run("struct Triple { a: int32, b: int32, c: int32 } fn main(): int32 { let mut a = Triple { a: 1, b: 2, c: 3 }; let mut b = Triple { a: 9, b: 9, c: 9 }; b = a; b.a + b.b + b.c }")
            .unwrap(),
        6
    );
}

#[test]
fn large_struct_assignment_is_a_copy() {
    assert_eq!(
        common::run("struct Triple { a: int32, b: int32, c: int32 } fn main(): int32 { let mut a = Triple { a: 1, b: 2, c: 3 }; let mut b = Triple { a: 9, b: 9, c: 9 }; b = a; a.a = 100; b.a }")
            .unwrap(),
        1
    );
}
