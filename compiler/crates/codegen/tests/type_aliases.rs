mod common;

#[test]
fn alias_struct_construct_and_field() {
    assert_eq!(
        common::run("struct X { a: int32, b: int32 } type Y = X; fn main(): int32 { let v = Y { a: 1, b: 2 }; v.a + v.b }")
            .unwrap(),
        3
    );
}

#[test]
fn alias_struct_param_and_return() {
    assert_eq!(
        common::run("struct X { a: int32, b: int32 } type Y = X; fn id(p: Y): Y { p } fn main(): int32 { id(Y { a: 7, b: 3 }).a }")
            .unwrap(),
        7
    );
}

#[test]
fn alias_of_alias() {
    assert_eq!(
        common::run("struct X { a: int32 } type Y = X; type Z = Y; fn main(): int32 { let z = Z { a: 9 }; z.a }")
            .unwrap(),
        9
    );
}

#[test]
fn alias_struct_field_assignment() {
    assert_eq!(
        common::run("struct X { a: int32 } type Y = X; fn main(): int32 { let mut v = Y { a: 1 }; v.a = 42; v.a }")
            .unwrap(),
        42
    );
}

#[test]
fn alias_distinct_rejects_original_type() {
    assert!(
        common::run("struct X { a: int32 } type Y = X; fn f(p: Y): int32 { p.a } fn main(): int32 { f(X { a: 5 }) }")
            .is_err()
    );
}

#[test]
fn alias_enum_construction() {
    assert_eq!(
        common::run("enum E { A(int32) } type F = E; fn main(): int32 { let e = F::A(5); if e == F::A(5) { 1 } else { 0 } }")
            .unwrap(),
        1
    );
}

#[test]
fn alias_unknown_target_is_error() {
    assert!(common::run("type Y = Nonexistent; fn main(): int32 { 0 }").is_err());
}

#[test]
fn alias_cycle_is_error() {
    assert!(common::run("type A = B; type B = A; fn main(): int32 { 0 }").is_err());
}
