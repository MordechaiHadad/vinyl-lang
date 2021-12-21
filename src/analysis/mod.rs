pub mod errors;

use ariadne::{Color, ColorGenerator, Label, Report, ReportKind, Source, sources};
use lasso::{Rodeo, Spur};
use crate::analysis::errors::{Error, NullReferenceError, TypeMismatchError};
use crate::analysis::TypeInfo::Unknown;
use crate::parser::ast::{AST, Expression, ExpressionKind, Type, Variable, PrimitiveType, LiteralKind, StatementKind, Identifier, Literal, Span};

type TypeId = usize;

#[derive(Debug, Clone, PartialEq)]
pub enum TypeInfo {
    Unknown,
    Ref(TypeId),
    Integer,
    FloatingPointLiteral,
    Bool,
    Char,
    String
}

pub struct AnalysisEngine<'a, 'b: 'a> {
    pub stack: Vec<(Spur, &'a Type)>,
    pub ast: &'b AST,
    pub rodeo: &'b mut Rodeo,
    pub vars: Vec<TypeInfo>,
    pub source: &'b str,
}

impl<'a> AnalysisEngine<'a, 'a> {
    pub fn new(ast: &'a AST, rodeo: &'a mut Rodeo, source: &'a str) -> Self {
        Self {stack: Vec::new(), ast, rodeo, vars: Vec::new(), source}
    }

    pub fn start(&mut self) {
        self.insert_to_stack();
        let null_references_errors = self.check_for_null_reference();

        self.print_errors(&null_references_errors);

        let type_checker_errors = self.type_checker();

        self.print_errors(&type_checker_errors);

        let not_infered: Vec<TypeInfo> = self.vars.iter().cloned().filter(|x| x == &TypeInfo::Unknown).collect();
    }

    fn insert_to_stack(&mut self) {
        for var in &self.ast.namespaces.first().unwrap().statements {
            match &var.kind {
                 StatementKind::Variable(variable) => {
                    self.stack.push((variable.name, &variable.var_type));
                 },
                 StatementKind::Function(function)=> {
                     self.stack.push((function.name, &function.return_type))
                 },
                _ => ()
            }
        }
    }

    fn check_for_null_reference(&mut self) -> Vec<Error> {
        let mut errors: Vec<Error> = Vec::new();
        for var in &self.ast.namespaces.first().unwrap().statements {
            if let StatementKind::Variable(variable) = &var.kind {
                if let Variable {expression, ..} = variable {
                    if let ExpressionKind::Reference(mutability, identifier) = &*expression.as_ref().unwrap().kind {
                        if self.stack.iter().all(|(x, _)| x != &identifier.symbol) {
                            errors.push( Error::NullReferenceError(
                                NullReferenceError {
                                    span: identifier.span.clone(),
                                    value: identifier.symbol.clone()
                                }
                            ));
                        }
                    }
                }

            }
        }
        errors
    }

    fn type_checker(&mut self) -> Vec<Error> {
        let mut errors: Vec<Error> = Vec::new();

        for var in &self.ast.namespaces.first().unwrap().statements {
            if let StatementKind::Variable(variable) = &var.kind {
                let mut a = 0;
                let mut b = 0;
                if let Type::Primitive(primitive) = &variable.var_type {
                    use PrimitiveType::*;

                    a = self.insert_to_vars(self.primitive_to_typeinfo(primitive));
                }
                match &*variable.expression.as_ref().unwrap().kind {
                    ExpressionKind::Literal(literal) => {
                        use LiteralKind::*;
                        b = match literal.kind {
                            Char => self.insert_to_vars(TypeInfo::Char),
                            Bool => self.insert_to_vars(TypeInfo::Bool),
                            Int => self.insert_to_vars(TypeInfo::Integer),
                            Float => self.insert_to_vars(TypeInfo::FloatingPointLiteral),
                            String => self.insert_to_vars(TypeInfo::String)
                        };
                    },
                    ExpressionKind::Reference(_, identifier) => {
                        if self.stack.iter().all(|(x, _)| x != &identifier.symbol) {
                            b = self.insert_to_vars(TypeInfo::Unknown);
                        } else {
                            b = match self.stack.iter().cloned().find(|(x, _)| x == &identifier.symbol).unwrap().1 {
                                Type::Primitive(primitive) => {
                                    self.insert_to_vars(self.primitive_to_typeinfo(primitive))
                                }
                            };
                        }
                    },
                    _ => todo!()
                }

                match self.unify(a, b) {
                    Ok(_) => (),
                    Err(_) => errors.push(Error::TypeMismatchError(TypeMismatchError {
                        span: variable.span.clone(),
                        expected_type: variable.var_type.clone(),
                        found_expression: variable.expression.clone().unwrap(),
                    }))
                }
            }
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
            _ => todo!()
        }
    }

    fn insert_to_vars(&mut self, info: TypeInfo) -> TypeId {
        let id = self.vars.len();
        self.vars.push(info);
        id
    }

    fn unify(&mut self, a: TypeId, b: TypeId) -> Result<(), (TypeInfo, TypeInfo)> {
        use TypeInfo::*;

        match(self.vars[a].clone(), self.vars[b].clone()) {

            (Ref(a), _) => self.unify(a, b),
            (_, Ref(b)) => self.unify(a, b),

            (Unknown, _) => { self.vars[a] = Ref(b); Ok(()) },
            (_, Unknown) => { self.vars[b] = Ref(a); Ok(()) },


            (Integer, Integer) => Ok(()),
            (FloatingPointLiteral, FloatingPointLiteral) => Ok(()),
            (FloatingPointLiteral, Integer) => Ok(()),
            (Bool, Bool) => Ok(()),
            (Char, Char) => Ok(()),
            (String, String) => Ok(()),

            (a, b) => {Err((a, b))}
        }
    }

    fn print_errors(&self, errors: &Vec<Error>) {
        let red = Color::Red;

        for error in errors {
            match error {
                Error::NullReferenceError(error) => {
                    Report::build(ReportKind::Error, error.span.file_id, error.span.range.0).with_code(69)
                        .with_message(format!("Cannot find value {} in scope", self.rodeo.resolve(&error.value)))
                        .with_label(Label::new(error.span).with_message("Not found in this scope").with_color(red)).finish().print(sources(vec![("test.vnl", &self.source)])).unwrap();
                }
                Error::TypeMismatchError(error) => {

                    let expected_type = self.pretty_print_type(&error.expected_type).unwrap();

                    let found_type = self.pretty_print_expression(&error.found_expression).unwrap();

                    Report::build(ReportKind::Error, error.span.file_id, error.span.range.0).with_code(420)
                        .with_message(format!("Mismatched types")).with_label(Label::new(error.found_expression.span).with_message(format!("Expected {}, found {}", expected_type, found_type))
                        .with_color(red)).finish().print(sources(vec![("test.vnl", &self.source)])).unwrap();
                }
            }
        }
    }

    fn pretty_print_type(&self, var_type: &Type) -> Result<&str, &str> {
        match var_type {
            Type::Primitive(primitive) => {
                match primitive {
                    PrimitiveType::I8 => Ok("int8"),
                    PrimitiveType::I16 => Ok("int16"),
                    PrimitiveType::I32 => Ok("int32"),
                    PrimitiveType::I64 => Ok("int64"),
                    PrimitiveType::I128 => Ok("int128"),
                    PrimitiveType::U8 => Ok("uint8"),
                    PrimitiveType::U16 => Ok("uint16"),
                    PrimitiveType::U32 => Ok("uint32"),
                    PrimitiveType::U64 => Ok("uint64"),
                    PrimitiveType::U128 => Ok("uint128"),
                    PrimitiveType::Char => Ok("char"),
                    PrimitiveType::Bool => Ok("bool"),
                    PrimitiveType::Float32 => Ok("float32"),
                    PrimitiveType::Float64 => Ok("float64"),
                    PrimitiveType::String => Ok("string"),
                    PrimitiveType::Void => Ok("void"),
                    PrimitiveType::Var => Ok("var")
                }
            }
        }
    }

    fn pretty_print_expression(&self, expression: &Expression) -> Result<&str, &str> {
        match &*expression.kind {
            ExpressionKind::Literal(literal) => {
                match literal.kind {
                    LiteralKind::Int => Ok("integer"),
                    LiteralKind::String => Ok("string"),
                    LiteralKind::Bool => Ok("boolean"),
                    LiteralKind::Char => Ok("char"),
                    LiteralKind::Float => Ok("floating point number")
                }
            },
            ExpressionKind::Reference(_, identifier) => {
                let reference = self.stack.iter().cloned().find(|(x, _)| x == &identifier.symbol).unwrap();

                Ok(self.pretty_print_type(reference.1).unwrap())
            },
            _ => todo!()
        }
    }
}