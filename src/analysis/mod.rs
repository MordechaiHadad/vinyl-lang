pub mod errors;

use ariadne::{Color, ColorGenerator, Label, Report, ReportKind, Source, sources};
use lasso::{Rodeo, Spur};
use crate::analysis::errors::{Error, NullReferenceError};
use crate::parser::ast::{AST, Expression, ExpressionKind, Type, Variable, PrimitiveType, LiteralKind, StatementKind, Identifier};

type TypeId = usize;

pub enum TypeInfo {
    Unknown,
    Ref(TypeId),
    Integer,
    FloatingPointLiteral,
    Bool,
    Char,
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

        let red = Color::Red;

        for error in &null_references_errors {
            match error {
                Error::NullReferenceError(error) => {
                    Report::build(ReportKind::Error, error.span.file_id, error.span.range.0)
                        .with_message(format!("Cannot find value {} in scope", self.rodeo.resolve(&error.value)))
                        .with_label(Label::new(error.span).with_message("Not found in this scope").with_color(red)).finish().print(sources(vec![("test.vnl", &self.source)])).unwrap();
                }
            }
        }
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

    fn type_checker(&mut self) -> Vec<String> {
        let mut errors: Vec<String> = Vec::new();

        for var in &self.ast.namespaces.first().unwrap().statements {
            if let StatementKind::Variable(variable) = &var.kind {
                let mut variable_type: TypeId;
                let mut literal_type: TypeId;
                if let Type::Primitive(var_type) = &variable.var_type {
                    use PrimitiveType::*;

                    variable_type = match var_type {
                        I8 | I16 | I32 | I64 | I128 | U8 | U16 | U32 | U64 | U128 => self.insert_to_vars(TypeInfo::Integer),
                        Float32 | Float64 => self.insert_to_vars(TypeInfo::FloatingPointLiteral),
                        Char => self.insert_to_vars(TypeInfo::Char),
                        Bool => self.insert_to_vars(TypeInfo::Bool),
                        Var => self.insert_to_vars(TypeInfo::Unknown),
                        _ => panic!("what the fuck u are using a fucking VOID ON A FUCKING VAR???")
                    };
                }
                   if let ExpressionKind::Literal(literal) =&*variable.expression.as_ref().unwrap().kind {
                       use LiteralKind::*;
                       literal_type = match literal.kind {
                           Char => self.insert_to_vars(TypeInfo::Char),
                           Bool => self.insert_to_vars(TypeInfo::Bool),
                           Int => self.insert_to_vars(TypeInfo::Integer),
                           Float => self.insert_to_vars(TypeInfo::FloatingPointLiteral),
                           _ => panic!("helllo")
                       }
                   }
            }
        }

        errors
    }

    fn insert_to_vars(&mut self, info: TypeInfo) -> TypeId {
        let id = self.vars.len();
        self.vars.push(info);
        id
    }
}