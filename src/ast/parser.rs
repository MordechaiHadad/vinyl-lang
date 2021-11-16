use crate::ast::ast::*;
use tree_sitter::{Parser, Language, Node};

pub fn parse_into_ast(node: &Node, source: &str) -> Option<Vec<AST>> {
    let mut cursor = node.walk();
    let mut ast = Vec::new();

    for child in node.children(&mut cursor.clone()) {
        const VariableDecleration: u16 = TreeSitter::VariableDeclaration as u16;
        const FunctionDeclaration: u16 = TreeSitter::FunctionDeclaration as u16;

        match child.kind_id() {
            VariableDecleration => { 
                let variable = parse_variable(&child, &source);
                
                ast.push(AST::Variable(variable));
            },
            FunctionDeclaration => {
                let function = parse_function(&child, &source);

                ast.push(AST::Function(function));
            }
            _ => continue,
        }

    }


    Some(ast)
}

fn parse_function(root: &Node, source: &str) -> Function {
    const LeftParen: u16 = TreeSitter::LeftParen as u16;
    const RightParen: u16 = TreeSitter::RightParen as u16;
    const LeftCurly: u16 = TreeSitter::LeftCurly as u16;
    const RightCurly: u16 = TreeSitter::RightCurly as u16;

    let mut cursor = root.walk();

    let mut children = root.children(&mut cursor);

    let mut subchild = children.next().unwrap();

    let return_type = parse_type(Span(subchild.start_byte(), subchild.end_byte()), &source).unwrap();

    subchild = children.next().unwrap();
    let name = Span(subchild.start_byte(), subchild.end_byte());

    
    subchild = children.next().unwrap();
    let parameters = parse_parameters(&subchild, &source);


    subchild = children.next().unwrap();
    let body = parse_function_body(&subchild, source);

    Function {
        return_type, name, parameters, body, span: Span(root.start_byte(), root.end_byte()),
        id: root.id()
    }

}

fn parse_variable(root: &Node, source: &str) -> Variable {
    const EqualSign: u16 = TreeSitter::EqualSign as u16;
    
    let mut cursor = root.walk();

    let mut children = root.children(&mut cursor);

    let mut subchild = children.next().unwrap();

    let var_type = parse_type(Span(subchild.start_byte(), subchild.end_byte()), &source).unwrap();

    subchild = children.next().unwrap();
    let name = Span(subchild.start_byte(), subchild.end_byte());

    subchild = children.next().unwrap();
    if subchild.kind_id() != EqualSign {
        return Variable {
        var_type, name, span: Span(root.start_byte(), root.end_byte()),
        id: root.id(), expression: None};
    }
    
    let node = children.next().unwrap();
    let expression = parse_expression(&node);   

    Variable {
    var_type, name, span: Span(root.start_byte(), root.end_byte()),
    id: root.id(), expression: Some(expression)}
}

fn parse_function_body(root: &Node, source: &str) -> Option<Block> {
    const VariableDecleration: u16 = TreeSitter::VariableDeclaration as u16;
    const Literal: u16 = TreeSitter::Literal as u16;
    const BinaryExpression: u16 = TreeSitter::BinaryExpression as u16;
    if root.child_count() <= 2 {
        return None;
    }

    let mut statements = Vec::new();
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        match child.kind_id() {
            VariableDecleration => {
                let var = parse_variable(&child, &source);
                statements.push(Statement {
                    kind: StatementKind::Variable(var),
                    span: Span(child.start_byte(), child.end_byte()),
                    id: child.id()
                });
            },
            Literal | BinaryExpression => {
                let expression = parse_expression(&child);
                statements.push(Statement {
                    kind: StatementKind::Expression(expression),
                    span: Span(child.start_byte(), child.end_byte()),
                    id: child.id()
                })
            }
            _ => continue,
        }
    }

    Some(Block {
        statements,
        span: Span(root.start_byte(), root.end_byte()),
        id: root.id(),
    })
}

fn parse_parameters(root: &Node, source: &str) -> Option<Parameters> {
    const Parameter: u16 = TreeSitter::Parameter as u16;
    if root.child_count() <= 2 {
        return None;
    }

    let mut parameters = Vec::new();
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        match child.kind_id() {
            Parameter => {
                let parameter = parse_parameter(&child, &source);
                parameters.push(parameter);
            }
            _ => continue,
        }
    }

    Some(Parameters {
        parameters,
        span: Span(root.start_byte(), root.end_byte()),
        id: root.id()
    })

    
}

fn parse_parameter(root: &Node, source: &str) -> Parameter {

    let mut cursor = root.walk();

    let mut children = root.children(&mut cursor);

    let mut subchild = children.next().unwrap();
    let param_type = parse_type(Span(subchild.start_byte(), subchild.end_byte()), &source).unwrap();


    subchild = children.next().unwrap();
    let name = Span(subchild.start_byte(), subchild.end_byte());

    Parameter {
        param_type,
        name,
        span: Span(root.start_byte(), root.end_byte()),
        id: root.id(),
    }
}

fn parse_type(span: Span, source: &str) -> Option<Type> {
    use PrimitiveType::*;
    use Type::*;
    let type_text = &source[span.0..span.1];

    let new_type = match type_text {
        "bool" => Some(Primitive(Bool)),
        "char" => Some(Primitive(Char)),
        "int8" => Some(Primitive(I8)),
        "int16" => Some(Primitive(I16)),
        "int32" => Some(Primitive(I32)),
        "int64" => Some(Primitive(I64)),
        "int128" => Some(Primitive(I128)),
        "uint8" => Some(Primitive(U8)),
        "uint16" => Some(Primitive(U16)),
        "uint32" => Some(Primitive(U32)),
        "uint64" => Some(Primitive(U64)),
        "uint128" => Some(Primitive(U128)),
        "float32" => Some(Primitive(Float32)),
        "float64" => Some(Primitive(Float64)),
        _ => None,
    };

    new_type
}

fn parse_expression(root: &Node) -> Expression {
    const Literal: u16 = TreeSitter::Literal as u16;
    const BinaryExpression: u16 = TreeSitter::BinaryExpression as u16;
    
    let mut expression_kind = match root.kind_id() {
        Literal => {
            const IntegerLiteral: u16 = TreeSitter::IntegerLiteral as u16;
            const BoolLiteral: u16 = TreeSitter::BoolLiteral as u16;
            const CharLiteral: u16 = TreeSitter::CharLiteral as u16;
            const FloatingPointLiteral: u16 = TreeSitter::FloatingPointLiteral as u16;
            const StringLiteral: u16 = TreeSitter::StringLiteral as u16;

            let node = root.child(0).unwrap();
            match node.kind_id() {
                IntegerLiteral => ExpressionKind::Literal(Literal {
                        id: node.id(),
                        kind: LiteralKind::Int,
                        value: Span(node.start_byte(), node.end_byte())
                    }),
                BoolLiteral => ExpressionKind::Literal(Literal {
                    id: node.id(),
                    kind: LiteralKind::Bool,
                    value: Span(node.start_byte(), node.end_byte())
                }),
                CharLiteral => ExpressionKind::Literal(Literal {
                    id: node.id(),
                    kind: LiteralKind::Char,
                    value: Span(node.start_byte(), node.end_byte())
                }),
                FloatingPointLiteral => ExpressionKind::Literal(Literal {
                    id: node.id(),
                    kind: LiteralKind::Float,
                    value: Span(node.start_byte(), node.end_byte())
                }),
                StringLiteral => ExpressionKind::Literal(Literal { 
                    id: node.id(),
                    kind: LiteralKind::String,
                    value: Span(node.start_byte(), node.end_byte())
                }),
                _ => ExpressionKind::Literal(Literal {
                    id: node.id(),
                    kind: LiteralKind::Int,
                    value: Span(node.start_byte(), node.end_byte())
                })
            }
        },
        BinaryExpression => {
            const AddSign: u16 = TreeSitter::PlusSign as u16;
            const MinusSign: u16 = TreeSitter::MinusSign as u16;
            const Multiply: u16 = TreeSitter::Multiply as u16;
            const Divide: u16 = TreeSitter::Divide as u16;
            const ShiftLeft: u16 = TreeSitter::ShiftLeft as u16;
            const ShiftRight: u16 = TreeSitter::ShiftRight as u16;
            const And: u16 = TreeSitter::And as u16;
            const Or: u16 = TreeSitter::Or as u16;
            const BitAnd: u16 = TreeSitter::BitAnd as u16;
            const BitOr: u16 = TreeSitter::BitOr as u16;
            const BitXor: u16 = TreeSitter::BitXor as u16;
            const Equal: u16 = TreeSitter::Equal as u16;
            const NotEqual: u16 = TreeSitter::NotEqual as u16;
            const LessThan: u16 = TreeSitter::LessThan as u16;
            const LessThanOrEqual: u16 = TreeSitter::LessThanOrEqual as u16;
            const GreaterThan: u16 = TreeSitter::GreaterThan as u16;
            const GreaterThanOrEqual: u16 = TreeSitter::GreaterThanOrEqual as u16;

            let mut cursor = root.walk();
            let mut children = root.children(&mut cursor);
            let left_expression = parse_expression(&children.next().unwrap());

            let operator = match children.next().unwrap().kind_id() {
                AddSign =>  BinaryOperator {
                    kind: BinaryOperatorKind::Add,
                },
                MinusSign => BinaryOperator {
                    kind: BinaryOperatorKind::Subtract
                },
                Multiply => BinaryOperator {
                    kind: BinaryOperatorKind::Multiply
                },
                Divide => BinaryOperator {
                    kind: BinaryOperatorKind::Divide
                },
                ShiftLeft => BinaryOperator {
                    kind: BinaryOperatorKind::ShiftLeft
                },
                ShiftRight => BinaryOperator {
                    kind: BinaryOperatorKind::ShiftRight
                },
                And => BinaryOperator {
                    kind: BinaryOperatorKind::And
                },
                Or => BinaryOperator {
                    kind: BinaryOperatorKind::Or
                },
                BitAnd => BinaryOperator {
                    kind: BinaryOperatorKind::BitAnd
                },
                BitOr => BinaryOperator {
                    kind: BinaryOperatorKind::BitOr
                },
                BitXor => BinaryOperator {
                    kind: BinaryOperatorKind::BitXor
                },
                Equal => BinaryOperator {
                    kind: BinaryOperatorKind::Equal
                },
                NotEqual => BinaryOperator {
                    kind: BinaryOperatorKind::NotEqual
                },
                LessThan => BinaryOperator {
                    kind: BinaryOperatorKind::LessThan
                },
                LessThanOrEqual => BinaryOperator {
                    kind: BinaryOperatorKind::LessThanOrEqual
                },
                GreaterThan => BinaryOperator {
                    kind: BinaryOperatorKind::GreaterThan
                },
                GreaterThanOrEqual => BinaryOperator {
                    kind: BinaryOperatorKind::GreaterThanOrEqual
                },
                _ => BinaryOperator {
                    kind: BinaryOperatorKind::Add,
                }
            };


            let right_expression = parse_expression(&children.next().unwrap());


            ExpressionKind::Binary(operator, Box::new(left_expression), Box::new(right_expression))

        }
        _ => ExpressionKind::Literal(Literal {
            id: root.id(),
            kind: LiteralKind::Int,
            value: Span(root.start_byte(), root.end_byte())
        })

    };

    Expression { id: root.id(), span: Span(root.start_byte(), root.end_byte()), kind: Box::new(expression_kind)}
}