mod enums;
pub mod export;

use std::any::Any;
use std::ptr::NonNull;

use crate::parser::ast::{
    Expression, ExpressionKind, LiteralKind, PrimitiveType, Span, StatementKind, Type, Variable,
    AST,
};
use enums::AnyVariableEnum;

use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::types::{AnyTypeEnum, IntType, StringRadix};
use inkwell::values::{AnyValueEnum, FunctionValue, GlobalValue};
use inkwell::AddressSpace;
use lasso::Rodeo;

pub struct CodegenEngine<'a> {
    pub rodeo: &'a mut Rodeo,
    pub context: &'a Context,
    pub source: &'a str,
    pub ast: &'a AST,
}

impl CodegenEngine<'_> {
    pub fn codegen(&self) -> Module {
        let module = self.context.create_module("test");
        let builder = self.context.create_builder();

        for statement in &self.ast.namespaces.first().unwrap().statements {
            match &statement.kind {
                StatementKind::Variable(variable) => {
                    self.variable_codegen(variable, &None, &module, &builder);
                }
                StatementKind::Function(function) => {
                    println!("{:?}", function);
                }
                _ => (),
            }
        }

        module.print_to_file("test");

        module
    }

    fn variable_codegen<'a, 'b>(
        &'a self,
        variable: &'a Variable,
        function: &'a Option<FunctionValue>,
        module: &'b Module<'a>,
        builder: &Builder,
    ) {
        let var_type = &variable.var_type;
        let var_type = self.type_codegen(var_type);

        match function {
            Some(function) => {}
            None => match var_type {
                AnyTypeEnum::IntType(value) => {
                    let new_var =
                        module.add_global(value, None, self.rodeo.resolve(&variable.name));
                    self.expression_codegen(
                        variable,
                        &AnyVariableEnum::GlobalValue(new_var),
                        builder,
                    );
                }
                AnyTypeEnum::FloatType(value) => {
                    let new_var =
                        module.add_global(value, None, self.rodeo.resolve(&variable.name));
                    self.expression_codegen(
                        variable,
                        &AnyVariableEnum::GlobalValue(new_var),
                        builder,
                    );
                }
                _ => (),
            },
        }
    }

    fn expression_codegen(
        &self,
        variable: &Variable,
        llvm_variable: &AnyVariableEnum,
        builder: &Builder,
    ) {
        match llvm_variable {
            AnyVariableEnum::GlobalValue(llvm_variable) => {
                let value = if let Some(value) = self.parse_expression(variable) {
                    println!("hello?");
                    match value {
                        AnyValueEnum::IntValue(value) => llvm_variable.set_initializer(&value),
                        AnyValueEnum::FloatValue(value) => llvm_variable.set_initializer(&value),
                        _ => (),
                    }
                };
            }
            AnyVariableEnum::PointerValue(llvm_variable) => {}
        }
    }

    fn parse_expression(&self, variable: &Variable) -> Option<AnyValueEnum> {
        /* match &variable.expression {
            None => {}
            Some(expression) => match *expression.kind {
                ExpressionKind::Binary(_, _, _) => todo!(),
                ExpressionKind::Literal(literal) => match literal.kind {
                    LiteralKind::String => todo!(),
                    LiteralKind::Char => todo!(),
                    LiteralKind::Int => todo!(),
                    LiteralKind::Bool => todo!(),
                    LiteralKind::Float => todo!(),
                },
                ExpressionKind::Reference(_, _) => todo!(),
            },
        } */
        None
    }

    fn type_codegen(&self, sent_type: &Type) -> AnyTypeEnum {
        match sent_type {
            Type::Primitive(the_type) => match the_type {
                PrimitiveType::I8 => AnyTypeEnum::IntType(self.context.i8_type()),
                PrimitiveType::I16 => AnyTypeEnum::IntType(self.context.i16_type()),
                PrimitiveType::I32 | PrimitiveType::Char => {
                    AnyTypeEnum::IntType(self.context.i32_type())
                }
                PrimitiveType::I64 => AnyTypeEnum::IntType(self.context.i64_type()),
                PrimitiveType::I128 => AnyTypeEnum::IntType(self.context.i128_type()),
                PrimitiveType::Float32 => AnyTypeEnum::FloatType(self.context.f32_type()),
                PrimitiveType::Float64 => AnyTypeEnum::FloatType(self.context.f64_type()),
                PrimitiveType::Bool => AnyTypeEnum::IntType(self.context.bool_type()),
                _ => AnyTypeEnum::VoidType(self.context.void_type()),
            },
            _ => AnyTypeEnum::VoidType(self.context.void_type()),
        }
    }
}
