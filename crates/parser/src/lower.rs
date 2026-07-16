use miette::{Diagnostic, NamedSource, SourceSpan};
use std::error::Error;
use std::fmt;
use tree_sitter::{Node, Tree};

use crate::ast::*;

#[derive(Debug, Diagnostic)]
#[diagnostic()]
pub struct LowerError {
    pub message: String,
    #[source_code]
    pub source: NamedSource<String>,
    #[label]
    pub span: SourceSpan,
}

impl fmt::Display for LowerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl Error for LowerError {}

pub fn lower(tree: &Tree, source: &str) -> Result<Vec<Item>, Vec<LowerError>> {
    let root = tree.root_node();
    lower_source_file(&root, source)
}

fn lower_source_file(node: &Node, source: &str) -> Result<Vec<Item>, Vec<LowerError>> {
    let mut items = Vec::new();
    let mut errors = Vec::new();
    for i in 0..node.named_child_count() {
        if let Some(child) = node.named_child(i as u32) {
            match lower_item(&child, source) {
                Ok(item) => items.push(item),
                Err(e) => errors.push(e),
            }
        }
    }
    if errors.is_empty() { Ok(items) } else { Err(errors) }
}

fn lower_item(node: &Node, source: &str) -> Result<Item, LowerError> {
    match node.kind() {
        "function_definition" => lower_function(node, source).map(Item::Function),
        "struct_definition" => Err(unimplemented(node, source, "structs")),
        "enum_definition" => Err(unimplemented(node, source, "enums")),
        kind => Err(invalid_kind(node, source, kind, "item")),
    }
}

fn lower_function(node: &Node, source: &str) -> Result<FunctionDef, LowerError> {
    let name = node_text(&child_by_field(node, "name", source)?, source);
    let params_node = child_by_field(node, "parameters", source)?;
    let params = lower_params(&params_node, source)?;

    let return_type = match node.child_by_field_name("return_type") {
        Some(ann) => Some(lower_type(&ann, source)?),
        None => None,
    };

    let body_node = child_by_field(node, "body", source)?;
    let body = lower_block(&body_node, source)?;

    Ok(FunctionDef { name, params, return_type, body })
}

fn lower_params(node: &Node, source: &str) -> Result<Vec<Param>, LowerError> {
    let mut params = Vec::new();
    for i in 0..node.named_child_count() {
        if let Some(child) = node.named_child(i as u32)
            && child.kind() == "parameter" {
                params.push(lower_param(&child, source)?);
            }
    }
    Ok(params)
}

fn lower_param(node: &Node, source: &str) -> Result<Param, LowerError> {
    let mutable = node.child_by_field_name("mut").is_some();
    let name = node_text(&child_by_field(node, "name", source)?, source);
    let type_ann = child_by_field(node, "type", source)?;
    let type_ = lower_type(&type_ann, source)?;
    Ok(Param { name, mutable, type_ })
}

fn lower_type(node: &Node, source: &str) -> Result<Type, LowerError> {
    for i in 0..node.named_child_count() {
        if let Some(child) = node.named_child(i as u32) {
            return match child.kind() {
                "simple_type" => lower_simple_type(&child, source),
                "generic_type" => lower_generic_type(&child, source),
                kind => Err(invalid_kind(node, source, kind, "type")),
            };
        }
    }
    Err(invalid_kind(node, source, node.kind(), "type"))
}

fn lower_simple_type(node: &Node, source: &str) -> Result<Type, LowerError> {
    let child = node.named_child(0).ok_or_else(|| span_error(node, source, "empty simple type"))?;
    match child.kind() {
        "primitive_type" => {
            let name = node_text(&child, source);
            Ok(match name.as_str() {
                "int8" => Type::Primitive(Primitive::Int8),
                "int16" => Type::Primitive(Primitive::Int16),
                "int32" => Type::Primitive(Primitive::Int32),
                "int64" => Type::Primitive(Primitive::Int64),
                "int128" => Type::Primitive(Primitive::Int128),
                "uint8" => Type::Primitive(Primitive::UInt8),
                "uint16" => Type::Primitive(Primitive::UInt16),
                "uint32" => Type::Primitive(Primitive::UInt32),
                "uint64" => Type::Primitive(Primitive::UInt64),
                "uint128" => Type::Primitive(Primitive::UInt128),
                "float32" => Type::Primitive(Primitive::Float32),
                "float64" => Type::Primitive(Primitive::Float64),
                "bool" => Type::Primitive(Primitive::Bool),
                "char" => Type::Primitive(Primitive::Char),
                "string" => Type::Primitive(Primitive::String),
                _ => return Err(span_error(node, source, &format!("unknown primitive type `{name}`"))),
            })
        }
        _ => Ok(Type::Named(node_text(&child, source))),
    }
}

fn lower_generic_type(node: &Node, source: &str) -> Result<Type, LowerError> {
    let name = node_text(&node.named_child(0).ok_or_else(|| span_error(node, source, "expected identifier in generic type"))?, source);
    let mut args = Vec::new();
    for i in 1..node.named_child_count() {
        if let Some(child) = node.named_child(i as u32) {
            args.push(lower_type(&child, source)?);
        }
    }
    Ok(Type::Generic { name, args })
}

fn lower_block(node: &Node, source: &str) -> Result<Vec<Stmt>, LowerError> {
    let mut stmts = Vec::new();
    for i in 0..node.named_child_count() {
        if let Some(child) = node.named_child(i as u32) {
            match lower_statement(&child, source) {
                Ok(Some(stmt)) => stmts.push(stmt),
                Ok(None) => {}
                Err(e) => return Err(e),
            }
        }
    }
    Ok(stmts)
}

fn lower_statement(node: &Node, source: &str) -> Result<Option<Stmt>, LowerError> {
    match node.kind() {
        "let_declaration" => lower_let(node, source).map(Some),
        "return_statement" => lower_return(node, source).map(Some),
        "expression_statement" => {
            for i in 0..node.named_child_count() {
                if let Some(child) = node.named_child(i as u32) {
                    return lower_expression(&child, source).map(|e| Some(Stmt::Expr(e)));
                }
            }
            Err(span_error(node, source, "empty expression statement"))
        }
        "if_expression" => lower_if(node, source).map(Some),
        _ => Ok(None),
    }
}

fn lower_let(node: &Node, source: &str) -> Result<Stmt, LowerError> {
    let mutable = node.child_by_field_name("mut").is_some();
    let name = node_text(&child_by_field(node, "name", source)?, source);
    let type_ = find_type_annotation(node, source)?;
    let value = lower_any_expression(node, source)?;
    Ok(Stmt::Let { name, mutable, type_, value })
}

fn find_type_annotation(node: &Node, source: &str) -> Result<Option<Type>, LowerError> {
    for i in 0..node.named_child_count() {
        if let Some(child) = node.named_child(i as u32)
            && child.kind() == "type_annotation" {
                return lower_type(&child, source).map(Some);
            }
    }
    Ok(None)
}

fn lower_return(node: &Node, source: &str) -> Result<Stmt, LowerError> {
    for i in 0..node.named_child_count() {
        if let Some(child) = node.named_child(i as u32) {
            return lower_expression(&child, source).map(|e| Stmt::Return(Some(e)));
        }
    }
    Ok(Stmt::Return(None))
}

fn lower_if(node: &Node, source: &str) -> Result<Stmt, LowerError> {
    let named = children(node);

    let condition = lower_expression(&named[0], source)?;
    let then_block = lower_block(&named[1], source)?;

    let mut else_if = Vec::new();
    let mut else_block = None;

    if named.len() > 2 {
        match named[2].kind() {
            "if_expression" => {
                let inner = lower_if(&named[2], source)?;
                if let Stmt::If { condition: c, then_block: t, else_if: e, else_block: el } = inner {
                    else_if.push((c, t));
                    else_if.extend(e);
                    else_block = el;
                }
            }
            "block" => {
                else_block = Some(lower_block(&named[2], source)?);
            }
            _ => {}
        }
    }

    Ok(Stmt::If { condition, then_block, else_if, else_block })
}

fn lower_expression(node: &Node, source: &str) -> Result<Expr, LowerError> {
    match node.kind() {
        "identifier" => Ok(Expr::Ident(node_text(node, source))),
        "string_literal" => lower_string(node, source),
        "integer_literal" => lower_int(node, source),
        "float_literal" => lower_float(node, source),
        "bool_literal" => Ok(Expr::Bool(node_text(node, source) == "true")),
        "call_expression" => lower_call(node, source),
        "binary_expression" => lower_binary(node, source),
        "parenthesized_expression" => {
            for i in 0..node.named_child_count() {
                if let Some(child) = node.named_child(i as u32) {
                    return lower_expression(&child, source).map(Box::new).map(Expr::Paren);
                }
            }
            Err(span_error(node, source, "empty parenthesized expression"))
        }
        "block" => lower_block(node, source).map(Expr::Block),
        kind => Err(invalid_kind(node, source, kind, "expression")),
    }
}

fn lower_string(node: &Node, source: &str) -> Result<Expr, LowerError> {
    let raw = node_text(node, source);
    let content = if raw.starts_with('f') {
        let c = raw.trim_start_matches('f');
        &c[1..c.len() - 1]
    } else {
        &raw[1..raw.len() - 1]
    };
    Ok(Expr::String(content.to_string()))
}

fn lower_int(node: &Node, source: &str) -> Result<Expr, LowerError> {
    let raw = node_text(node, source);
    let val = if let Some(hex) = raw.strip_prefix("0x") {
        i128::from_str_radix(hex, 16)
    } else if let Some(oct) = raw.strip_prefix("0o") {
        i128::from_str_radix(oct, 8)
    } else if let Some(bin) = raw.strip_prefix("0b") {
        i128::from_str_radix(bin, 2)
    } else {
        raw.parse()
    };
    match val {
        Ok(v) => Ok(Expr::Int(v)),
        Err(_) => Err(span_error(node, source, &format!("invalid integer literal `{raw}`"))),
    }
}

fn lower_float(node: &Node, source: &str) -> Result<Expr, LowerError> {
    let raw = node_text(node, source);
    match raw.parse::<f64>() {
        Ok(v) => Ok(Expr::Float(v)),
        Err(_) => Err(span_error(node, source, &format!("invalid float literal `{raw}`"))),
    }
}

fn lower_call(node: &Node, source: &str) -> Result<Expr, LowerError> {
    let func_node = child_by_field(node, "function", source)?;
    let function = lower_expression(&func_node, source)?;

    let args_node = child_by_field(node, "arguments", source)?;
    let mut args = Vec::new();
    for i in 0..args_node.named_child_count() {
        if let Some(child) = args_node.named_child(i as u32) {
            args.push(lower_expression(&child, source)?);
        }
    }

    Ok(Expr::Call { function: Box::new(function), args })
}

fn lower_binary(node: &Node, source: &str) -> Result<Expr, LowerError> {
    let children = children(node);
    if children.len() < 3 {
        return Err(span_error(node, source, "incomplete binary expression"));
    }
    let left = lower_expression(&children[0], source)?;
    let op = lower_binary_op(&children[1], source)?;
    let right = lower_expression(&children[2], source)?;
    Ok(Expr::Binary { left: Box::new(left), op, right: Box::new(right) })
}

fn lower_binary_op(node: &Node, source: &str) -> Result<BinaryOp, LowerError> {
    Ok(match node_text(node, source).as_str() {
        "+" => BinaryOp::Add,
        "-" => BinaryOp::Sub,
        "*" => BinaryOp::Mul,
        "/" => BinaryOp::Div,
        "%" => BinaryOp::Rem,
        "**" => BinaryOp::Pow,
        "//" => BinaryOp::FloorDiv,
        "==" => BinaryOp::Eq,
        "!=" => BinaryOp::Ne,
        "<" => BinaryOp::Lt,
        ">" => BinaryOp::Gt,
        "<=" => BinaryOp::Le,
        ">=" => BinaryOp::Ge,
        "&&" => BinaryOp::And,
        "||" => BinaryOp::Or,
        "&" => BinaryOp::BitAnd,
        "|" => BinaryOp::BitOr,
        "^" => BinaryOp::BitXor,
        "<<" => BinaryOp::Shl,
        ">>" => BinaryOp::Shr,
        ".." => BinaryOp::Range,
        "..=" => BinaryOp::RangeInclusive,
        other => return Err(span_error(node, source, &format!("unknown operator `{other}`"))),
    })
}

fn lower_any_expression(node: &Node, source: &str) -> Result<Expr, LowerError> {
    for i in (0..node.named_child_count()).rev() {
        if let Some(child) = node.named_child(i as u32) {
            match child.kind() {
                "identifier" | "type_annotation" | "parameters" => continue,
                _ => return lower_expression(&child, source),
            }
        }
    }
    Err(span_error(node, source, "expected expression"))
}

fn children<'a>(node: &'a Node<'a>) -> Vec<Node<'a>> {
    let mut v = Vec::new();
    for i in 0..node.named_child_count() {
        if let Some(child) = node.named_child(i as u32) {
            v.push(child);
        }
    }
    v
}

fn node_text(node: &Node, source: &str) -> String {
    source[node.start_byte()..node.end_byte()].to_string()
}

fn child_by_field<'a>(node: &Node<'a>, field: &str, source: &str) -> Result<Node<'a>, LowerError> {
    node.child_by_field_name(field).ok_or_else(|| {
        span_error(node, source, &format!("missing field `{field}`"))
    })
}

fn span_error(node: &Node, source: &str, message: &str) -> LowerError {
    LowerError {
        message: message.to_string(),
        source: NamedSource::new("", source.to_string()),
        span: SourceSpan::from(node.start_byte()..node.end_byte()),
    }
}

fn invalid_kind(node: &Node, source: &str, kind: &str, context: &str) -> LowerError {
    span_error(node, source, &format!("unsupported {context}: `{kind}`"))
}

fn unimplemented(node: &Node, source: &str, feature: &str) -> LowerError {
    span_error(node, source, &format!("{feature} not yet implemented"))
}
