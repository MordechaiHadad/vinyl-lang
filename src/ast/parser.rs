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
                    println!("doesnt have equal sign continuing");
                    ast.push(AST::Variable(Variable{
                    var_type, name, span: Span(child.start_byte(), child.end_byte()),
                    id: child.id(), expression: None,
                }));
                    continue;
                }

                let nodes: Vec<Node> = children.collect();
                let expression = parse_expression(&nodes);

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

fn parse_expression(nodes: &Vec<Node>) -> Expression {
    const IntegerLiteral: u16 = TreeSitter::IntegerLiteal as u16;
    const BoolLiteral: u16 = TreeSitter::BoolLiteral as u16;
    const CharLiteral: u16 = TreeSitter::CharLiteral as u16;
    const FloatingPointLiteral: u16 = TreeSitter::FloatingPointLiteral as u16;
    const StringLiteral: u16 = TreeSitter::StringLiteral as u16;

    let mut expression_kind = match nodes[0].kind_id() {
        IntegerLiteral => ExpressionKind::Literal(Literal {
                id: nodes[0].id(),
                kind: LiteralKind::Int,
                value: Span(nodes[0].start_byte(), nodes[0].end_byte())
            }),
        BoolLiteral => ExpressionKind::Literal(Literal {
            id: nodes[0].id(),
            kind: LiteralKind::Bool,
            value: Span(nodes[0].start_byte(), nodes[0].end_byte())
        }),
        CharLiteral => ExpressionKind::Literal(Literal {
            id: nodes[0].id(),
            kind: LiteralKind::Char,
            value: Span(nodes[0].start_byte(), nodes[0].end_byte())
        }),
        FloatingPointLiteral => ExpressionKind::Literal(Literal {
            id: nodes[0].id(),
            kind: LiteralKind::Float,
            value: Span(nodes[0].start_byte(), nodes[0].end_byte())
        }),
        StringLiteral => ExpressionKind::Literal(Literal { 
            id: nodes[0].id(),
            kind: LiteralKind::String,
            value: Span(nodes[0].start_byte(), nodes[0].end_byte())
        }),
        _ => ExpressionKind::Literal(Literal {
            id: nodes[0].id(),
            kind: LiteralKind::Int,
            value: Span(nodes[0].start_byte(), nodes[0].end_byte())
        })
    };

    Expression { id: nodes[0].id(), span: Span(nodes[0].start_byte(), nodes.last().unwrap().end_byte()), kind: Box::new(expression_kind)}
}
