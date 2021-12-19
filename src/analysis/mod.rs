use lasso::{Rodeo, Spur};
use crate::parser::ast::{AST, Expression, ExpressionKind, Type, Variable, PrimitiveType, LiteralKind, StatementKind};

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
    pub vars: Vec<TypeInfo>
}

impl<'a> AnalysisEngine<'a, 'a> {
    pub fn new(ast: &'a AST, rodeo: &'a mut Rodeo) -> Self {
        Self {stack: Vec::new(), ast, rodeo, vars: Vec::new()}
    }

    pub fn start(&mut self) {
        self.insert_to_stack();
        let null_errors = self.check_for_null_reference();
        if null_errors.len() > 0 {
            for error in null_errors {
                println!("{}", error);
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

    fn check_for_null_reference(&mut self) -> Vec<String> {
        let mut errors: Vec<String> = Vec::new();
        for var in &self.ast.namespaces.first().unwrap().statements {
            if let StatementKind::Variable(variable) = &var.kind {
                if let Variable {expression, ..} = variable {
                    if let ExpressionKind::Reference(mutability, identifier) = &*expression.as_ref().unwrap().kind {
                        if self.stack.iter().all(|(x, _)| x != &identifier.symbol) {
                            errors.push(format!("{} is null reference", self.rodeo.resolve(&identifier.symbol)));
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