mod common;

#[test]
fn large_struct_equality_same_values() {
    assert_eq!(
        common::run("struct Test { a: int32, b: int32, c: int32 } fn main(): int32 { let a = Test { a: 1, b: 2, c: 3 }; let b = Test { a: 1, b: 2, c: 3 }; if a == b { 1 } else { 0 } }")
            .unwrap(),
        1
    );
}

#[test]
fn large_struct_equality_different_values() {
    assert_eq!(
        common::run("struct Test { a: int32, b: int32, c: int32 } fn main(): int32 { let a = Test { a: 1, b: 2, c: 3 }; let b = Test { a: 9, b: 2, c: 3 }; if a == b { 1 } else { 0 } }")
            .unwrap(),
        0
    );
}

#[test]
fn small_struct_equality_with_padding() {
    assert_eq!(
        common::run("struct Mix { a: int8, b: int64 } fn main(): int32 { let a = Mix { a: 1, b: 100 }; let b = Mix { a: 1, b: 100 }; if a == b { 1 } else { 0 } }")
            .unwrap(),
        1
    );
}

#[test]
fn array_of_structs_index() {
    assert_eq!(
        common::run("struct Triple { a: int32, b: int32, c: int32 } fn main(): int32 { let arr = [Triple { a: 1, b: 2, c: 3 }, Triple { a: 4, b: 5, c: 6 }]; arr[1].b }")
            .unwrap(),
        5
    );
}

#[test]
fn array_of_structs_last_field() {
    assert_eq!(
        common::run("struct Triple { a: int32, b: int32, c: int32 } fn main(): int32 { let arr = [Triple { a: 1, b: 2, c: 3 }, Triple { a: 4, b: 5, c: 6 }]; arr[1].c }")
            .unwrap(),
        6
    );
}

#[test]
fn array_of_tuples_stride() {
    assert_eq!(
        common::run("fn main(): int32 { let arr = [(1, 2, 3), (4, 5, 6)]; arr[1].2 }").unwrap(),
        6
    );
}

#[test]
fn main_returning_large_struct_is_error() {
    assert!(common::run("struct Big { a: int32, b: int32, c: int32 } fn main(): Big { Big { a: 1, b: 2, c: 3 } }").is_err());
}

#[test]
fn infinite_recursive_struct_is_rejected() {
    assert!(common::run("struct A { b: B } struct B { a: A } fn main(): int32 { 0 }").is_err());
}

#[test]
fn packed_struct_field_not_clobbered_by_aggregate_neighbor() {
    assert_eq!(
        common::run("struct Inner { a: int16, b: int16 } struct Outer { y: int8, i: Inner } fn main(): int32 { let o = Outer { y: 7, i: Inner { a: 1, b: 2 } }; if o.y == 7 { 1 } else { 0 } }")
            .unwrap(),
        1
    );
}

#[test]
fn large_enum_equality_with_padding_is_deterministic() {
    assert_eq!(
        common::run("enum Big { A(int32, int32, int32, int32, int32) } fn main(): int32 { let x = Big::A(1, 2, 3, 4, 5); let y = Big::A(1, 2, 3, 4, 5); if x == y { 1 } else { 0 } }")
            .unwrap(),
        1
    );
}

#[test]
fn two_chunk_enum_equality_is_deterministic() {
    assert_eq!(
        common::run("enum Big { A(int32, int32, int32) } fn main(): int32 { let x = Big::A(1, 2, 3); let y = Big::A(1, 2, 3); if x == y { 1 } else { 0 } }")
            .unwrap(),
        1
    );
}

#[test]
fn two_chunk_struct_field_embedding_does_not_overflow() {
    assert_eq!(
        common::run("struct Mid { a: int32, b: int32, c: int32 } struct Outer { m: Mid, b: int32 } fn main(): int32 { let o = Outer { m: Mid { a: 1, b: 2, c: 3 }, b: 7 }; if o.b == 7 && o.m.c == 3 { 1 } else { 0 } }")
            .unwrap(),
        1
    );
}

#[test]
fn two_chunk_struct_survives_function_return() {
    assert_eq!(
        common::run("struct Mid { a: int32, b: int32, c: int32 } struct Outer { m: Mid, b: int32 } fn make(x: int32): Outer { Outer { m: Mid { a: x, b: x + 1, c: x + 2 }, b: x + 3 } } fn main(): int32 { let o = make(1); if o.b == 4 && o.m.c == 3 { 1 } else { 0 } }")
            .unwrap(),
        1
    );
}

#[test]
fn padded_tuple_equality_is_deterministic() {
    assert_eq!(
        common::run(
            "fn main(): int32 { let a = (1, 100); let b = (1, 100); if a == b { 1 } else { 0 } }"
        )
        .unwrap(),
        1
    );
}

#[test]
fn small_array_equality_same_elements() {
    assert_eq!(
        common::run(
            "fn main(): int32 { let a = [1, 2]; let b = [1, 2]; if a == b { 1 } else { 0 } }"
        )
        .unwrap(),
        1
    );
}

#[test]
fn small_array_equality_different_elements() {
    assert_eq!(
        common::run(
            "fn main(): int32 { let a = [1, 2]; let b = [1, 3]; if a == b { 1 } else { 0 } }"
        )
        .unwrap(),
        0
    );
}

#[test]
fn small_array_inequality() {
    assert_eq!(
        common::run(
            "fn main(): int32 { let a = [1, 2]; let b = [1, 3]; if a != b { 1 } else { 0 } }"
        )
        .unwrap(),
        1
    );
}

#[test]
fn packed_small_struct_float_equality_is_ieee() {
    assert_eq!(
        common::run("struct P { a: float32, b: float32 } fn main(): int32 { let a = P { a: 0.0, b: 1.0 }; let b = P { a: -0.0, b: 1.0 }; if a == b { 1 } else { 0 } }")
            .unwrap(),
        1
    );
}

#[test]
fn packed_small_struct_float_inequality() {
    assert_eq!(
        common::run("struct P { a: float32, b: float32 } fn main(): int32 { let a = P { a: 0.0, b: 1.0 }; let b = P { a: 0.5, b: 1.0 }; if a == b { 1 } else { 0 } }")
            .unwrap(),
        0
    );
}

#[test]
fn float_tuple_equality_is_ieee() {
    assert_eq!(
        common::run("fn main(): int32 { let a = (0.0, 1.0); let b = (-0.0, 1.0); if a == b { 1 } else { 0 } }")
            .unwrap(),
        1
    );
}

#[test]
fn two_chunk_float_struct_equality_is_ieee() {
    assert_eq!(
        common::run("struct T { a: float32, b: float32, c: float32 } fn main(): int32 { let a = T { a: 0.0, b: 0.0, c: 0.0 }; let b = T { a: -0.0, b: 0.0, c: 0.0 }; if a == b { 1 } else { 0 } }")
            .unwrap(),
        1
    );
}

#[test]
fn float_array_equality_is_ieee() {
    assert_eq!(
        common::run("fn main(): int32 { let a: [float64; 2] = [0.0, 1.0]; let b: [float64; 2] = [-0.0, 1.0]; if a == b { 1 } else { 0 } }")
            .unwrap(),
        1
    );
}

#[test]
fn enum_float_payload_equality_same_values() {
    assert_eq!(
        common::run("enum E { A(float32), B } fn main(): int32 { if E::A(0.0) == E::A(-0.0) { 1 } else { 0 } }")
            .unwrap(),
        1
    );
}

#[test]
fn enum_float_payload_equality_different_values() {
    assert_eq!(
        common::run("enum E { A(float32), B } fn main(): int32 { if E::A(1.0) == E::A(2.0) { 1 } else { 0 } }")
            .unwrap(),
        0
    );
}

#[test]
fn enum_float_payload_equality_different_variant() {
    assert_eq!(
        common::run(
            "enum E { A(float32), B } fn main(): int32 { if E::A(1.0) == E::B() { 1 } else { 0 } }"
        )
        .unwrap(),
        0
    );
}

#[test]
fn struct_float_nan_equality_is_false() {
    assert_eq!(
        common::run("struct F { x: float } fn main(): int32 { let a = F { x: 0.0 / 0.0 }; let b = F { x: 0.0 / 0.0 }; if a == b { 1 } else { 0 } }")
            .unwrap(),
        0
    );
}
