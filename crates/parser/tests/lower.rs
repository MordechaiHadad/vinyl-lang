use vinyl_parser::ast::*;

mod common;

#[test]
fn function_def() {
    let items = common::do_lower("fn add(a: int32, b: int32): int64 { a + b }");
    assert_eq!(items.len(), 1);
    let func = match &items[0] {
        Item::Function(f) => f,
        _ => panic!("expected function"),
    };
    assert_eq!(func.name, "add");
    assert_eq!(func.params.len(), 2);
    assert_eq!(func.params[0].name, "a");
    assert_eq!(func.params[1].name, "b");
    assert!(func.return_type.is_some());
}

#[test]
fn function_no_return_type() {
    let items = common::do_lower("fn main() {}");
    let func = match &items[0] {
        Item::Function(f) => f,
        _ => panic!("expected function"),
    };
    assert!(func.return_type.is_none());
    assert!(func.params.is_empty());
}

#[test]
fn mut_param() {
    let items = common::do_lower("fn inc(mut x: int32) {}");
    let func = match &items[0] {
        Item::Function(f) => f,
        _ => panic!("expected function"),
    };
    assert_eq!(func.params[0].name, "x");
}

#[test]
fn let_statements() {
    let items =
        common::do_lower("fn f() { let x: int32 = 42; let y = 10; let mut z: float64 = 3.14; }");
    let func = match &items[0] {
        Item::Function(f) => f,
        _ => panic!("expected function"),
    };
    assert_eq!(func.body.len(), 3);

    if let Stmt::Let {
        name,
        mutable,
        type_,
        ..
    } = &func.body[0]
    {
        assert_eq!(name, "x");
        assert!(!mutable);
        assert!(type_.is_some());
    } else {
        panic!("expected let statement");
    }

    if let Stmt::Let {
        name,
        mutable,
        type_,
        ..
    } = &func.body[1]
    {
        assert_eq!(name, "y");
        assert!(!mutable);
        assert!(type_.is_none());
    } else {
        panic!("expected let statement");
    }

    if let Stmt::Let {
        name,
        mutable,
        type_,
        ..
    } = &func.body[2]
    {
        assert_eq!(name, "z");
        assert!(!*mutable);
        assert!(type_.is_some());
    } else {
        panic!("expected let statement");
    }
}

#[test]
fn literal_values() {
    let items = common::do_lower(
        "fn f() { let a = 42; let b = 3.14; let c = true; let d = 'x'; let e = \"hello\"; }",
    );
    let func = match &items[0] {
        Item::Function(f) => f,
        _ => panic!("expected function"),
    };

    let check_int = |idx| {
        if let Stmt::Let {
            value: Expr::Int(v, _),
            ..
        } = &func.body[idx]
        {
            assert_eq!(*v, 42);
        } else {
            panic!("expected int literal at {idx}");
        }
    };

    let check_float = |idx| {
        if let Stmt::Let {
            value: Expr::Float(v, _),
            ..
        } = &func.body[idx]
        {
            assert!((*v - (314.0_f64 / 100.0)).abs() < 1e-10);
        } else {
            panic!("expected float literal at {idx}");
        }
    };

    let check_bool = |idx, expected| {
        if let Stmt::Let {
            value: Expr::Bool(v, _),
            ..
        } = &func.body[idx]
        {
            assert_eq!(*v, expected);
        } else {
            panic!("expected bool literal at {idx}");
        }
    };

    let check_char = |idx| {
        if let Stmt::Let {
            value: Expr::Char(v, _),
            ..
        } = &func.body[idx]
        {
            assert_eq!(*v, 'x');
        } else {
            panic!("expected char literal at {idx}");
        }
    };

    let check_string = |idx| {
        if let Stmt::Let {
            value: Expr::String(v, _),
            ..
        } = &func.body[idx]
        {
            assert_eq!(v, "hello");
        } else {
            panic!("expected string literal at {idx}");
        }
    };

    check_int(0);
    check_float(1);
    check_bool(2, true);
    check_char(3);
    check_string(4);
}

#[test]
fn binary_expression_structure() {
    let items = common::do_lower("fn f(): int32 { 1 + 2 }");
    let func = match &items[0] {
        Item::Function(f) => f,
        _ => panic!("expected function"),
    };

    let last = func.body.last().unwrap();
    if let Stmt::Value(
        Expr::Binary {
            left, op, right, ..
        },
        _,
    ) = last
    {
        assert_eq!(op, &BinaryOp::Add);
        if let Expr::Int(lv, _) = left.as_ref() {
            assert_eq!(*lv, 1);
        } else {
            panic!("expected int literal as left operand");
        }
        if let Expr::Int(rv, _) = right.as_ref() {
            assert_eq!(*rv, 2);
        } else {
            panic!("expected int literal as right operand");
        }
    } else {
        panic!("expected binary expression");
    }
}

#[test]
fn if_expression() {
    let items = common::do_lower("fn f(): int32 { if true { 1 } else { 2 } }");
    let func = match &items[0] {
        Item::Function(f) => f,
        _ => panic!("expected function"),
    };

    let last = func.body.last().unwrap();
    let expr = match last {
        Stmt::Value(e, _) => e,
        _ => panic!("expected value statement"),
    };
    match expr {
        Expr::If {
            condition,
            then_block,
            else_if,
            else_block,
            ..
        } => {
            assert!(matches!(condition.as_ref(), Expr::Bool(true, _)));
            assert!(!then_block.is_empty());
            assert!(else_if.is_empty());
            assert!(else_block.is_some());
        }
        _ => panic!("expected if expression"),
    }
}

#[test]
fn attributes() {
    let items = common::do_lower("@deprecated\n@inline(always)\nfn old_function() {}");
    let func = match &items[0] {
        Item::Function(f) => f,
        _ => panic!("expected function"),
    };
    assert_eq!(func.attrs.len(), 2);
    assert_eq!(func.attrs[0].name, "deprecated");
    assert_eq!(func.attrs[1].name, "inline");
}

#[test]
fn array_expression() {
    let items = common::do_lower("fn f() { let a = [1, 2, 3]; }");
    let func = match &items[0] {
        Item::Function(f) => f,
        _ => panic!("expected function"),
    };
    if let Stmt::Let {
        value: Expr::Array(elements, _),
        ..
    } = &func.body[0]
    {
        assert_eq!(elements.len(), 3);
    } else {
        panic!("expected array expression");
    }
}

#[test]
fn while_to_loop() {
    let items = common::do_lower("fn f() { while true { break; } }");
    let func = match &items[0] {
        Item::Function(f) => f,
        _ => panic!("expected function"),
    };
    assert_eq!(func.body.len(), 1);
    match &func.body[0] {
        Stmt::Loop { body, .. } => {
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
        Stmt::Loop { body, .. } => {
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::Break(_) => {}
                _ => panic!("expected break statement"),
            }
        }
        _ => panic!("expected loop statement"),
    }
}

#[test]
fn return_statement() {
    let items = common::do_lower("fn f(): int32 { return 42; }");
    let func = match &items[0] {
        Item::Function(f) => f,
        _ => panic!("expected function"),
    };
    match &func.body[0] {
        Stmt::Return(Some(Expr::Int(v, _)), _) => {
            assert_eq!(*v, 42);
        }
        other => panic!("expected return with int, got {:?}", other),
    }
}

#[test]
fn return_void() {
    let items = common::do_lower("fn f() { return; }");
    let func = match &items[0] {
        Item::Function(f) => f,
        _ => panic!("expected function"),
    };
    match &func.body[0] {
        Stmt::Return(None, _) => {}
        other => panic!("expected return without value, got {:?}", other),
    }
}

#[test]
fn primitive_types() {
    let items = common::do_lower(
        "fn f(x: int8, y: uint16, z: float32, w: string, c: char, b: bool, u: unit) {}",
    );
    let func = match &items[0] {
        Item::Function(f) => f,
        _ => panic!("expected function"),
    };

    let expected = [
        (Primitive::Int8, "int8"),
        (Primitive::UInt16, "uint16"),
        (Primitive::Float32, "float32"),
        (Primitive::String, "string"),
        (Primitive::Char, "char"),
        (Primitive::Bool, "bool"),
        (Primitive::Unit, "unit"),
    ];

    for (i, (expected_prim, expected_name)) in expected.iter().enumerate() {
        let param_type = &func.params[i].type_;
        assert_eq!(
            *param_type,
            Type::Primitive(expected_prim.clone()),
            "expected {expected_name} at param {i}, got {param_type:?}"
        );
    }
}

#[test]
fn array_type() {
    let items = common::do_lower("fn f(a: [int32; 5]) {}");
    let func = match &items[0] {
        Item::Function(f) => f,
        _ => panic!("expected function"),
    };
    match &func.params[0].type_ {
        Type::Array { element, size } => {
            assert_eq!(**element, Type::Primitive(Primitive::Int32));
            assert_eq!(*size, 5);
        }
        other => panic!("expected array type, got {other:?}"),
    }
}

#[test]
fn generic_type() {
    let items = common::do_lower("fn f(a: Option<int32>) {}");
    let func = match &items[0] {
        Item::Function(f) => f,
        _ => panic!("expected function"),
    };
    match &func.params[0].type_ {
        Type::Generic { name, args } => {
            assert_eq!(name, "Option");
            assert_eq!(args.len(), 1);
            assert_eq!(args[0], Type::Primitive(Primitive::Int32));
        }
        other => panic!("expected generic type, got {other:?}"),
    }
}

#[test]
fn multiple_functions() {
    let items = common::do_lower("fn foo() {} fn bar() {} fn baz() {}");
    assert_eq!(items.len(), 3);
    for (i, name) in ["foo", "bar", "baz"].iter().enumerate() {
        let f = match &items[i] {
            Item::Function(f) => f,
            _ => panic!("expected function"),
        };
        assert_eq!(f.name, *name);
    }
}

#[test]
fn hex_int_literal() {
    let items = common::do_lower("fn f() { let a = 0xFF; }");
    let func = match &items[0] {
        Item::Function(f) => f,
        _ => panic!("expected function"),
    };
    if let Stmt::Let {
        value: Expr::Int(v, _),
        ..
    } = &func.body[0]
    {
        assert_eq!(*v, 255);
    } else {
        panic!("expected int literal");
    }
}

#[test]
fn binary_int_literal() {
    let items = common::do_lower("fn f() { let a = 0b1010; }");
    let func = match &items[0] {
        Item::Function(f) => f,
        _ => panic!("expected function"),
    };
    if let Stmt::Let {
        value: Expr::Int(v, _),
        ..
    } = &func.body[0]
    {
        assert_eq!(*v, 10);
    } else {
        panic!("expected int literal");
    }
}

#[test]
fn negative_int_literal() {
    let items = common::do_lower("fn f() { let a = -42; }");
    let func = match &items[0] {
        Item::Function(f) => f,
        _ => panic!("expected function"),
    };
    if let Stmt::Let {
        value: Expr::Int(v, _),
        ..
    } = &func.body[0]
    {
        assert_eq!(*v, -42);
    } else {
        panic!("expected int literal");
    }
}

#[test]
fn raw_string() {
    let items = common::do_lower("fn f() { let a = r\"hello\\nworld\"; }");
    let func = match &items[0] {
        Item::Function(f) => f,
        _ => panic!("expected function"),
    };
    if let Stmt::Let {
        value: Expr::String(v, _),
        ..
    } = &func.body[0]
    {
        assert_eq!(v, "hello\\nworld");
    } else {
        panic!("expected string literal");
    }
}

#[test]
fn char_literal() {
    let items = common::do_lower("fn f() { let a = 'a'; }");
    let func = match &items[0] {
        Item::Function(f) => f,
        _ => panic!("expected function"),
    };
    if let Stmt::Let {
        value: Expr::Char(v, _),
        ..
    } = &func.body[0]
    {
        assert_eq!(*v, 'a');
    } else {
        panic!("expected char literal");
    }
}

#[test]
fn int_float_type_aliases() {
    let items = common::do_lower("fn f(x: int, y: float) {}");
    let func = match &items[0] {
        Item::Function(f) => f,
        _ => panic!("expected function"),
    };
    assert_eq!(func.params[0].type_, Type::Primitive(Primitive::Int64));
    assert_eq!(func.params[1].type_, Type::Primitive(Primitive::Float64));
}

#[test]
fn unit_literal_expression() {
    let items = common::do_lower("fn f() { let x = unit; }");
    let func = match &items[0] {
        Item::Function(f) => f,
        _ => panic!("expected function"),
    };
    assert!(matches!(func.body[0], Stmt::Let { value: Expr::Unit(_), .. }));
}
