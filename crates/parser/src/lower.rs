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

pub fn lower(tree: &Tree, source: &str, source_name: &str) -> Result<Vec<Item>, Vec<LowerError>> {
    let root = tree.root_node();
    lower_source_file(&root, source, source_name)
}

fn lower_source_file(
    node: &Node,
    source: &str,
    source_name: &str,
) -> Result<Vec<Item>, Vec<LowerError>> {
    let mut items = Vec::new();
    let mut errors = Vec::new();
    let mut pending_attrs: Vec<Attr> = Vec::new();
    for i in 0..node.named_child_count() {
        if let Some(child) = node.named_child(i as u32) {
            match child.kind() {
                "attribute" => match lower_attr(&child, source, source_name) {
                    Ok(attr) => pending_attrs.push(attr),
                    Err(e) => errors.push(e),
                },
                "comment" => {}
                kind => {
                    let mut item = lower_item(&child, kind, source, source_name);
                    if let Ok(Item::Function(f)) = &mut item {
                        f.attrs = std::mem::take(&mut pending_attrs);
                    }
                    pending_attrs.clear();
                    match item {
                        Ok(item) => items.push(item),
                        Err(e) => errors.push(e),
                    }
                }
            }
        }
    }
    if errors.is_empty() {
        Ok(items)
    } else {
        Err(errors)
    }
}

fn lower_item(
    node: &Node,
    kind: &str,
    source: &str,
    source_name: &str,
) -> Result<Item, LowerError> {
    match kind {
        "function_definition" => {
            lower_function(node, Vec::new(), source, source_name).map(Item::Function)
        }
        "struct_definition" => Err(unimplemented(node, source, source_name, "structs")),
        "enum_definition" => Err(unimplemented(node, source, source_name, "enums")),
        kind => Err(invalid_kind(node, source, source_name, kind, "item")),
    }
}

fn lower_attr(node: &Node, source: &str, source_name: &str) -> Result<Attr, LowerError> {
    let span = SourceSpan::from(node.start_byte()..node.end_byte());
    let name = node_text(&child_by_field(node, "name", source, source_name)?, source);
    let mut args = Vec::new();
    for i in 0..node.named_child_count() {
        if let Some(child) = node.named_child(i as u32)
            && node.field_name_for_named_child(i as u32) != Some("name")
        {
            args.push(lower_expression(&child, source, source_name)?);
        }
    }
    Ok(Attr { span, name, args })
}

fn lower_function(
    node: &Node,
    attrs: Vec<Attr>,
    source: &str,
    source_name: &str,
) -> Result<FunctionDef, LowerError> {
    let span = SourceSpan::from(node.start_byte()..node.end_byte());
    let name = node_text(&child_by_field(node, "name", source, source_name)?, source);
    let params_node = child_by_field(node, "parameters", source, source_name)?;
    let params = lower_params(&params_node, source, source_name)?;

    let return_type = match node.child_by_field_name("return_type") {
        Some(ann) => Some(lower_type(&ann, source, source_name)?),
        None => None,
    };

    let body_node = child_by_field(node, "body", source, source_name)?;
    let body = lower_block(&body_node, source, source_name)?;

    Ok(FunctionDef {
        span,
        attrs,
        name,
        params,
        return_type,
        body,
    })
}

fn lower_params(node: &Node, source: &str, source_name: &str) -> Result<Vec<Param>, LowerError> {
    let mut params = Vec::new();
    for i in 0..node.named_child_count() {
        if let Some(child) = node.named_child(i as u32)
            && child.kind() == "parameter"
        {
            params.push(lower_param(&child, source, source_name)?);
        }
    }
    Ok(params)
}

fn lower_param(node: &Node, source: &str, source_name: &str) -> Result<Param, LowerError> {
    let span = SourceSpan::from(node.start_byte()..node.end_byte());
    let mutable = node.child_by_field_name("mut").is_some();
    let name = node_text(&child_by_field(node, "name", source, source_name)?, source);
    let type_ann = child_by_field(node, "type", source, source_name)?;
    let type_ = lower_type(&type_ann, source, source_name)?;
    Ok(Param {
        span,
        name,
        mutable,
        type_,
    })
}

fn lower_type(node: &Node, source: &str, source_name: &str) -> Result<Type, LowerError> {
    match node.kind() {
        "simple_type" => return lower_simple_type(node, source, source_name),
        "generic_type" => return lower_generic_type(node, source, source_name),
        "array_type" => return lower_array_type(node, source, source_name),
        _ => {}
    }
    for i in 0..node.named_child_count() {
        if let Some(child) = node.named_child(i as u32) {
            return match child.kind() {
                "simple_type" => lower_simple_type(&child, source, source_name),
                "generic_type" => lower_generic_type(&child, source, source_name),
                "array_type" => lower_array_type(&child, source, source_name),
                kind => Err(invalid_kind(node, source, source_name, kind, "type")),
            };
        }
    }
    Err(invalid_kind(node, source, source_name, node.kind(), "type"))
}

fn lower_simple_type(node: &Node, source: &str, source_name: &str) -> Result<Type, LowerError> {
    let child = node
        .named_child(0)
        .ok_or_else(|| span_error(node, source, source_name, "empty simple type"))?;
    match child.kind() {
        "primitive_type" => {
            let name = node_text(&child, source);
            Ok(match name.as_str() {
                "int8" => Type::Primitive(Primitive::Int8),
                "int16" => Type::Primitive(Primitive::Int16),
                "int32" => Type::Primitive(Primitive::Int32),
                "int64" => Type::Primitive(Primitive::Int64),
                "int128" => Type::Primitive(Primitive::Int128),
                "isize" => Type::Primitive(Primitive::ISize),
                "uint8" => Type::Primitive(Primitive::UInt8),
                "uint16" => Type::Primitive(Primitive::UInt16),
                "uint32" => Type::Primitive(Primitive::UInt32),
                "uint64" => Type::Primitive(Primitive::UInt64),
                "uint128" => Type::Primitive(Primitive::UInt128),
                "usize" => Type::Primitive(Primitive::USize),
                "float32" => Type::Primitive(Primitive::Float32),
                "float64" => Type::Primitive(Primitive::Float64),
                "int" => Type::Primitive(Primitive::Int64),
                "float" => Type::Primitive(Primitive::Float64),
                "bool" => Type::Primitive(Primitive::Bool),
                "char" => Type::Primitive(Primitive::Char),
                "string" => Type::Primitive(Primitive::String),
                "unit" => Type::Primitive(Primitive::Unit),
                _ => {
                    return Err(span_error(
                        node,
                        source,
                        source_name,
                        &format!("unknown primitive type `{name}`"),
                    ));
                }
            })
        }
        _ => Ok(Type::Named(node_text(&child, source))),
    }
}

fn lower_generic_type(node: &Node, source: &str, source_name: &str) -> Result<Type, LowerError> {
    let name = node_text(
        &node.named_child(0).ok_or_else(|| {
            span_error(
                node,
                source,
                source_name,
                "expected identifier in generic type",
            )
        })?,
        source,
    );
    let mut args = Vec::new();
    for i in 1..node.named_child_count() {
        if let Some(child) = node.named_child(i as u32) {
            args.push(lower_type(&child, source, source_name)?);
        }
    }
    Ok(Type::Generic { name, args })
}

fn lower_array_type(node: &Node, source: &str, source_name: &str) -> Result<Type, LowerError> {
    let children = children(node);
    if children.len() < 2 {
        return Err(span_error(
            node,
            source,
            source_name,
            "incomplete array type",
        ));
    }
    let element = lower_type(&children[0], source, source_name)?;
    let size_text = node_text(&children[1], source);
    let size: usize = size_text.parse().map_err(|_| {
        span_error(
            &children[1],
            source,
            source_name,
            &format!("invalid array size `{size_text}`"),
        )
    })?;
    Ok(Type::Array {
        element: Box::new(element),
        size,
    })
}

fn lower_block(node: &Node, source: &str, source_name: &str) -> Result<Vec<Stmt>, LowerError> {
    let mut stmts = Vec::new();
    let child_count = node.named_child_count();
    for i in 0..child_count {
        if let Some(child) = node.named_child(i as u32) {
            let is_last = i == child_count - 1;
            match lower_statement(&child, source, source_name) {
                Ok(Some(stmt)) => {
                    if is_last {
                        if let Stmt::Expr(Expr::If {
                            span: if_span,
                            condition,
                            then_block,
                            else_if,
                            else_block,
                        }) = stmt
                        {
                            stmts.push(Stmt::Value(
                                Expr::If {
                                    span: if_span,
                                    condition,
                                    then_block,
                                    else_if,
                                    else_block,
                                },
                                if_span,
                            ));
                        } else {
                            stmts.push(stmt);
                        }
                    } else {
                        stmts.push(stmt);
                    }
                }
                Ok(None) => {
                    if let Ok(expr) = lower_expression(&child, source, source_name) {
                        let span = SourceSpan::from(child.start_byte()..child.end_byte());
                        stmts.push(Stmt::Value(expr, span));
                    }
                }
                Err(e) => return Err(e),
            }
        }
    }
    Ok(stmts)
}

fn lower_statement(
    node: &Node,
    source: &str,
    source_name: &str,
) -> Result<Option<Stmt>, LowerError> {
    match node.kind() {
        "let_declaration" => lower_let(node, source, source_name).map(Some),
        "return_statement" => lower_return(node, source, source_name).map(Some),
        "expression_statement" => {
            for i in 0..node.named_child_count() {
                if let Some(child) = node.named_child(i as u32) {
                    return lower_expression(&child, source, source_name)
                        .map(|e| Some(Stmt::Expr(e)));
                }
            }
            Err(span_error(
                node,
                source,
                source_name,
                "empty expression statement",
            ))
        }
        "if_expression" => lower_if(node, source, source_name).map(|e| Some(Stmt::Expr(e))),
        "while_statement" => {
            let span = SourceSpan::from(node.start_byte()..node.end_byte());
            let named = children(node);
            let condition = match named.first() {
                Some(n) => lower_expression(n, source, source_name)?,
                None => {
                    return Err(span_error(
                        node,
                        source,
                        source_name,
                        "incomplete while: missing condition",
                    ));
                }
            };
            let body = match named.get(1) {
                Some(n) => lower_block(n, source, source_name)?,
                None => {
                    return Err(span_error(
                        node,
                        source,
                        source_name,
                        "incomplete while: missing body",
                    ));
                }
            };
            let break_stmt = Stmt::Break(span);
            let if_expr = Expr::If {
                span,
                condition: Box::new(condition),
                then_block: body,
                else_if: Vec::new(),
                else_block: Some(vec![break_stmt]),
            };
            Ok(Some(Stmt::Loop {
                span,
                body: vec![Stmt::Expr(if_expr)],
            }))
        }
        "loop_statement" => {
            let span = SourceSpan::from(node.start_byte()..node.end_byte());
            let body = match node.named_child(0) {
                Some(n) => lower_block(&n, source, source_name)?,
                None => {
                    return Err(span_error(
                        node,
                        source,
                        source_name,
                        "incomplete loop: missing body",
                    ));
                }
            };
            Ok(Some(Stmt::Loop { span, body }))
        }
        "break_statement" => {
            let span = SourceSpan::from(node.start_byte()..node.end_byte());
            Ok(Some(Stmt::Break(span)))
        }
        "continue_statement" => {
            let span = SourceSpan::from(node.start_byte()..node.end_byte());
            Ok(Some(Stmt::Continue(span)))
        }
        _ => Ok(None),
    }
}

fn lower_let(node: &Node, source: &str, source_name: &str) -> Result<Stmt, LowerError> {
    let span = SourceSpan::from(node.start_byte()..node.end_byte());
    let mutable = node.child_by_field_name("mut").is_some();
    let name = node_text(&child_by_field(node, "name", source, source_name)?, source);
    let type_ = find_type_annotation(node, source, source_name)?;
    let value = lower_any_expression(node, source, source_name)?;
    Ok(Stmt::Let {
        span,
        name,
        mutable,
        type_,
        value,
    })
}

fn find_type_annotation(
    node: &Node,
    source: &str,
    source_name: &str,
) -> Result<Option<Type>, LowerError> {
    for i in 0..node.named_child_count() {
        if let Some(child) = node.named_child(i as u32)
            && child.kind() == "type_annotation"
        {
            return lower_type(&child, source, source_name).map(Some);
        }
    }
    Ok(None)
}

fn lower_return(node: &Node, source: &str, source_name: &str) -> Result<Stmt, LowerError> {
    let span = SourceSpan::from(node.start_byte()..node.end_byte());
    for i in 0..node.named_child_count() {
        if let Some(child) = node.named_child(i as u32) {
            return lower_expression(&child, source, source_name)
                .map(|e| Stmt::Return(Some(e), span));
        }
    }
    Ok(Stmt::Return(None, span))
}

fn lower_if(node: &Node, source: &str, source_name: &str) -> Result<Expr, LowerError> {
    let span = SourceSpan::from(node.start_byte()..node.end_byte());
    let named = children(node);

    let condition = match named.first() {
        Some(n) => lower_expression(n, source, source_name)?,
        None => {
            return Err(span_error(
                node,
                source,
                source_name,
                "incomplete if: missing condition",
            ));
        }
    };
    let then_block = match named.get(1) {
        Some(n) => lower_block(n, source, source_name)?,
        None => {
            return Err(span_error(
                node,
                source,
                source_name,
                "incomplete if: missing body",
            ));
        }
    };

    let mut else_if = Vec::new();
    let mut else_block = None;

    if let Some(else_node) = named.get(2) {
        match else_node.kind() {
            "if_expression" => {
                let inner = lower_if(else_node, source, source_name)?;
                if let Expr::If {
                    condition: c,
                    then_block: t,
                    else_if: e,
                    else_block: el,
                    ..
                } = inner
                {
                    else_if.push((*c, t));
                    else_if.extend(e);
                    else_block = el;
                }
            }
            "block" => {
                else_block = Some(lower_block(else_node, source, source_name)?);
            }
            _ => {}
        }
    }

    Ok(Expr::If {
        span,
        condition: Box::new(condition),
        then_block,
        else_if,
        else_block,
    })
}

fn lower_expression(node: &Node, source: &str, source_name: &str) -> Result<Expr, LowerError> {
    let span = || SourceSpan::from(node.start_byte()..node.end_byte());
    match node.kind() {
        "identifier" => Ok(Expr::Ident(node_text(node, source), span())),
        "string_literal" => lower_string(node, source),
        "raw_string_literal" => lower_raw_string(node, source),
        "char_literal" => lower_char(node, source),
        "integer_literal" => lower_int(node, source),
        "float_literal" => lower_float(node, source),
        "bool_literal" => Ok(Expr::Bool(node_text(node, source) == "true", span())),
        "unit_literal" => Ok(Expr::Unit(span())),
        "call_expression" => lower_call(node, source, source_name),
        "binary_expression" => lower_binary(node, source, source_name),
        "parenthesized_expression" => {
            for i in 0..node.named_child_count() {
                if let Some(child) = node.named_child(i as u32) {
                    return lower_expression(&child, source, source_name)
                        .map(Box::new)
                        .map(|e| Expr::Paren(e, span()));
                }
            }
            Err(span_error(
                node,
                source,
                source_name,
                "empty parenthesized expression",
            ))
        }
        "block" => lower_block(node, source, source_name).map(|s| Expr::Block(s, span())),
        "array_expression" => {
            let children = children(node);
            let elements: Result<Vec<Expr>, _> = children
                .iter()
                .map(|c| lower_expression(c, source, source_name))
                .collect();
            elements.map(|e| Expr::Array(e, span()))
        }
        "index_expression" => {
            let children = children(node);
            if children.len() < 2 {
                return Err(span_error(
                    node,
                    source,
                    source_name,
                    "incomplete index expression",
                ));
            }
            let array = lower_expression(&children[0], source, source_name)?;
            let index = lower_expression(&children[1], source, source_name)?;
            Ok(Expr::Index {
                span: span(),
                array: Box::new(array),
                index: Box::new(index),
            })
        }
        "if_expression" => lower_if(node, source, source_name),
        kind => Err(invalid_kind(node, source, source_name, kind, "expression")),
    }
}

fn lower_string(node: &Node, source: &str) -> Result<Expr, LowerError> {
    let span = SourceSpan::from(node.start_byte()..node.end_byte());
    let raw = node_text(node, source);
    let content = if raw.starts_with('f') {
        let c = raw.trim_start_matches('f');
        &c[1..c.len() - 1]
    } else {
        &raw[1..raw.len() - 1]
    };
    Ok(Expr::String(content.to_string(), span))
}

fn lower_raw_string(node: &Node, source: &str) -> Result<Expr, LowerError> {
    let span = SourceSpan::from(node.start_byte()..node.end_byte());
    let raw = node_text(node, source);
    let content = &raw[2..raw.len() - 1];
    Ok(Expr::String(content.to_string(), span))
}

fn lower_char(node: &Node, source: &str) -> Result<Expr, LowerError> {
    let span = SourceSpan::from(node.start_byte()..node.end_byte());
    let raw = node_text(node, source);
    let c = raw.chars().nth(1).unwrap_or('\0');
    Ok(Expr::Char(c, span))
}

fn lower_int(node: &Node, source: &str) -> Result<Expr, LowerError> {
    let span = SourceSpan::from(node.start_byte()..node.end_byte());
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
        Ok(v) => Ok(Expr::Int(v, span)),
        Err(_) => Err(span_error(
            node,
            source,
            "",
            &format!("invalid integer literal `{raw}`"),
        )),
    }
}

fn lower_float(node: &Node, source: &str) -> Result<Expr, LowerError> {
    let span = SourceSpan::from(node.start_byte()..node.end_byte());
    let raw = node_text(node, source);
    match raw.parse::<f64>() {
        Ok(v) => Ok(Expr::Float(v, span)),
        Err(_) => Err(span_error(
            node,
            source,
            "",
            &format!("invalid float literal `{raw}`"),
        )),
    }
}

fn lower_call(node: &Node, source: &str, source_name: &str) -> Result<Expr, LowerError> {
    let span = SourceSpan::from(node.start_byte()..node.end_byte());
    let func_node = child_by_field(node, "function", source, source_name)?;
    let function = lower_expression(&func_node, source, source_name)?;

    let args_node = child_by_field(node, "arguments", source, source_name)?;
    let mut args = Vec::new();
    for i in 0..args_node.named_child_count() {
        if let Some(child) = args_node.named_child(i as u32) {
            args.push(lower_expression(&child, source, source_name)?);
        }
    }

    Ok(Expr::Call {
        span,
        function: Box::new(function),
        args,
    })
}

fn lower_binary(node: &Node, source: &str, source_name: &str) -> Result<Expr, LowerError> {
    let span = SourceSpan::from(node.start_byte()..node.end_byte());
    let children = children(node);
    if children.len() < 2 {
        return Err(span_error(
            node,
            source,
            source_name,
            "incomplete binary expression",
        ));
    }
    let left = lower_expression(&children[0], source, source_name)?;
    let op = lower_binary_op(
        &child_by_field(node, "operator", source, source_name)?,
        source,
        source_name,
    )?;
    let right = lower_expression(&children[1], source, source_name)?;
    Ok(Expr::Binary {
        span,
        left: Box::new(left),
        op,
        right: Box::new(right),
    })
}

fn lower_binary_op(node: &Node, source: &str, source_name: &str) -> Result<BinaryOp, LowerError> {
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
        "&&" | "and" => BinaryOp::And,
        "||" | "or" => BinaryOp::Or,
        "&" => BinaryOp::BitAnd,
        "|" => BinaryOp::BitOr,
        "^" => BinaryOp::BitXor,
        "<<" => BinaryOp::Shl,
        ">>" => BinaryOp::Shr,
        ".." => BinaryOp::Range,
        "..=" => BinaryOp::RangeInclusive,
        other => {
            return Err(span_error(
                node,
                source,
                source_name,
                &format!("unknown operator `{other}`"),
            ));
        }
    })
}

fn lower_any_expression(node: &Node, source: &str, source_name: &str) -> Result<Expr, LowerError> {
    for i in (0..node.named_child_count()).rev() {
        if let Some(child) = node.named_child(i as u32) {
            match node.field_name_for_named_child(i as u32) {
                Some(
                    "name" | "parameters" | "type" | "mut" | "arguments" | "return_type"
                    | "operator",
                ) => continue,
                _ => return lower_expression(&child, source, source_name),
            }
        }
    }
    Err(span_error(node, source, source_name, "expected expression"))
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

fn child_by_field<'a>(
    node: &Node<'a>,
    field: &str,
    source: &str,
    source_name: &str,
) -> Result<Node<'a>, LowerError> {
    node.child_by_field_name(field).ok_or_else(|| {
        span_error(
            node,
            source,
            source_name,
            &format!("missing field `{field}`"),
        )
    })
}

fn span_error(node: &Node, source: &str, source_name: &str, message: &str) -> LowerError {
    LowerError {
        message: message.to_string(),
        source: NamedSource::new(source_name, source.to_string()),
        span: SourceSpan::from(node.start_byte()..node.end_byte()),
    }
}

fn invalid_kind(
    node: &Node,
    source: &str,
    source_name: &str,
    kind: &str,
    context: &str,
) -> LowerError {
    span_error(
        node,
        source,
        source_name,
        &format!("unsupported {context}: `{kind}`"),
    )
}

fn unimplemented(node: &Node, source: &str, source_name: &str, feature: &str) -> LowerError {
    span_error(
        node,
        source,
        source_name,
        &format!("{feature} not yet implemented"),
    )
}
