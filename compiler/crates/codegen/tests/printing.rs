mod common;

#[test]
fn println_replaces_main_return_output() {
    assert_eq!(common::run("fn main() { println(69); }").unwrap(), 69);
}
