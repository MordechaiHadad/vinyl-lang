mod common;

#[test]
fn alias_construct_and_read() {
    let source = "struct X { a: int32, b: int32 } type Y = X; fn f(v: Y): int32 { v.a } fn main(): int32 { let p = Y { a: 1, b: 2 }; f(p) }";
    assert!(common::compile(source).is_ok());
}

#[test]
fn alias_is_distinct_from_target() {
    let source = "struct X { a: int32 } type Y = X; fn f(p: Y): int32 { p.a } fn main(): int32 { let x = X { a: 5 }; f(x) }";
    assert!(
        common::compile(source).is_err(),
        "passing the original type where the alias is expected must fail"
    );
}

#[test]
fn alias_distinct_from_other_alias_of_same_target() {
    let source = "struct X { a: int32 } type Y = X; type Z = X; fn f(p: Y): int32 { p.a } fn main(): int32 { let z = Z { a: 5 }; f(z) }";
    assert!(
        common::compile(source).is_err(),
        "two aliases of the same target must not unify"
    );
}

#[test]
fn alias_rejects_incompatible_literal() {
    let source = "struct X { a: int32 } type Y = X; fn f(p: Y): int32 { p.a } fn main(): int32 { let n = 42; f(n) }";
    assert!(common::compile(source).is_err());
}

#[test]
fn alias_unknown_target_fails() {
    let source = "type Y = Missing; fn main() {}";
    assert!(common::compile(source).is_err());
}

#[test]
fn alias_cycle_fails() {
    let source = "type A = B; type B = A; fn main() {}";
    assert!(common::compile(source).is_err());
}

#[test]
fn alias_self_cycle_fails() {
    let source = "type A = A; fn main() {}";
    assert!(common::compile(source).is_err());
}

#[test]
fn alias_hiding_infinite_recursive_struct_fails() {
    let source = "type X = S; struct S { f: X } fn main(): int32 { 0 }";
    assert!(
        common::compile(source).is_err(),
        "recursion reached through an alias must be rejected"
    );
}

#[test]
fn alias_chain_infinite_recursive_struct_fails() {
    let source = "type X = Y; type Y = S; struct S { f: X } fn main(): int32 { 0 }";
    assert!(
        common::compile(source).is_err(),
        "recursion reached through an alias chain must be rejected"
    );
}

#[test]
fn alias_to_non_recursive_struct_is_ok() {
    let source =
        "struct Inner { a: int32 } type X = Inner; struct Outer { f: X } fn main(): int32 { 0 }";
    assert!(
        common::compile(source).is_ok(),
        "alias to a finite type must not be rejected"
    );
}

#[test]
fn alias_field_access_via_alias() {
    let source = "struct X { a: int32 } type Y = X; fn main(): int32 { let p = Y { a: 3 }; p.a }";
    assert!(common::compile(source).is_ok());
}
