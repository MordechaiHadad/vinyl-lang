mod common;

#[test]
fn tuple_struct_construction_and_field_access() {
    assert_eq!(
        common::run("tuple Pair(int32, int32) fn main(): int32 { let p = Pair(10, 20); p.0 + p.1 }")
            .unwrap(),
        30
    );
}

#[test]
fn tuple_struct_construction_large() {
    assert_eq!(
        common::run("tuple Five(int32, int32, int32, int32, int32) fn main(): int32 { let p = Five(1, 2, 3, 4, 5); p.0 + p.1 + p.2 + p.3 + p.4 }")
            .unwrap(),
        15
    );
}

#[test]
fn tuple_struct_construction_two_chunk() {
    assert_eq!(
        common::run("tuple Four(int32, int32, int32, int32) fn main(): int32 { let p = Four(1, 2, 3, 4); p.0 + p.1 + p.2 + p.3 }")
            .unwrap(),
        10
    );
}

#[test]
fn tuple_struct_equality() {
    assert_eq!(
        common::run("tuple Pair(int32, int32) fn main(): int32 { let a = Pair(1, 2); let b = Pair(1, 2); if a == b { 1 } else { 0 } }")
            .unwrap(),
        1
    );
}

#[test]
fn tuple_struct_equality_different() {
    assert_eq!(
        common::run("tuple Pair(int32, int32) fn main(): int32 { let a = Pair(1, 2); let b = Pair(1, 3); if a == b { 1 } else { 0 } }")
            .unwrap(),
        0
    );
}

#[test]
fn tuple_struct_passed_to_function() {
    assert_eq!(
        common::run("tuple Foo(int32) fn id(x: Foo): int32 { x.0 } fn main(): int32 { let f = Foo(7); id(f) }")
            .unwrap(),
        7
    );
}

#[test]
fn tuple_struct_returned_from_function() {
    assert_eq!(
        common::run("tuple Foo(int32, int32) fn make(): Foo { Foo(3, 4) } fn main(): int32 { make().1 }")
            .unwrap(),
        4
    );
}

#[test]
fn tuple_struct_whole_assignment() {
    assert_eq!(
        common::run("tuple Three(int32, int32, int32) fn main(): int32 { let mut a = Three(1, 2, 3); let mut b = Three(9, 9, 9); b = a; b.0 + b.1 + b.2 }")
            .unwrap(),
        6
    );
}
