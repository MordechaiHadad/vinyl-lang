use crate::parser::ast::{ExpressionKind, AST};
use convert_case::{Case, Casing};
use lasso::Rodeo;
use std::fs::File;
use std::io::prelude::*;
use tree_sitter::Language;

pub fn treesitter_to_enum(lang: &Language) {
    for i in 0..lang.node_kind_count() {
        let new_case = lang
            .node_kind_for_id(i as u16)
            .unwrap()
            .to_case(Case::UpperCamel);

        let new_name = match new_case.as_str() {
            "(" => "LeftParen",
            ")" => "RightParen",
            "{" => "LeftCurly",
            "}" => "RigthCurly",
            ";" => "SemiColon",
            "=" => "EqualSign",
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
            "End" => "EOF",
            "[" => "LeftBracket",
            "]" => "RightBracket",
            name => name,
        };

        println!("{new_name} = {i},");
    }
}
