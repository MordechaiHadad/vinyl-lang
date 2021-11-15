use crate::ast::ast::*;
use tree_sitter::{Parser, Language, Node};

pub fn parse_into_ast(node: &Node, source: &str) -> Option<Vec<AST>> {
    let mut cursor = node.walk();
    let mut ast = Vec::new();

    for child in node.children(&mut cursor.clone()) {
        const VariableDecleration: u16 = TreeSitter::VariableDeclaration as u16;
        const FunctionDeclaration: u16 = TreeSitter::FunctionDeclaration as u16;

        match child.kind_id() {
            VariableDecleration => { // If node is a variable declaration do:

                let mut children = child.children(&mut cursor);


                let mut subchild = children.next().unwrap();
                let var_type = parse_type(Span(subchild.start_byte(), subchild.end_byte()), &source).unwrap();


                subchild = children.next().unwrap();
                let name = Span(subchild.start_byte(), subchild.end_byte());

                subchild = children.next().unwrap();

                if subchild.kind_id() != 1 {
                    ast.push(AST::Variable(Variable{
                    var_type, name, span: Span(child.start_byte(), child.end_byte()),
                    id: child.id(), expression: None,
                }));
                    continue;
                }

                let node = children.next().unwrap();
                let expression = parse_expression(&node);

                ast.push(AST::Variable(Variable{
                    var_type, name, span: Span(child.start_byte(), child.end_byte()),
                    id: child.id(), expression: Some(expression),
                }));
            },
            _ => continue,
        }

    }

    Some(ast)
}

fn parse_type(span: Span, source: &str) -> Option<Type> {
    use crate::ast::ast::PrimitiveType::*;
    use crate::ast::ast::Type::*;
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
