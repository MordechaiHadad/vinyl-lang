use crate::ast::ast::{AST, PrimitiveType, Type, Variable};

use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::AddressSpace;
use inkwell::values::FunctionValue;

pub fn codegen(ast: &Vec<AST>, source: &str) {
    let context = Context::create();
    let module = context.create_module("test");
    let builder = context.create_builder();

    for node in ast {
        match node {
            AST::Variable(variable) => {
                // let var_type = &variable.var_type;
                // let var_type = match var_type {
                //     Type::Primitive(hii) => {
                //         match hii {
                //             PrimitiveType::I32 => context.i32_type(),
                //             _ => continue,
                //         }
                //     },
                //     _ => continue,
                // };
                // let variable = module.add_global(var_type, Some(AddressSpace::Const), &source[variable.name.0..variable.name.1]);
                // let i32_value = var_type.const_int(10,  false);
                // variable.set_initializer(&i32_value);
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
    use inkwell::types::AnyTypeEnum;

    let var_type = &variable.var_type;
    let var_type = type_codegen(&var_type, &context);

    match function {
        Some(function) => {

        },
        None => {
            match var_type {
                AnyTypeEnum::IntType(value) => {
                    let var_value = match &*variable.expression.as_ref().unwrap().kind {
                        crate::ast::ast::ExpressionKind::Binary(..) => &crate::ast::ast::Span(10, 10),
                        crate::ast::ast::ExpressionKind::Literal(expression) => &expression.value,
                    };
                    let new_var = module.add_global(value, None, &source[variable.name.0..variable.name.1]);                
                    let int_value = value.const_int_from_string(&source[var_value.0..var_value.1], inkwell::types::StringRadix::Decimal).unwrap();
                    new_var.set_initializer(&int_value);
                }
                AnyTypeEnum::FloatType(value) => {
                    // let variable = module.add_global(value, Some(AddressSpace::Const), &source[variable.name.0..variable.name.1]);                
                }
                _ => println!("nothing")
            }
        }
    }
}

fn type_codegen<'a>(sent_type: &'a Type, context: &'a Context) -> inkwell::types::AnyTypeEnum<'a> {
    use inkwell::types::AnyTypeEnum;


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
                _ => AnyTypeEnum::VoidType(context.void_type()),
            }
        },
        _ => AnyTypeEnum::VoidType(context.void_type())
    };

    sent_type
}