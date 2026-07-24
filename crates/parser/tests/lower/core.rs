use vinyl_parser::ast::{
    expression::Expression,
    item::{EnumVariantData, Item},
    operator::{BinaryOp, UnaryOp},
    pattern::{LiteralPattern, Pattern},
    statement::Statement,
    types::{Primitive, Type},
};

#[path = "../common/mod.rs"]
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

    if let Statement::Let {
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

    if let Statement::Let {
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

    if let Statement::Let {
        name,
        mutable,
        type_,
        ..
    } = &func.body[2]
    {
        assert_eq!(name, "z");
        assert!(*mutable);
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
        if let Statement::Let {
            value: Expression::Int(v, _),
            ..
        } = &func.body[idx]
        {
            assert_eq!(*v, 42);
        } else {
            panic!("expected int literal at {idx}");
        }
    };

    let check_float = |idx| {
        if let Statement::Let {
            value: Expression::Float(v, _),
            ..
        } = &func.body[idx]
        {
            assert!((*v - (314.0_f64 / 100.0)).abs() < 1e-10);
        } else {
            panic!("expected float literal at {idx}");
        }
    };

    let check_bool = |idx, expected| {
        if let Statement::Let {
            value: Expression::Bool(v, _),
            ..
        } = &func.body[idx]
        {
            assert_eq!(*v, expected);
        } else {
            panic!("expected bool literal at {idx}");
        }
    };

    let check_char = |idx| {
        if let Statement::Let {
            value: Expression::Char(v, _),
            ..
        } = &func.body[idx]
        {
            assert_eq!(*v, 'x');
        } else {
            panic!("expected char literal at {idx}");
        }
    };

    let check_string = |idx| {
        if let Statement::Let {
            value: Expression::String(v, _),
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
    if let Statement::Value(
        Expression::Binary {
            left, op, right, ..
        },
        _,
    ) = last
    {
        assert_eq!(op, &BinaryOp::Add);
        if let Expression::Int(lv, _) = left.as_ref() {
            assert_eq!(*lv, 1);
        } else {
            panic!("expected int literal as left operand");
        }
        if let Expression::Int(rv, _) = right.as_ref() {
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
        Statement::Value(e, _) => e,
        _ => panic!("expected value statement"),
    };
    match expr {
        Expression::If {
            condition,
            then_block,
            else_if,
            else_block,
            ..
        } => {
            assert!(matches!(condition.as_ref(), Expression::Bool(true, _)));
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
    if let Statement::Let {
        value: Expression::Array(elements, _),
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

#[test]
fn return_statement() {
    let items = common::do_lower("fn f(): int32 { return 42; }");
    let func = match &items[0] {
        Item::Function(f) => f,
        _ => panic!("expected function"),
    };
    match &func.body[0] {
        Statement::Return(Some(Expression::Int(v, _)), _) => {
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
        Statement::Return(None, _) => {}
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
    if let Statement::Let {
        value: Expression::Int(v, _),
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
    if let Statement::Let {
        value: Expression::Int(v, _),
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
    if let Statement::Let {
        value: Expression::Int(v, _),
        ..
    } = &func.body[0]
    {
        assert_eq!(*v, -42);
    } else {
        panic!("expected int literal (constant folded from -42)");
    }
}

#[test]
fn raw_string() {
    let items = common::do_lower("fn f() { let a = r\"hello\\nworld\"; }");
    let func = match &items[0] {
        Item::Function(f) => f,
        _ => panic!("expected function"),
    };
    if let Statement::Let {
        value: Expression::String(v, _),
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
    if let Statement::Let {
        value: Expression::Char(v, _),
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
    assert!(matches!(
        func.body[0],
        Statement::Let {
            value: Expression::Unit(_),
            ..
        }
    ));
}

#[test]
fn unary_not_bool() {
    let items = common::do_lower("fn f() { let a = !true; let b = not false; }");
    let func = match &items[0] {
        Item::Function(f) => f,
        _ => panic!("expected function"),
    };
    match &func.body[0] {
        Statement::Let {
            value: Expression::Bool(v, _),
            ..
        } => assert!(!*v),
        _ => panic!("expected folded bool literal (!true = false)"),
    }
    match &func.body[1] {
        Statement::Let {
            value: Expression::Bool(v, _),
            ..
        } => assert!(*v),
        _ => panic!("expected folded bool literal (not false = true)"),
    }
}

#[test]
fn unary_neg_folding() {
    let items = common::do_lower("fn f(): int64 { -42 }");
    let func = match &items[0] {
        Item::Function(f) => f,
        _ => panic!("expected function"),
    };
    let last = func.body.last().unwrap();
    match last {
        Statement::Value(Expression::Int(v, _), _) => assert_eq!(*v, -42),
        _ => panic!("expected folded int literal, got {last:?}"),
    }
}

#[test]
fn unary_not_folding_true() {
    let items = common::do_lower("fn f(): bool { !true }");
    let func = match &items[0] {
        Item::Function(f) => f,
        _ => panic!("expected function"),
    };
    let last = func.body.last().unwrap();
    match last {
        Statement::Value(Expression::Bool(v, _), _) => assert!(!*v),
        _ => panic!("expected folded bool literal, got {last:?}"),
    }
}

#[test]
fn unary_not_folding_false() {
    let items = common::do_lower("fn f(): bool { not false }");
    let func = match &items[0] {
        Item::Function(f) => f,
        _ => panic!("expected function"),
    };
    let last = func.body.last().unwrap();
    match last {
        Statement::Value(Expression::Bool(v, _), _) => assert!(*v),
        _ => panic!("expected folded bool literal, got {last:?}"),
    }
}

#[test]
fn unary_neg_variable() {
    let items = common::do_lower("fn f(x: int64): int64 { -x }");
    let func = match &items[0] {
        Item::Function(f) => f,
        _ => panic!("expected function"),
    };
    let last = func.body.last().unwrap();
    match last {
        Statement::Value(
            Expression::Unary {
                op: UnaryOp::Neg,
                operand,
                ..
            },
            _,
        ) => match operand.as_ref() {
            Expression::Ident(name, _) => assert_eq!(name, "x"),
            _ => panic!("expected ident operand"),
        },
        _ => panic!("expected unary neg expression"),
    }
}

#[test]
fn unary_double_not() {
    let items = common::do_lower("fn f(): bool { !!true }");
    let func = match &items[0] {
        Item::Function(f) => f,
        _ => panic!("expected function"),
    };
    let last = func.body.last().unwrap();
    match last {
        Statement::Value(Expression::Bool(v, _), _) => assert!(*v),
        _ => panic!("expected folded bool literal (double not = identity), got {last:?}"),
    }
}

#[test]
fn unary_precedence() {
    let items = common::do_lower("fn f(): int64 { -2 * 3 }");
    let func = match &items[0] {
        Item::Function(f) => f,
        _ => panic!("expected function"),
    };
    let last = func.body.last().unwrap();
    match last {
        Statement::Value(
            Expression::Binary {
                left,
                op: BinaryOp::Mul,
                right,
                ..
            },
            _,
        ) => {
            match left.as_ref() {
                Expression::Int(v, _) => assert_eq!(*v, -2),
                _ => panic!("expected folded int literal -2 as left operand"),
            }
            match right.as_ref() {
                Expression::Int(v, _) => assert_eq!(*v, 3),
                _ => panic!("expected int literal 3 as right operand"),
            }
        }
        _ => {
            panic!("expected binary expression (-2 unfolded to Int(-2) by const folding, then * 3)")
        }
    }
}

#[test]
fn struct_definition_lower() {
    let items = common::do_lower("struct Point {\n    x: int32,\n    y: float64,\n}");
    assert_eq!(items.len(), 1);
    let s = match &items[0] {
        Item::Struct(s) => s,
        other => panic!("expected struct, got {other:?}"),
    };
    assert_eq!(s.name, "Point");
    assert_eq!(s.fields.len(), 2);
    assert_eq!(s.fields[0].name, "x");
    assert_eq!(s.fields[0].type_, Type::Primitive(Primitive::Int32));
    assert_eq!(s.fields[1].name, "y");
    assert_eq!(s.fields[1].type_, Type::Primitive(Primitive::Float64));
}

#[test]
fn struct_empty_lower() {
    let items = common::do_lower("struct Empty {}");
    let s = match &items[0] {
        Item::Struct(s) => s,
        other => panic!("expected struct, got {other:?}"),
    };
    assert_eq!(s.name, "Empty");
    assert!(s.fields.is_empty());
}

#[test]
fn tuple_type_in_param() {
    let items = common::do_lower("fn f(x: (int32, float64)) {}");
    let func = match &items[0] {
        Item::Function(f) => f,
        _ => panic!("expected function"),
    };
    assert_eq!(func.params.len(), 1);
    match &func.params[0].type_ {
        Type::Tuple(elements) => {
            assert_eq!(elements.len(), 2);
            assert_eq!(elements[0], Type::Primitive(Primitive::Int32));
            assert_eq!(elements[1], Type::Primitive(Primitive::Float64));
        }
        _ => panic!("expected tuple type"),
    }
}

#[test]
fn tuple_definition_lower() {
    let items = common::do_lower("tuple Pair(int32, float64)");
    assert_eq!(items.len(), 1);
    let t = match &items[0] {
        Item::TupleStruct(t) => t,
        other => panic!("expected tuple struct, got {other:?}"),
    };
    assert_eq!(t.name, "Pair");
    assert_eq!(t.types.len(), 2);
    assert_eq!(t.types[0], Type::Primitive(Primitive::Int32));
    assert_eq!(t.types[1], Type::Primitive(Primitive::Float64));
}

#[test]
fn tuple_empty_lower() {
    let items = common::do_lower("tuple Unit()");
    let t = match &items[0] {
        Item::TupleStruct(t) => t,
        other => panic!("expected tuple struct, got {other:?}"),
    };
    assert_eq!(t.name, "Unit");
    assert!(t.types.is_empty());
}

#[test]
fn enum_definition_lower() {
    let items = common::do_lower(
        "enum Option {\n    None,\n    Some(int32),\n    Error { code: int32, message: string },\n}",
    );
    assert_eq!(items.len(), 1);
    let e = match &items[0] {
        Item::Enum(e) => e,
        other => panic!("expected enum, got {other:?}"),
    };
    assert_eq!(e.variants.len(), 3);

    assert_eq!(e.variants[0].name, "None");
    assert!(e.variants[0].data.is_none());

    assert_eq!(e.variants[1].name, "Some");
    match &e.variants[1].data {
        Some(EnumVariantData::Tuple(types)) => {
            assert_eq!(types.len(), 1);
            assert_eq!(types[0], Type::Primitive(Primitive::Int32));
        }
        other => panic!("expected tuple variant, got {other:?}"),
    }

    assert_eq!(e.variants[2].name, "Error");
    match &e.variants[2].data {
        Some(EnumVariantData::Struct(fields)) => {
            assert_eq!(fields.len(), 2);
            assert_eq!(fields[0].name, "code");
            assert_eq!(fields[1].name, "message");
        }
        other => panic!("expected struct variant, got {other:?}"),
    }
}

#[test]
fn tuple_expression_lower() {
    let items = common::do_lower("fn f() { let a = (1, 2); let b = (1,); }");
    let func = match &items[0] {
        Item::Function(f) => f,
        other => panic!("expected function, got {other:?}"),
    };
    match &func.body[0] {
        Statement::Let {
            value: Expression::Tuple(elements, _),
            ..
        } => {
            assert_eq!(elements.len(), 2);
            assert!(matches!(elements[0], Expression::Int(1, _)));
            assert!(matches!(elements[1], Expression::Int(2, _)));
        }
        other => panic!("expected tuple expression, got {other:?}"),
    }
    match &func.body[1] {
        Statement::Let {
            value: Expression::Tuple(elements, _),
            ..
        } => {
            assert_eq!(elements.len(), 1);
            assert!(matches!(elements[0], Expression::Int(1, _)));
        }
        other => panic!("expected tuple expression, got {other:?}"),
    }
}

#[test]
fn field_access_lower() {
    let items = common::do_lower("fn f(p: Point): int32 { p.x }");
    let func = match &items[0] {
        Item::Function(f) => f,
        other => panic!("expected function, got {other:?}"),
    };
    let last = func.body.last().unwrap();
    match last {
        Statement::Value(Expression::Field { object, name, .. }, _) => {
            assert_eq!(name, "x");
            match object.as_ref() {
                Expression::Ident(n, _) => assert_eq!(n, "p"),
                other => panic!("expected ident, got {other:?}"),
            }
        }
        other => panic!("expected field access, got {other:?}"),
    }
}

#[test]
fn match_expression_lower() {
    let items = common::do_lower("fn f(x: int32): int32 { match x { 1 => 10, _ => 0 } }");
    let func = match &items[0] {
        Item::Function(f) => f,
        other => panic!("expected function, got {other:?}"),
    };
    let last = func.body.last().unwrap();
    match last {
        Statement::Value(Expression::Match { value, arms, .. }, _) => {
            match value.as_ref() {
                Expression::Ident(n, _) => assert_eq!(n, "x"),
                other => panic!("expected ident, got {other:?}"),
            }
            assert_eq!(arms.len(), 2);

            match &arms[0].pattern {
                Pattern::Literal(LiteralPattern::Int(1), _) => {}
                other => panic!("expected int literal pattern, got {other:?}"),
            }
            match &arms[0].body.as_ref() {
                Expression::Int(10, _) => {}
                other => panic!("expected int body, got {other:?}"),
            }

            match &arms[1].pattern {
                Pattern::Wildcard(_) => {}
                other => panic!("expected wildcard pattern, got {other:?}"),
            }
        }
        other => panic!("expected match expression, got {other:?}"),
    }
}
