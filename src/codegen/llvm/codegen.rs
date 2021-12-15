use std::any::Any;
use std::ptr::NonNull;

use crate::parser::ast::{AST, Expression, ExpressionKind, LiteralKind, PrimitiveType, Span, Type, Variable};
use super::enums::AnyVariableEnum;

use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::AddressSpace;
use inkwell::values::{AnyValueEnum, FunctionValue, GlobalValue};
use inkwell::types::{AnyTypeEnum, IntType, StringRadix};


pub fn codegen<'a>(ast: &'a Vec<AST>, source: &'a str, context: &'a Context) -> Module<'a> {
    let module = context.create_module("test");
    let builder = context.create_builder();

    for node in ast {
        match node {
            AST::Variable(variable) => {
                variable_codegen(&variable, &None, &module, &source, &context, &builder);

            },
            AST::Function(function) => {
                println!("{:?}", function);
            }
        }
    }

    module.print_to_file("test");

    module
}

fn variable_codegen<'a,'b>(variable: &'a Variable, function: &'a Option<FunctionValue>, module: &'b Module<'a>, source: &'a str, context: &'a Context, builder: &Builder) {

    let var_type = &variable.var_type;
    let var_type = type_codegen(&var_type, &context);

    match function {
        Some(function) => {

        },
        None => {
            match var_type {
                AnyTypeEnum::IntType(value) => {
                    let new_var = module.add_global(value, None, &source[variable.name.0..variable.name.1]);                
                    expression_codegen(&variable, &AnyVariableEnum::GlobalValue(new_var), &builder, &source, &context);

                }
                AnyTypeEnum::FloatType(value) => {

                    let new_var = module.add_global(value, None, &source[variable.name.0..variable.name.1]);                
                    expression_codegen(&variable, &AnyVariableEnum::GlobalValue(new_var), &builder, &source, &context);

                },
                _ => println!("nothing")
            }
        }
    }
}

fn expression_codegen(variable: &Variable, llvm_variable: &AnyVariableEnum, builder: &Builder, source: &str, context: &Context) {

    match llvm_variable {
        AnyVariableEnum::GlobalValue(llvm_variable) => {

            let value = match parse_expression(&variable, &source, &context) {
                Some(value) => {
                    match value {
                        AnyValueEnum::IntValue(value) => llvm_variable.set_initializer(&value),
                        AnyValueEnum::FloatValue(value) => llvm_variable.set_initializer(&value),
                        _ => ()
                    }
                },
                None => ()
            };
        },
        AnyVariableEnum::PointerValue(llvm_variable) => {

        }
    }

}

fn parse_expression<'a>(variable: &'a Variable, source: &'a str, context: &'a Context) -> Option<AnyValueEnum<'a>> {
    None
}

fn type_codegen<'a>(sent_type: &'a Type, context: &'a Context) -> AnyTypeEnum<'a> {

    let sent_type = match sent_type {
        Type::Primitive(the_type) => {
            match the_type {
                PrimitiveType::I8 => AnyTypeEnum::IntType(context.i8_type()),
                PrimitiveType::I16 => AnyTypeEnum::IntType(context.i16_type()),
                PrimitiveType::I32 | PrimitiveType::Char => AnyTypeEnum::IntType(context.i32_type()),
                PrimitiveType::I64 => AnyTypeEnum::IntType(context.i64_type()),
                PrimitiveType::I128 => AnyTypeEnum::IntType(context.i128_type()),
                PrimitiveType::Float32 => AnyTypeEnum::FloatType(context.f32_type()),
                PrimitiveType::Float64 => AnyTypeEnum::FloatType(context.f64_type()),
                PrimitiveType::Bool => AnyTypeEnum::IntType(context.bool_type()),
                _ => AnyTypeEnum::VoidType(context.void_type()),
            }
        },
        _ => AnyTypeEnum::VoidType(context.void_type())
    };

    sent_type
}