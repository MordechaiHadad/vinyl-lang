mod common;

#[test]
fn enum_none_eq_none() {
    assert_eq!(
        common::run("enum Option { None, Some(int32) } fn main(): int32 { if Option::None() == Option::None() { 1 } else { 0 } }")
            .unwrap(),
        1
    );
}

#[test]
fn enum_none_ne_some() {
    assert_eq!(
        common::run("enum Option { None, Some(int32) } fn main(): int32 { if Option::None() == Option::Some(42) { 1 } else { 0 } }")
            .unwrap(),
        0
    );
}

#[test]
fn enum_some_eq_same_value() {
    assert_eq!(
        common::run("enum Option { None, Some(int32) } fn main(): int32 { if Option::Some(10) == Option::Some(10) { 1 } else { 0 } }")
            .unwrap(),
        1
    );
}

#[test]
fn enum_some_ne_different_value() {
    assert_eq!(
        common::run("enum Option { None, Some(int32) } fn main(): int32 { if Option::Some(10) == Option::Some(20) { 1 } else { 0 } }")
            .unwrap(),
        0
    );
}

#[test]
fn enum_none_eq_none_via_var() {
    assert_eq!(
        common::run("enum Option { None, Some(int32) } fn main(): int32 { let x = Option::None(); if x == Option::None() { 1 } else { 0 } }")
            .unwrap(),
        1
    );
}

#[test]
fn enum_some_eq_same_via_var() {
    assert_eq!(
        common::run("enum Option { None, Some(int32) } fn main(): int32 { let x = Option::Some(42); if x == Option::Some(42) { 1 } else { 0 } }")
            .unwrap(),
        1
    );
}

#[test]
fn enum_multi_variant_eq() {
    assert_eq!(
        common::run("enum Color { Red, Green, Blue } fn main(): int32 { if Color::Red() == Color::Red() { 1 } else { 0 } }")
            .unwrap(),
        1
    );
}

#[test]
fn enum_multi_variant_ne() {
    assert_eq!(
        common::run("enum Color { Red, Green, Blue } fn main(): int32 { if Color::Red() == Color::Green() { 1 } else { 0 } }")
            .unwrap(),
        0
    );
}
