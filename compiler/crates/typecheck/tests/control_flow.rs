mod common;

#[test]
fn loop_basic() {
    let source = "fn main() { loop { break; } }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn loop_with_continue() {
    let source = "fn main() { loop { continue; break; } }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn loop_with_let() {
    let source = "fn main() { loop { let x = 1; break; } }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn break_outside_loop() {
    let source = "fn main() { break; }";
    let items = common::compile(source);
    assert!(items.is_err(), "typeck should fail: break outside loop");
}

#[test]
fn continue_outside_loop() {
    let source = "fn main() { continue; }";
    let items = common::compile(source);
    assert!(items.is_err(), "typeck should fail: continue outside loop");
}

#[test]
fn nested_loops() {
    let source = "fn main() { loop { loop { break; } break; } }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn else_if_expression() {
    let source = "fn main(): int { if false { 1 } else if true { 2 } else { 3 } }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn ref_parameter_requires_reference_argument() {
    let source = "fn oof(param: &int) { param * 2 } fn main(): int { let mut x = 10; oof(x); x }";
    let errors = common::compile(source).expect_err("typeck should require `&`");
    assert!(
        errors
            .iter()
            .any(|error| error.to_string().contains("must be a reference"))
    );
}

#[test]
fn break_in_if_inside_loop() {
    let source = "fn main() { loop { if true { break; } } }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn while_basic() {
    let source = "fn main() { let mut x = 0; while x < 10 { let y = x + 1; } }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn while_condition_bool() {
    let source = "fn main() { while true { break; } }";
    let items = common::compile(source);
    assert!(items.is_ok(), "typeck should succeed: {:?}", items.err());
}

#[test]
fn while_condition_not_bool() {
    let source = "fn main() { while 1 { break; } }";
    let items = common::compile(source);
    assert!(
        items.is_err(),
        "typeck should fail: while condition must be bool"
    );
}

#[test]
fn while_condition_string() {
    let source = "fn main() { while \"hello\" { break; } }";
    let items = common::compile(source);
    assert!(
        items.is_err(),
        "typeck should fail: while condition must be bool"
    );
}

// must update to return only 1 warning, or merge the two into one if the spans are similar.
#[test]
fn unreachable_after_return() {
    let source = "fn main(): int32 { return 1; let x = 2; x }";
    let (result, warnings) = common::compile_with_warnings(source);
    assert!(result.is_ok(), "typeck should succeed: {:?}", result.err());
    assert_eq!(warnings.len(), 2, "should warn about `let x = 2;` and `x`");
}

#[test]
fn unreachable_after_break() {
    let source = "fn main() { loop { break; let x = 1; } }";
    let (result, warnings) = common::compile_with_warnings(source);
    assert!(result.is_ok(), "typeck should succeed: {:?}", result.err());
    assert_eq!(warnings.len(), 1, "should warn about `let x = 1;`");
}

#[test]
fn unreachable_after_continue() {
    let source = "fn main() { loop { continue; let x = 1; } }";
    let (result, warnings) = common::compile_with_warnings(source);
    assert!(result.is_ok(), "typeck should succeed: {:?}", result.err());
    assert_eq!(warnings.len(), 1, "should warn about `let x = 1;`");
}

#[test]
fn no_warning_when_reachable() {
    let source = "fn main() { loop { break; } let x = 1; }";
    let (result, warnings) = common::compile_with_warnings(source);
    assert!(result.is_ok(), "typeck should succeed: {:?}", result.err());
    assert_eq!(warnings.len(), 0, "no unreachable code");
}

#[test]
fn unreachable_in_nested_block() {
    let source = "fn main(): int32 { loop { break; { let x = 1; } } 1 }";
    let (result, warnings) = common::compile_with_warnings(source);
    assert!(result.is_ok(), "typeck should succeed: {:?}", result.err());
    assert_eq!(
        warnings.len(),
        1,
        "should warn about the nested block expression"
    );
}

#[test]
fn no_warning_break_in_nested_if() {
    let source = "fn main() { loop { if true { break; } let x = 1; } }";
    let (result, warnings) = common::compile_with_warnings(source);
    assert!(result.is_ok(), "typeck should succeed: {:?}", result.err());
    assert_eq!(
        warnings.len(),
        0,
        "break inside if does not make subsequent code unreachable"
    );
}

#[test]
fn multiple_unreachable_statements() {
    let source = "fn main() { loop { break; let x = 1; let y = 2; } }";
    let (result, warnings) = common::compile_with_warnings(source);
    assert!(result.is_ok(), "typeck should succeed: {:?}", result.err());
    assert_eq!(warnings.len(), 2, "should warn about both statements");
}
