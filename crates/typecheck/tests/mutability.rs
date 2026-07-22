mod common;

#[test]
fn assign_to_immutable_var() {
    let source = "fn main() { let x = 5; x = 10; }";
    let result = common::compile(source);
    assert!(result.is_err(), "should reject assignment to immutable var");
}

#[test]
fn assign_to_immutable_ref() {
    let source = "fn main() { let x = 5; let y = &x; y = 10; }";
    let result = common::compile(source);
    assert!(
        result.is_err(),
        "should reject write through immutable ref binding"
    );
}

#[test]
fn function_return_ref() {
    let source = "fn get_ref(): &int32 { let x = 5; x }";
    let result = common::compile(source);
    assert!(result.is_err(), "should reject function returning &T");
}

#[test]
fn mutable_var_assign_success() {
    let source = "fn main() { let mut x = 5; x = 10; }";
    let result = common::compile(source);
    assert!(
        result.is_ok(),
        "should allow assignment to mutable var: {:?}",
        result.err()
    );
}

#[test]
fn mutable_ref_assign_success() {
    let source = "fn main() { let mut x = 5; let mut y = &x; y = 10; }";
    let result = common::compile(source);
    assert!(
        result.is_ok(),
        "should allow write through mutable ref binding: {:?}",
        result.err()
    );
}

#[test]
fn ref_to_immutable_var() {
    let source = "fn main() { let x = 10; let y = &x; }";
    let result = common::compile(source);
    assert!(
        result.is_ok(),
        "should allow ref to immutable var: {:?}",
        result.err()
    );
}

#[test]
fn assign_readonly_ref() {
    let source = "fn main() { let x = 10; let y = &x; y = 5; }";
    let result = common::compile(source);
    assert!(
        result.is_err(),
        "should reject writing through readonly ref"
    );
}

#[test]
fn compound_assign_immutable() {
    let source = "fn main() { let x = 5; x += 3; }";
    let result = common::compile(source);
    assert!(
        result.is_err(),
        "should reject compound assign to immutable var"
    );
}

#[test]
fn write_through_ref_param() {
    let source = "fn foo(p: &int32) { p = 10; }";
    let result = common::compile(source);
    assert!(
        result.is_ok(),
        "should allow write through ref param: {:?}",
        result.err()
    );
}

#[test]
fn pass_immutable_to_ref_param() {
    let source = "fn foo(p: &int32) {} fn main() { let x = 5; foo(&x); }";
    let result = common::compile(source);
    assert!(
        result.is_err(),
        "should reject passing immutable to ref param"
    );
}

#[test]
fn pass_mutable_to_ref_param() {
    let source = "fn foo(p: &int32) {} fn main() { let mut x = 5; foo(&x); }";
    let result = common::compile(source);
    assert!(
        result.is_ok(),
        "should allow passing mutable to ref param: {:?}",
        result.err()
    );
}

#[test]
fn reject_ref_to_index_element() {
    let source = "fn main() { let arr = [1, 2, 3]; let y = &arr[0]; }";
    let result = common::compile(source);
    assert!(result.is_err(), "should reject taking ref to index element");
}

#[test]
fn reject_ref_escaping_inner_scope() {
    let source =
        "fn main() { let outer = 10; let mut x = &outer; { let inner = 20; x = &inner; } }";
    let result = common::compile(source);
    assert!(result.is_err(), "should reject ref escaping inner scope");
}

#[test]
fn ref_to_same_scope_allowed() {
    let source = "fn main() { let a = 10; let b = 20; let mut x = &a; { x = &b; } }";
    let result = common::compile(source);
    assert!(
        result.is_ok(),
        "should allow ref to same-scope var: {:?}",
        result.err()
    );
}

#[test]
fn let_ref_to_index_rejected() {
    let source = "fn main() { let arr = [1, 2]; let x: &int32 = &arr[0]; }";
    let result = common::compile(source);
    assert!(
        result.is_err(),
        "should reject typed let with ref to index element"
    );
}
