use vinyl_parser::ast::{item::Item, statement::Statement};

#[path = "../common/mod.rs"]
mod common;

#[test]
fn while_to_loop() {
    let items = common::do_lower("fn f() { while true { break; } }");
    let func = match &items[0] {
        Item::Function(f) => f,
        _ => panic!("expected function"),
    };
    assert_eq!(func.body.len(), 1);
    match &func.body[0] {
        Statement::Loop { body, .. } => {
            assert!(!body.is_empty());
        }
        _ => panic!("expected loop statement"),
    }
}

#[test]
fn loop_statement() {
    let items = common::do_lower("fn f() { loop { break; } }");
    let func = match &items[0] {
        Item::Function(f) => f,
        _ => panic!("expected function"),
    };
    assert_eq!(func.body.len(), 1);
    match &func.body[0] {
        Statement::Loop { body, .. } => {
            assert_eq!(body.len(), 1);
            match &body[0] {
                Statement::Break(_) => {}
                _ => panic!("expected break statement"),
            }
        }
        _ => panic!("expected loop statement"),
    }
}
