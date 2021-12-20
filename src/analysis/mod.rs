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

        for hi in &self.vars {
            println!("{:?}", hi);
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

    fn type_checker(&mut self) -> Vec<Error> {
        let mut errors: Vec<Error> = Vec::new();

        for var in &self.ast.namespaces.first().unwrap().statements {
            if let StatementKind::Variable(variable) = &var.kind {
                let mut a = 0;
                let mut b = 0;
                let mut found_literal = Literal {
                    kind: LiteralKind::Int,
                    value: Default::default(),
                    span: Span {
                        range: (0, 0),
                        file_id: ""
                    },
                    id: 0
                };
                if let Type::Primitive(var_type) = &variable.var_type {
                    use PrimitiveType::*;

                    a = match var_type {
                        I8 | I16 | I32 | I64 | I128 | U8 | U16 | U32 | U64 | U128 => self.insert_to_vars(TypeInfo::Integer),
                        Float32 | Float64 => self.insert_to_vars(TypeInfo::FloatingPointLiteral),
                        Char => self.insert_to_vars(TypeInfo::Char),
                        Bool => self.insert_to_vars(TypeInfo::Bool),
                        Var => self.insert_to_vars(TypeInfo::Unknown),
                        _ => panic!("what the fuck u are using a fucking VOID ON A FUCKING VAR???")
                    };
                }
                if let ExpressionKind::Literal(literal) = &*variable.expression.as_ref().unwrap().kind {
                    use LiteralKind::*;
                    found_literal = literal.clone();
                    b = match literal.kind {
                        Char => self.insert_to_vars(TypeInfo::Char),
                        Bool => self.insert_to_vars(TypeInfo::Bool),
                        Int => self.insert_to_vars(TypeInfo::Integer),
                        Float => self.insert_to_vars(TypeInfo::FloatingPointLiteral),
                        String => self.insert_to_vars(TypeInfo::String)
                    };
                }

                match self.unify(a, b) {
                    Ok(_) => (),
                    Err(_) => errors.push(Error::TypeMismatchError(TypeMismatchError {
                        span: variable.span.clone(),
                        expected_type: variable.var_type.clone(),
                        found_type: found_literal
                    }))
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

    fn unify(&mut self, a: TypeId, b: TypeId) -> Result<(), String> {
        use TypeInfo::*;

        match(self.vars[a].clone(), self.vars[b].clone()) {

            (Ref(a), _) => self.unify(a, b),
            (_, Ref(b)) => self.unify(a, b),

            (Unknown, _) => { self.vars[a] = Ref(b); Ok(()) },
            (_, Unknown) => { self.vars[b] = Ref(a); Ok(()) },


            (Integer, Integer) => Ok(()),
            (FloatingPointLiteral, FloatingPointLiteral) => Ok(()),

            (a, b) => {Err(format!("Type mismatch between {:?} and {:?}", a, b))}
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
                    Report::build(ReportKind::Error, error.span.file_id, error.span.range.0).with_code(420)
                        .with_message(format!("Mismatched types")).with_label(Label::new(error.found_type.span).with_message(format!("Expected {:?}, found {:?}", error.expected_type, error.found_type.kind))
                        .with_color(red)).finish().print(sources(vec![("test.vnl", &self.source)])).unwrap();
                }
            }
        }
    }
}