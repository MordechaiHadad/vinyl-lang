pub mod errors;

use crate::analysis::errors::{AnalyzerError, NullReferenceError, TypeMismatchError};
use crate::analysis::TypeInfo::Unknown;
use crate::parser::ast::{
    Expression, ExpressionKind, Identifier, Literal, LiteralKind, PrimitiveType, Span,
    StatementKind, Type, Variable, AST,
};
use ariadne::{sources, Color, ColorGenerator, Label, Report, ReportKind, Source};
use lasso::{Rodeo, Spur};
use std::fmt::Display;
use std::process::exit;

type TypeId = usize;

#[derive(Debug, Clone, PartialEq)]
pub enum TypeInfo {
    Unknown,
    Ref(TypeId),
    Integer,
    FloatingPointLiteral,
    Bool,
    Char,
    String,
}

pub struct AnalysisEngine<'b> {
    pub stack: Vec<(Spur, Type)>,
    pub ast: &'b AST,
    pub rodeo: &'b mut Rodeo,
    pub vars: Vec<TypeInfo>,
    pub source: &'b str,
}

impl<'a> AnalysisEngine<'a> {
    pub fn new(ast: &'a AST, rodeo: &'a mut Rodeo, source: &'a str) -> Self {
        Self {
            stack: Vec::new(),
            ast,
            rodeo,
            vars: Vec::new(),
            source,
        }
    }

    pub fn start(&mut self) {
        self.insert_to_stack();
        let null_references_errors = self.check_for_null_reference();

        let type_checker_errors = self.type_checker();

        let not_infered: Vec<TypeInfo> = self
            .vars
            .iter()
            .cloned()
            .filter(|x| x == &TypeInfo::Unknown)
            .collect();

        let number_of_errors = null_references_errors.len() + type_checker_errors.len();
        if number_of_errors > 0 {
            exit(number_of_errors as i32);
        }
    }

    fn insert_to_stack(&mut self) {
        for var in &self.ast.namespaces.first().unwrap().statements {
            match &var.kind {
                StatementKind::Variable(variable) => {
                    self.stack.push((variable.name, variable.var_type));
                }
                StatementKind::Function(function) => {
                    self.stack.push((function.name, function.return_type))
                }
                _ => (),
            }
        }
    }

    fn check_for_null_reference(&mut self) -> Vec<AnalyzerError> {
        let mut errors: Vec<AnalyzerError> = Vec::new();
        for var in &self.ast.namespaces.first().unwrap().statements {
            if let StatementKind::Variable(variable) = &var.kind {
                if let Variable { expression, .. } = variable {
                    match expression {
                        None => (),
                        Some(expression) => {
                            if let ExpressionKind::Reference(mutability, identifier) =
                                &*expression.kind
                            {
                                if self.stack.iter().all(|(x, _)| x != &identifier.symbol) {
                                    errors.push(AnalyzerError::NullReferenceError(
                                        NullReferenceError {
                                            span: identifier.span,
                                            value: identifier.symbol,
                                        },
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
        if !errors.is_empty() {
            self.print_errors(&errors);
        }
        errors
    }

    fn type_checker(&mut self) -> Vec<AnalyzerError> {
        let mut errors: Vec<AnalyzerError> = Vec::new();

        for var in &self.ast.namespaces.first().unwrap().statements {
            if let StatementKind::Variable(variable) = &var.kind {
                let mut a = 0;
                let mut b = 0;
                if let Type::Primitive(primitive) = &variable.var_type {
                    use PrimitiveType::*;

                    a = self.insert_to_vars(self.primitive_to_typeinfo(primitive));
                }
                match &variable.expression {
                    None => (),
                    Some(expression) => match &*expression.kind {
                        ExpressionKind::Literal(literal) => {
                            use LiteralKind::*;
                            b = match literal.kind {
                                Char => self.insert_to_vars(TypeInfo::Char),
                                Bool => self.insert_to_vars(TypeInfo::Bool),
                                Int => self.insert_to_vars(TypeInfo::Integer),
                                Float => self.insert_to_vars(TypeInfo::FloatingPointLiteral),
                                String => self.insert_to_vars(TypeInfo::String),
                            };
                        }
                        ExpressionKind::Reference(_, identifier) => {
                            if self.stack.iter().all(|(x, _)| x != &identifier.symbol) {
                                b = self.insert_to_vars(TypeInfo::Unknown);
                            } else {
                                let current = self
                                    .stack
                                    .iter()
                                    .position(|x| x == &(identifier.symbol, variable.var_type))
                                    .unwrap();
                                b = match self.stack.iter().nth(current - 1).unwrap().1 {
                                    Type::Primitive(primitive) => {
                                        self.insert_to_vars(self.primitive_to_typeinfo(&primitive))
                                    }
                                };
                            }
                        }
                        _ => todo!(),
                    },
                }
                match self.unify(a, b) {
                    Ok(_) => (),
                    Err(_) => errors.push(AnalyzerError::TypeMismatchError(TypeMismatchError {
                        variable: variable.clone(),
                    })),
                }
            }
        }

        if !errors.is_empty() {
            self.print_errors(&errors);
        }
        errors
    }

    fn primitive_to_typeinfo(&self, primitive: &PrimitiveType) -> TypeInfo {
        use PrimitiveType::*;
        match primitive {
            I8 | I16 | I32 | I64 | I128 | U8 | U16 | U32 | U64 | U128 => TypeInfo::Integer,
            Float32 | Float64 => TypeInfo::FloatingPointLiteral,
            Char => TypeInfo::Char,
            Bool => TypeInfo::Bool,
            String => TypeInfo::String,
            Var => TypeInfo::Unknown,
            _ => todo!(),
        }
    }

    fn insert_to_vars(&mut self, info: TypeInfo) -> TypeId {
        let id = self.vars.len();
        self.vars.push(info);
        id
    }

    fn unify(&mut self, a: TypeId, b: TypeId) -> Result<(), (TypeInfo, TypeInfo)> {
        use TypeInfo::*;

        match (self.vars[a].clone(), self.vars[b].clone()) {
            (Ref(a), _) => self.unify(a, b),
            (_, Ref(b)) => self.unify(a, b),

            (Unknown, _) => {
                self.vars[a] = Ref(b);
                Ok(())
            }
            (_, Unknown) => {
                self.vars[b] = Ref(a);
                Ok(())
            }

            (Integer, Integer) => Ok(()),
            (FloatingPointLiteral, FloatingPointLiteral) => Ok(()),
            (FloatingPointLiteral, Integer) => Ok(()),
            (Bool, Bool) => Ok(()),
            (Char, Char) => Ok(()),
            (String, String) => Ok(()),

            (a, b) => Err((a, b)),
        }
    }

    fn print_errors(&self, errors: &[AnalyzerError]) {
        let red = Color::Red;

        for error in errors {
            match error {
                AnalyzerError::NullReferenceError(error) => {
                    Report::build(ReportKind::Error, error.span.file_id, error.span.range.0)
                        .with_code(69)
                        .with_message(format!(
                            "Cannot find value {} in scope",
                            self.rodeo.resolve(&error.value)
                        ))
                        .with_label(
                            Label::new(error.span)
                                .with_message("Not found in this scope")
                                .with_color(red),
                        )
                        .finish()
                        .print(sources(vec![("test.vnl", &self.source)]))
                        .unwrap();
                }
                AnalyzerError::TypeMismatchError(error) => {
                    let found_type = self.pretty_print_expression(&error.variable).unwrap();

                    Report::build(
                        ReportKind::Error,
                        error.variable.span.file_id,
                        error.variable.span.range.0,
                    )
                    .with_code(420)
                    .with_message("Mismatched types")
                    .with_label(
                        Label::new(error.variable.expression.as_ref().unwrap().span)
                            .with_message(format!(
                                "Expected {}, found {}",
                                error.variable.var_type, found_type
                            ))
                            .with_color(red),
                    )
                    .finish()
                    .print(sources(vec![("test.vnl", &self.source)]))
                    .unwrap();
                }
            }
        }
    }

    fn pretty_print_expression(&self, variable: &Variable) -> Result<String, &str> {
        let s = String::from;
        match &*variable.expression.as_ref().unwrap().kind {
            ExpressionKind::Literal(literal) => match literal.kind {
                LiteralKind::Int => Ok(s("integer")),
                LiteralKind::String => Ok(s("string")),
                LiteralKind::Bool => Ok(s("boolean")),
                LiteralKind::Char => Ok(s("char")),
                LiteralKind::Float => Ok(s("floating point number")),
            },
            ExpressionKind::Reference(_, identifier) => {
                let current = self
                    .stack
                    .iter()
                    .position(|x| x == &(identifier.symbol, variable.var_type))
                    .unwrap();
                let reference = self.stack.iter().nth(current - 1).unwrap();

                Ok(format!("{}", reference.1))
            }
            _ => todo!(),
        }
    }
}
