use crate::parser::ast::{AST, Expression, ExpressionKind, Literal, LiteralKind, PrimitiveType, Type};

pub fn check_type(ast: &Vec<AST>) -> Vec<String> {
    let mut errors = Vec::new();
    for node in ast {
        match node {
            AST::Variable(variable) => {
                match &variable.expression {
                    Some(expression) => {
                        match type_check_variable(&variable.var_type, &expression.kind) {
                            Ok(()) => (),
                            Err(error) => errors.push(error)
                        }
                    },
                    None => (),
                }
            },
            _ => ()
        }
    }
    errors
}

fn type_check_variable(variable_type: &Type, expression: &ExpressionKind) -> Result<(), String> {
    match (variable_type, expression) {
        (Type::Primitive(variable_type), ExpressionKind::Literal(literal)) => {
            match (variable_type, &literal.kind) {
                (PrimitiveType::I8, LiteralKind::Int) => Ok(()),
                (PrimitiveType::I16, LiteralKind::Int) => Ok(()),
                (PrimitiveType::I32, LiteralKind::Int) => Ok(()),
                (PrimitiveType::I64, LiteralKind::Int) => Ok(()),
                (PrimitiveType::U8, LiteralKind::Int) => Ok(()),
                (PrimitiveType::U16, LiteralKind::Int) => Ok(()),
                (PrimitiveType::U32, LiteralKind::Int) => Ok(()),
                (PrimitiveType::U64, LiteralKind::Int) => Ok(()),
                (PrimitiveType::Float32, LiteralKind::Float) => Ok(()),
                (PrimitiveType::Float64, LiteralKind::Float) => Ok(()),
                (PrimitiveType::Char, LiteralKind::Char) => Ok(()),
                (PrimitiveType::Bool, LiteralKind::Bool) => Ok(()),
                (variable_type, expression_type) => Err(format!("Type mismatch between {:?} and {:?}", variable_type, expression_type))
            }
        },
        (Type::Primitive(_), ExpressionKind::Reference(..)) => Err(format!("Type checking with reference is not supported")),
        _ => Err(format!("Those arent supported yet"))
    }
}