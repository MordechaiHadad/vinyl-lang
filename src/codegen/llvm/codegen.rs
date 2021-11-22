use crate::ast::ast::{AST, Expression, ExpressionKind, PrimitiveType, Span, Type, Variable};

use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::AddressSpace;
use inkwell::values::FunctionValue;
use inkwell::types::{AnyTypeEnum, StringRadix};

pub fn codegen(ast: &Vec<AST>, source: &str) {
    let context = Context::create();
    let module = context.create_module("test");
    let builder = context.create_builder();

    for node in ast {
        match node {
            AST::Variable(variable) => {
                variable_codegen(&variable, &None, &module, &source, &context);

            },
            AST::Function(function) => {
                println!("{:?}", function);
            }
        }
    }

    module.print_to_file("test");


}

fn variable_codegen<'a,'b>(variable: &'a Variable, function: &'a Option<FunctionValue>, module: &'b Module<'a>, source: &'a str, context: &'a Context) {

    let var_type = &variable.var_type;
    let var_type = type_codegen(&var_type, &context);

    match function {
        Some(function) => {

        },
        None => {
            match var_type {
                AnyTypeEnum::IntType(value) => {
                    let new_var = module.add_global(value, None, &source[variable.name.0..variable.name.1]);                

                    if let Some(expression) = &variable.expression {
                        match &*expression.kind {
                            ExpressionKind::Binary(..) => {},
                            ExpressionKind::Literal(expression) => {

                                let int_value = value.const_int_from_string(&source[expression.value.0..expression.value.1], StringRadix::Decimal).unwrap();
                                new_var.set_initializer(&int_value);
                            }
                        }
                    }
                }
                AnyTypeEnum::FloatType(value) => {

                    let new_var = module.add_global(value, None, &source[variable.name.0..variable.name.1]);                
                    if let Some(expression) = &variable.expression {
                        match &*expression.kind {
                            ExpressionKind::Binary(..) => {},
                            ExpressionKind::Literal(expression) => {

                                let float_value = value.const_float_from_string(&source[expression.value.0..expression.value.1]);
                                new_var.set_initializer(&float_value);
                            }
                        }
                    }
                }
                _ => println!("nothing")
            }
        }
    }
}

fn parse_expression(expression: &Option<Expression>) -> Option<Span> {
    match expression {
        Some(expression) => {
            match &*expression.kind {
                ExpressionKind::Literal(value) => Some(value.value),
                _ => None,
            }
        },
        None => None
    }
}

fn type_codegen<'a>(sent_type: &'a Type, context: &'a Context) -> AnyTypeEnum<'a> {

    let sent_type = match sent_type {
        Type::Primitive(the_type) => {
            match the_type {
                PrimitiveType::I8 => AnyTypeEnum::IntType(context.i8_type()),
                PrimitiveType::I16 => AnyTypeEnum::IntType(context.i16_type()),
                PrimitiveType::I32 => AnyTypeEnum::IntType(context.i32_type()),
                PrimitiveType::I64 => AnyTypeEnum::IntType(context.i64_type()),
                PrimitiveType::I128 => AnyTypeEnum::IntType(context.i128_type()),
                PrimitiveType::Float32 => AnyTypeEnum::FloatType(context.f32_type()),
                PrimitiveType::Float64 => AnyTypeEnum::FloatType(context.f64_type()),
                PrimitiveType::Char => AnyTypeEnum::IntType(context.i32_type()),
                _ => AnyTypeEnum::VoidType(context.void_type()),
            }
        },
        _ => AnyTypeEnum::VoidType(context.void_type())
    };

    sent_type
}