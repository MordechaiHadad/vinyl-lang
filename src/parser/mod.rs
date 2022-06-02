pub mod ast;
mod errors;

use crate::parser::errors::{NonSupportedPrimitives, ParserError};
use ariadne::{sources, Color, Label, Report, ReportKind};
use ast::*;
use lasso::Rodeo;
use num_traits::FromPrimitive;
use std::convert::TryInto;
use std::ops::Range;
use tree_sitter::{Language, Node, Parser};

pub struct ParserEngine<'a> {
    pub source: &'a str,
    pub rodeo: &'a mut Rodeo,
    pub errors: Vec<ParserError>,
}

impl<'a> ParserEngine<'a> {
    pub fn new(rodeo: &'a mut Rodeo, source: &'a str) -> Self {
        let mut errors: Vec<ParserError> = Vec::new();
        ParserEngine {
            source,
            rodeo,
            errors,
        }
    }

    pub fn parse_into_ast(&mut self, node: &Node) -> AST {
        let mut cursor = node.walk();
        let mut ast = AST {
            namespaces: Vec::new(),
        };
        ast.namespaces.push(Namespace {
            statements: Vec::new(),
            span: Span {
                range: (node.start_byte(), node.end_byte()),
                file_id: "test.vnl",
            },
            id: node.id(),
            name: self.rodeo.get_or_intern("ALOHA"),
        });
        let mut namespace = ast.namespaces.first_mut().unwrap();

        for child in node.children(&mut cursor.clone()) {
            match FromPrimitive::from_u16(child.kind_id()) {
                Some(TreesitterNodeID::VariableDeclaration) => match self.parse_variable(&child) {
                    Ok(variable) => namespace.statements.push(Statement {
                        kind: StatementKind::Variable(variable),
                        span: Span {
                            range: (child.start_byte(), child.end_byte()),
                            file_id: "test.vnl",
                        },
                        id: child.id(),
                    }),
                    Err(_) => (),
                },
                Some(TreesitterNodeID::FunctionDeclaration) => {
                    let function = self.parse_function(&child);

                    namespace.statements.push(Statement {
                        kind: StatementKind::Function(function),
                        span: Span {
                            range: (child.start_byte(), child.end_byte()),
                            file_id: "test.vnl",
                        },
                        id: child.id(),
                    });
                }
                _ => (),
            }
        }

        if !self.errors.is_empty() {
            self.print_errors();
        }
        ast
    }

    fn parse_function(&mut self, root: &Node) -> Function {
        let mut cursor = root.walk();

        let mut children = root.children(&mut cursor);

        let mut function = Function {
            return_type: Type::Primitive(PrimitiveType::Void),
            name: self.rodeo.get_or_intern("NULL"),
            parameters: None,
            body: None,
            span: Span {
                range: (root.start_byte(), root.end_byte()),
                file_id: "test.vnl",
            },
            id: root.id(),
        };

        for subchild in children {
            match FromPrimitive::from_u16(subchild.kind_id()) {
                Some(TreesitterNodeID::PrimitiveType) => {
                    function.return_type =
                        self.parse_type(subchild.start_byte()..subchild.end_byte())
                }
                Some(TreesitterNodeID::Identifier) => {
                    function.name = self
                        .rodeo
                        .get_or_intern(&self.source[subchild.start_byte()..subchild.end_byte()])
                }
                Some(TreesitterNodeID::Parameters) => {
                    function.parameters = self.parse_parameters(&subchild)
                }
                Some(TreesitterNodeID::Block) => {
                    function.body = self.parse_function_body(&subchild)
                }
                _ => continue,
            }
        }

        function
    }

    fn parse_variable(&mut self, root: &Node) -> Result<Variable, ()> {
        let mut cursor = root.walk();

        let mut children = root.children(&mut cursor);

        let mut var = Variable {
            mutability: Mutability::Immutable,
            var_type: Type::Primitive(PrimitiveType::Void),
            expression: None,
            name: self.rodeo.get_or_intern("NULL"),
            span: Span {
                range: (root.start_byte(), root.end_byte()),
                file_id: "test.vnl",
            },
            id: root.id(),
        };

        for subchild in children {
            match FromPrimitive::from_u16(subchild.kind_id()) {
                Some(TreesitterNodeID::MutabilitySpecifier) => var.mutability = Mutability::Mutable,
                Some(TreesitterNodeID::Identifier) => {
                    var.name = self
                        .rodeo
                        .get_or_intern(&self.source[subchild.start_byte()..subchild.end_byte()])
                }
                Some(TreesitterNodeID::PrimitiveType) => {
                    var.var_type = self.parse_type(subchild.start_byte()..subchild.end_byte())
                }
                Some(TreesitterNodeID::BinaryExpression)
                | Some(TreesitterNodeID::ArrayCreationExpression)
                | Some(TreesitterNodeID::Literal) => {
                    var.expression = Some(self.parse_expression(&subchild))
                }
                _ => continue,
            }
        }

        Ok(var)
    }

    fn parse_function_body(&mut self, root: &Node) -> Option<Block> {
        if root.child_count() <= 2 {
            return None;
        }

        let mut statements = Vec::new();
        let mut cursor = root.walk();
        for child in root.children(&mut cursor) {
            match FromPrimitive::from_u16(child.kind_id()) {
                Some(TreesitterNodeID::VariableDeclaration) => match self.parse_variable(&child) {
                    Ok(var) => statements.push(Statement {
                        kind: StatementKind::Variable(var),
                        span: Span {
                            range: (child.start_byte(), child.end_byte()),
                            file_id: "test.vnl",
                        },
                        id: child.id(),
                    }),
                    Err(error) => todo!(),
                },
                Some(TreesitterNodeID::Literal)
                | Some(TreesitterNodeID::BinaryExpression)
                | Some(TreesitterNodeID::ArrayCreationExpression) => {
                    let expression = self.parse_expression(&child);
                    statements.push(Statement {
                        kind: StatementKind::Expression(expression),
                        span: Span {
                            range: (child.start_byte(), child.end_byte()),
                            file_id: "test.vnl",
                        },
                        id: child.id(),
                    })
                }
                _ => continue,
            }
        }

        Some(Block {
            statements,
            span: Span {
                range: (root.start_byte(), root.end_byte()),
                file_id: "test.vnl",
            },
            id: root.id(),
        })
    }

    fn parse_parameters(&mut self, root: &Node) -> Option<Parameters> {
        if root.child_count() <= 2 {
            return None;
        }

        let mut parameters = Vec::new();
        let mut cursor = root.walk();
        for child in root.children(&mut cursor) {
            match FromPrimitive::from_u16(child.kind_id()) {
                Some(TreesitterNodeID::Parameter) => {
                    let parameter = self.parse_parameter(&child);
                    parameters.push(parameter);
                }
                _ => continue,
            }
        }

        Some(Parameters {
            parameters,
            span: Span {
                range: (root.start_byte(), root.end_byte()),
                file_id: "test.vnl",
            },
            id: root.id(),
        })
    }

    fn parse_parameter(&mut self, root: &Node) -> Parameter {
        let mut cursor = root.walk();

        let mut children = root.children(&mut cursor);

        let mut subchild = children.next().unwrap();
        let param_type = self.parse_type(subchild.start_byte()..subchild.end_byte());

        subchild = children.next().unwrap();
        let name = self
            .rodeo
            .get_or_intern(&self.source[subchild.start_byte()..subchild.end_byte()]);
        Parameter {
            param_type,
            name,
            span: Span {
                range: (root.start_byte(), root.end_byte()),
                file_id: "test.vnl",
            },
            id: root.id(),
        }
    }

    fn parse_type(&mut self, range: Range<usize>) -> Type {
        use PrimitiveType::*;
        use Type::*;
        let type_text = &self.source[range];

        match type_text {
            "bool" => Primitive(Bool),
            "char" => Primitive(Char),
            "int8" => Primitive(I8),
            "int16" => Primitive(I16),
            "int32" => Primitive(I32),
            "int64" => Primitive(I64),
            "int128" => Primitive(I128),
            "uint8" => Primitive(U8),
            "uint16" => Primitive(U16),
            "uint32" => Primitive(U32),
            "uint64" => Primitive(U64),
            "uint128" => Primitive(U128),
            "float32" => Primitive(Float32),
            "float64" => Primitive(Float64),
            "void" => Primitive(Void),
            "var" => Primitive(Var),
            "string" => Primitive(String),
            _ => Primitive(Var),
        }
    }

    fn parse_expression(&mut self, root: &Node) -> Expression {
        let mut expression_kind = match FromPrimitive::from_u16(root.kind_id()) {
            Some(TreesitterNodeID::Literal) => {
                let node = root.child(0).unwrap();
                match FromPrimitive::from_u16(node.kind_id()) {
                    Some(TreesitterNodeID::IntegerLiteral) => ExpressionKind::Literal(Literal {
                        id: node.id(),
                        kind: LiteralKind::Int,
                        value: self
                            .rodeo
                            .get_or_intern(&self.source[node.start_byte()..node.end_byte()]),
                        span: Span {
                            range: (node.start_byte(), node.end_byte()),
                            file_id: "test.vnl",
                        },
                    }),
                    Some(TreesitterNodeID::BoolLiteral) => ExpressionKind::Literal(Literal {
                        id: node.id(),
                        kind: LiteralKind::Bool,
                        value: self
                            .rodeo
                            .get_or_intern(&self.source[node.start_byte()..node.end_byte()]),
                        span: Span {
                            range: (node.start_byte(), node.end_byte()),
                            file_id: "test.vnl",
                        },
                    }),
                    Some(TreesitterNodeID::CharLiteral) => ExpressionKind::Literal(Literal {
                        id: node.id(),
                        kind: LiteralKind::Char,
                        value: self.rodeo.get_or_intern(
                            &self.source[node.start_byte() + 1..node.end_byte() - 1],
                        ),
                        span: Span {
                            range: (node.start_byte(), node.end_byte()),
                            file_id: "test.vnl",
                        },
                    }),
                    Some(TreesitterNodeID::RealLiteral) => ExpressionKind::Literal(Literal {
                        id: node.id(),
                        kind: LiteralKind::Float,
                        value: self
                            .rodeo
                            .get_or_intern(&self.source[node.start_byte()..node.end_byte()]),
                        span: Span {
                            range: (node.start_byte(), node.end_byte()),
                            file_id: "test.vnl",
                        },
                    }),
                    Some(TreesitterNodeID::StringLiteral) => ExpressionKind::Literal(Literal {
                        id: node.id(),
                        kind: LiteralKind::String,
                        value: self.rodeo.get_or_intern(
                            &self.source[node.start_byte() + 1..node.end_byte() - 1],
                        ),
                        span: Span {
                            range: (node.start_byte(), node.end_byte()),
                            file_id: "test.vnl",
                        },
                    }),
                    _ => ExpressionKind::Literal(Literal {
                        id: node.id(),
                        kind: LiteralKind::Int,
                        value: self
                            .rodeo
                            .get_or_intern(&self.source[node.start_byte()..node.end_byte()]),
                        span: Span {
                            range: (node.start_byte(), node.end_byte()),
                            file_id: "test.vnl",
                        },
                    }),
                }
            }
            Some(TreesitterNodeID::BinaryExpression) => {
                let mut cursor = root.walk();
                let mut children = root.children(&mut cursor);
                let left_expression = self.parse_expression(&children.next().unwrap());

                let operator = match FromPrimitive::from_u16(children.next().unwrap().kind_id()) {
                    Some(TreesitterNodeID::PlusSign) => BinaryOperator {
                        kind: BinaryOperatorKind::Add,
                    },
                    Some(TreesitterNodeID::MinusSign) => BinaryOperator {
                        kind: BinaryOperatorKind::Subtract,
                    },
                    Some(TreesitterNodeID::Multiply) => BinaryOperator {
                        kind: BinaryOperatorKind::Multiply,
                    },
                    Some(TreesitterNodeID::Divide) => BinaryOperator {
                        kind: BinaryOperatorKind::Divide,
                    },
                    Some(TreesitterNodeID::ShiftLeft) => BinaryOperator {
                        kind: BinaryOperatorKind::ShiftLeft,
                    },
                    Some(TreesitterNodeID::ShiftRight) => BinaryOperator {
                        kind: BinaryOperatorKind::ShiftRight,
                    },
                    Some(TreesitterNodeID::And) => BinaryOperator {
                        kind: BinaryOperatorKind::And,
                    },
                    Some(TreesitterNodeID::Or) => BinaryOperator {
                        kind: BinaryOperatorKind::Or,
                    },
                    Some(TreesitterNodeID::BitAnd) => BinaryOperator {
                        kind: BinaryOperatorKind::BitAnd,
                    },
                    Some(TreesitterNodeID::BitOr) => BinaryOperator {
                        kind: BinaryOperatorKind::BitOr,
                    },
                    Some(TreesitterNodeID::BitXor) => BinaryOperator {
                        kind: BinaryOperatorKind::BitXor,
                    },
                    Some(TreesitterNodeID::EqualSign) => BinaryOperator {
                        kind: BinaryOperatorKind::Equal,
                    },
                    Some(TreesitterNodeID::NotEqual) => BinaryOperator {
                        kind: BinaryOperatorKind::NotEqual,
                    },
                    Some(TreesitterNodeID::LessThan) => BinaryOperator {
                        kind: BinaryOperatorKind::LessThan,
                    },
                    Some(TreesitterNodeID::LessThanOrEqual) => BinaryOperator {
                        kind: BinaryOperatorKind::LessThanOrEqual,
                    },
                    Some(TreesitterNodeID::GreaterThan) => BinaryOperator {
                        kind: BinaryOperatorKind::GreaterThan,
                    },
                    Some(TreesitterNodeID::GreaterThanOrEqual) => BinaryOperator {
                        kind: BinaryOperatorKind::GreaterThanOrEqual,
                    },
                    Some(TreesitterNodeID::Modulus) => BinaryOperator {
                        kind: BinaryOperatorKind::Modulus,
                    },
                    _ => BinaryOperator {
                        kind: BinaryOperatorKind::Add,
                    },
                };

                let right_expression = self.parse_expression(&children.next().unwrap());

                ExpressionKind::Binary(
                    operator,
                    Box::new(left_expression),
                    Box::new(right_expression),
                )
            }
            Some(TreesitterNodeID::Reference) => ExpressionKind::Reference(
                Mutability::Immutable,
                Identifier {
                    symbol: self
                        .rodeo
                        .get_or_intern(&self.source[root.start_byte()..root.end_byte()]),
                    span: Span {
                        range: (root.start_byte(), root.end_byte()),
                        file_id: "test.vnl",
                    },
                },
            ),
            _ => ExpressionKind::Literal(Literal {
                id: root.id(),
                kind: LiteralKind::Int,
                value: self
                    .rodeo
                    .get_or_intern(&self.source[root.start_byte()..root.end_byte()]),
                span: Span {
                    range: (root.start_byte(), root.end_byte()),
                    file_id: "test.vnl",
                },
            }),
        };

        Expression {
            id: root.id(),
            span: Span {
                range: (root.start_byte(), root.end_byte()),
                file_id: "test.vnl",
            },
            kind: Box::new(expression_kind),
        }
    }

    fn print_errors(&self) {
        use ParserError::*;
        let red = Color::Red;

        for error in &self.errors {
            match error {
                NonSupportedPrimitives(error) => {
                    Report::build(ReportKind::Error, error.span.file_id, error.span.range.0)
                        .with_code(1)
                        .with_message(error.error_message)
                        .with_label(Label::new(error.span).with_color(red))
                        .finish()
                        .print(sources(vec![("test.vnl", &self.source)]))
                        .unwrap();
                }
            }
        }
    }
}
