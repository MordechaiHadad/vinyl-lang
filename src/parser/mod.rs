pub mod ast;
mod errors;

use ast::*;
use lasso::Rodeo;
use tree_sitter::{Language, Node, Parser};
use crate::parser::errors::{NonSupportedPrimitives, ParserError};

pub struct ParserEngine<'a> {
    pub source: &'a str,
    pub rodeo: &'a mut Rodeo,
    errors: Vec<ParserError>
}

impl ParserEngine<'_> {
    pub fn new(rodeo: &mut Rodeo, source: &str) -> Self {
        let errors: Vec<ParserError> = Vec::new();
        ParserEngine { errors, source, rodeo}
    }

    pub fn parse_into_ast(&mut self, node: &Node) -> Result<AST, Vec<ParserError>> {
        let mut errors: Vec<ParserError> = Vec::new();
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
            const VARIABLE_DECLERATION: u16 = TreeSitter::VariableDeclaration as u16;
            const FUNCTION_DECLARATION: u16 = TreeSitter::FunctionDeclaration as u16;

            match child.kind_id() {
                VARIABLE_DECLERATION => {
                    match self.parse_variable(&child) {
                        Ok(variable) => namespace.statements.push(Statement {
                                kind: StatementKind::Variable(variable),
                                span: Span {
                                    range: (child.start_byte(), child.end_byte()),
                                    file_id: "test.vnl",
                                },
                                id: child.id(),
                            }),
                        Err(error) => errors.push(error),
                    }
                }
                FUNCTION_DECLARATION => {
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

        if !self.errors.is_empty() { Err(&self.errors)}
        Ok(ast)
    }

    fn parse_function(&mut self, root: &Node) -> Function {
        const LEFT_PAREN: u16 = TreeSitter::LeftParen as u16;
        const RIGHT_PAREN: u16 = TreeSitter::RightParen as u16;
        const LEFT_CURLY: u16 = TreeSitter::LeftCurly as u16;
        const RIGHT_CURLY: u16 = TreeSitter::RightCurly as u16;

        let mut cursor = root.walk();

        let mut children = root.children(&mut cursor);

        let mut subchild = children.next().unwrap();

        let return_type = self
            .parse_type(Span {
                range: (subchild.start_byte(), subchild.end_byte()),
                file_id: "test.vnl",
            })
            .unwrap();

        subchild = children.next().unwrap();
        let name = self
            .rodeo
            .get_or_intern(&self.source[subchild.start_byte()..subchild.end_byte()]);

        subchild = children.next().unwrap();
        let parameters = self.parse_parameters(&subchild);

        subchild = children.next().unwrap();
        let body = self.parse_function_body(&subchild);

        Function {
            return_type,
            name,
            parameters,
            body,
            span: Span {
                range: (root.start_byte(), root.end_byte()),
                file_id: "test.vnl",
            },
            id: root.id(),
        }
    }

    fn parse_variable(&mut self, root: &Node) -> Result<Variable, ParserError> {

        const EQUAL_SIGN: u16 = TreeSitter::EqualSign as u16;

        let mut cursor = root.walk();

        let mut children = root.children(&mut cursor);

        let mut subchild = children.next().unwrap();

        let span = Span {
          range: (subchild.start_byte(), subchild.end_byte()),
            file_id: "test.vnl",
        };

        let var_type = self
            .parse_type(span).unwrap();

        subchild = children.next().unwrap();
        let name = self
            .rodeo
            .get_or_intern(&self.source[subchild.start_byte()..subchild.end_byte()]);

        subchild = children.next().unwrap();
        if subchild.kind_id() != EQUAL_SIGN {
            return Ok(Variable {
                var_type,
                name,
                span: Span {
                    range: (root.start_byte(), root.end_byte()),
                    file_id: "test.vnl",
                },
                id: root.id(),
                expression: None,
            });
        }

        let node = children.next().unwrap();
        let expression = self.parse_expression(&node);

        Ok(Variable {
            var_type,
            name,
            span: Span {
                range: (root.start_byte(), root.end_byte()),
                file_id: "test.vnl",
            },
            id: root.id(),
            expression: Some(expression),
        })
    }

    fn parse_function_body(&mut self, root: &Node) -> Option<Block> {
        const VARIABLE_DECLERATION: u16 = TreeSitter::VariableDeclaration as u16;
        const LITERAL: u16 = TreeSitter::Literal as u16;
        const BINARY_EXPRESSION: u16 = TreeSitter::BinaryExpression as u16;
        if root.child_count() <= 2 {
            return None;
        }

        let mut statements = Vec::new();
        let mut cursor = root.walk();
        for child in root.children(&mut cursor) {
            match child.kind_id() {
                VARIABLE_DECLERATION => {
                    match self.parse_variable(&child) {
                        Ok(var) => statements.push(Statement {
                            kind: StatementKind::Variable(var),
                            span: Span {
                                range: (child.start_byte(), child.end_byte()),
                                file_id: "test.vnl",
                            },
                            id: child.id(),
                        }),
                        Err(error) => todo!()
                    }

                }
                LITERAL | BINARY_EXPRESSION => {
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
        const PARAMETER: u16 = TreeSitter::Parameter as u16;
        if root.child_count() <= 2 {
            return None;
        }

        let mut parameters = Vec::new();
        let mut cursor = root.walk();
        for child in root.children(&mut cursor) {
            match child.kind_id() {
                PARAMETER => {
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
        let param_type = self
            .parse_type(Span {
                range: (subchild.start_byte(), subchild.end_byte()),
                file_id: "test.vnl",
            })
            .unwrap();

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

    fn parse_type(&mut self, span: Span) -> Result<Type, &str> {
        use PrimitiveType::*;
        use Type::*;
        let type_text = &self.source[span.range.0..span.range.1];

        match type_text {
            "bool" => Ok(Primitive(Bool)),
            "char" => Ok(Primitive(Char)),
            "int8" => Err("int8 type is not supported yet..."),
            "int16" => Err("int16 type is not supported yet..."),
            "int32" => Ok(Primitive(I32)),
            "int64" => Err("int64 type is not supported yet..."),
            "int128" => Err("int128 type is not supported yet..."),
            "uint8" => Err("uint8 type is not supported yet..."),
            "uint16" => Err("uint16 type is not supported yet..."),
            "uint32" => Ok(Primitive(U32)),
            "uint64" => Err("uint64 type is not supported yet..."),
            "uint128" => Err("uint128 type is not supported yet..."),
            "float32" => Err("float32 type is not supported yet..."),
            "float64" => Ok(Primitive(Float64)),
            "void" => Ok(Primitive(Void)),
            "var" => Ok(Primitive(Var)),
            "string" => Ok(Primitive(String)),
            _ => Err("what?"),
        }
    }

    fn parse_expression(&mut self, root: &Node) -> Expression {
        const LITERAL: u16 = TreeSitter::Literal as u16;
        const BINARY_EXPRESSION: u16 = TreeSitter::BinaryExpression as u16;
        const REFERENCE: u16 = TreeSitter::Reference as u16;

        let mut expression_kind = match root.kind_id() {
            LITERAL => {
                const INTEGER_LITERAL: u16 = TreeSitter::IntegerLiteral as u16;
                const BOOL_LITERAL: u16 = TreeSitter::BoolLiteral as u16;
                const CHAR_LITERAL: u16 = TreeSitter::CharLiteral as u16;
                const FLOATING_POINT_LITERAL: u16 = TreeSitter::FloatingPointLiteral as u16;
                const STRING_LITERAL: u16 = TreeSitter::StringLiteral as u16;

                let node = root.child(0).unwrap();
                match node.kind_id() {
                    INTEGER_LITERAL => ExpressionKind::Literal(Literal {
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
                    BOOL_LITERAL => ExpressionKind::Literal(Literal {
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
                    CHAR_LITERAL => ExpressionKind::Literal(Literal {
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
                    FLOATING_POINT_LITERAL => ExpressionKind::Literal(Literal {
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
                    STRING_LITERAL => ExpressionKind::Literal(Literal {
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
            BINARY_EXPRESSION => {
                const ADD_SIGN: u16 = TreeSitter::PlusSign as u16;
                const MINUS_SIGN: u16 = TreeSitter::MinusSign as u16;
                const MULTIPLY: u16 = TreeSitter::Multiply as u16;
                const DIVIDE: u16 = TreeSitter::Divide as u16;
                const SHIFT_LEFT: u16 = TreeSitter::ShiftLeft as u16;
                const SHIFT_RIGHT: u16 = TreeSitter::ShiftRight as u16;
                const AND: u16 = TreeSitter::And as u16;
                const OR: u16 = TreeSitter::Or as u16;
                const BIT_AND: u16 = TreeSitter::BitAnd as u16;
                const BIT_OR: u16 = TreeSitter::BitOr as u16;
                const BIT_XOR: u16 = TreeSitter::BitXor as u16;
                const EQUAL: u16 = TreeSitter::Equal as u16;
                const NOT_EQUAL: u16 = TreeSitter::NotEqual as u16;
                const LESS_THAN: u16 = TreeSitter::LessThan as u16;
                const LESS_THAN_OR_EQUAL: u16 = TreeSitter::LessThanOrEqual as u16;
                const GREATER_THAN: u16 = TreeSitter::GreaterThan as u16;
                const GREATER_THAN_OR_EQUAL: u16 = TreeSitter::GreaterThanOrEqual as u16;
                const MODULUS: u16 = TreeSitter::Modulus as u16;

                let mut cursor = root.walk();
                let mut children = root.children(&mut cursor);
                let left_expression = self.parse_expression(&children.next().unwrap());

                let operator = match children.next().unwrap().kind_id() {
                    ADD_SIGN => BinaryOperator {
                        kind: BinaryOperatorKind::Add,
                    },
                    MINUS_SIGN => BinaryOperator {
                        kind: BinaryOperatorKind::Subtract,
                    },
                    MULTIPLY => BinaryOperator {
                        kind: BinaryOperatorKind::Multiply,
                    },
                    DIVIDE => BinaryOperator {
                        kind: BinaryOperatorKind::Divide,
                    },
                    SHIFT_LEFT => BinaryOperator {
                        kind: BinaryOperatorKind::ShiftLeft,
                    },
                    SHIFT_RIGHT => BinaryOperator {
                        kind: BinaryOperatorKind::ShiftRight,
                    },
                    AND => BinaryOperator {
                        kind: BinaryOperatorKind::And,
                    },
                    OR => BinaryOperator {
                        kind: BinaryOperatorKind::Or,
                    },
                    BIT_AND => BinaryOperator {
                        kind: BinaryOperatorKind::BitAnd,
                    },
                    BIT_OR => BinaryOperator {
                        kind: BinaryOperatorKind::BitOr,
                    },
                    BIT_XOR => BinaryOperator {
                        kind: BinaryOperatorKind::BitXor,
                    },
                    EQUAL => BinaryOperator {
                        kind: BinaryOperatorKind::Equal,
                    },
                    NOT_EQUAL => BinaryOperator {
                        kind: BinaryOperatorKind::NotEqual,
                    },
                    LESS_THAN => BinaryOperator {
                        kind: BinaryOperatorKind::LessThan,
                    },
                    LESS_THAN_OR_EQUAL => BinaryOperator {
                        kind: BinaryOperatorKind::LessThanOrEqual,
                    },
                    GREATER_THAN => BinaryOperator {
                        kind: BinaryOperatorKind::GreaterThan,
                    },
                    GREATER_THAN_OR_EQUAL => BinaryOperator {
                        kind: BinaryOperatorKind::GreaterThanOrEqual,
                    },
                    MODULUS => BinaryOperator {
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
            REFERENCE => ExpressionKind::Reference(
                Mutability::Not,
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
}