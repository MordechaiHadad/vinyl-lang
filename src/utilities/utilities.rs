use tree_sitter::Node;
use crate::parser::ast::{AST, ExpressionKind};
use std::io::prelude::*;
use std::fs::File;

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
        "integer_literal" => "integer_literal",
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
        "<=" =>  "LessThanOrEqual",
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
        _ => "",
    };
    let mut cursor = root.walk();

    println!("{} = {},", kind, root.kind_id());
    for child in root.children(&mut cursor) {
        treesitter_to_enum(&child);
    }
}

pub fn print_ast(ast: &Vec<AST>, source: &str) {
    let mut file = File::create("output.html").unwrap();

    writeln!(file, "
    <html>
        <head>
            <style>
                body {{
                    background-color: #2e3440;
                    color: #E5E9F0;
                }}
            </style>
        </head>
    <body>
    ");

    for child in ast {
        match child {
            AST::Function(function) => {
                writeln!(file, "<div style=\"background-color:#4c566a;\">");
                writeln!(file, "<h3>Function Decleration: {} [{}, {}]<h3>", function.id, function.span.0, function.span.1);
                writeln!(file, "<h3>Return Type: {:?}</h3>", function.return_type);
                writeln!(file, "</div>");
            },
            AST::Variable(variable) => {
                writeln!(file, "<div style=\"background-color:#4c566a;\">");
                writeln!(file, "<h3>Variable Decleration: {} [{}, {}]</h3>", variable.id, variable.span.0, variable.span.1);
                writeln!(file, "<h3>Type: {:?}</h3>", variable.var_type);
                match &variable.expression {
                    Some(expression) => {
                        writeln!(file, "<h3>Is Initialized?: True</h3>");

                        match &*expression.kind {
                            ExpressionKind::Binary(..) => {},
                            ExpressionKind::Literal(literal) => {
                                writeln!(file, "<h3>{:?} Literal Expression: {} [{}, {}]:", literal.kind, expression.id, expression.span.0, expression.span.1);
                                writeln!(file, "<h4>Value: {}", &source[literal.value.0..literal.value.1]);
                            }

                        }
                    },
                    None => {writeln!(file, "<h3>Is Initialized?: False</h3>");}
                }
                writeln!(file, "</div>");
            }
        }
    }

    writeln!(file, "
    </body>
    </html>
    ");
}