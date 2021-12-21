use crate::parser::ast::{ExpressionKind, AST};
use lasso::Rodeo;
use std::fs::File;
use std::io::prelude::*;
use tree_sitter::Node;

pub fn treesitter_to_enum(root: &Node) {
    let kind = match root.kind() {
        "(" => "LeftParen",
        ")" => "RightParen",
        "{" => "LeftCurly",
        "}" => "RigthCurly",
        ";" => "SemiColon",
        "=" => "EqualSign",
        "variable_declaration" => "VariableDecleration",
        "function_decleration" => "FunctionDecleration",
        "primitive_type" => "PrimitiveType",
        "identifier" => "Identifier",
        "literal" => "Literal",
        "integer_literal" => "IntegerLiteral",
        "floating_point_literal" => "FloatingPointLiteral",
        "string_literal" => "StringLiteral",
        "char_literal" => "CharLiteral",
        "bool_literal" => "BoolLiteral",
        "binary_expression" => "BinaryExpression",
        "&&" => "And",
        "||" => "Or",
        "&" => "BitAnd",
        "|" => "BitOr",
        "^" => "BitXor",
        "==" => "Equal",
        "!=" => "NotEqual",
        "<" => "LessThan",
        "<=" => "LessThanOrEqual",
        ">" => "GreaterThan",
        ">=" => "GreaterThanOrEqual",
        "<<" => "ShiftLeft",
        ">>" => "ShiftRight",
        "+" => "PlusSign",
        "-" => "MinusSign",
        "*" => "Multiply",
        "/" => "Divide",
        "%" => "Modulus",
        "parameters" => "Parameters",
        "parameter" => "Parameter",
        "block" => "Block",
        "reference" => "Reference",
        _ => "",
    };
    let mut cursor = root.walk();

    println!("{} = {},", kind, root.kind_id());
    for child in root.children(&mut cursor) {
        treesitter_to_enum(&child);
    }
}
