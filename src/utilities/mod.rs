use crate::parser::ast::{ExpressionKind, AST};
use convert_case::{Case, Casing};
use lasso::Rodeo;
use std::fs::File;
use std::io::prelude::*;
use tree_sitter::Language;

pub fn convert_treesitter_nodes(lang: &Language) {
    for i in 0..lang.node_kind_count() {
        if !lang.node_kind_is_visible(i as u16) {
            continue;
        }
        let node_kind = lang.node_kind_for_id(i as u16).unwrap();

        let new_case = if node_kind != "-" {
            node_kind.to_case(Case::UpperCamel)
        } else {
            String::from(node_kind)
        };

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
            "[" => "LeftBracket",
            "]" => "RightBracket",
            name => name,
        };

        println!("{new_name} = {i},");
    }
}
