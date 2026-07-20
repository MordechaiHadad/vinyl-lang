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
