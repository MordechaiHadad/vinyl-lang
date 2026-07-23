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
                    if let Ok(ref mut item) = item {
                        match item {
                            Item::Function(f) => f.attrs = std::mem::take(&mut pending_attrs),
                            Item::Struct(s) => {
                                s.attrs = std::mem::take(&mut pending_attrs);
                            }
                            Item::TupleStruct(t) => {
                                t.attrs = std::mem::take(&mut pending_attrs);
                            }
                            Item::Enum(e) => e.attrs = std::mem::take(&mut pending_attrs),
                        }
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
        "struct_definition" => {
            lower_struct_definition(node, Vec::new(), source, source_name).map(Item::Struct)
        }
        "tuple_definition" => {
            lower_tuple_definition(node, Vec::new(), source, source_name).map(Item::TupleStruct)
        }
        "enum_definition" => {
            lower_enum_definition(node, Vec::new(), source, source_name).map(Item::Enum)
        }
        kind => Err(invalid_kind(node, source, source_name, kind, "item")),
    }
}

fn lower_struct_definition(
    node: &Node,
    attrs: Vec<Attr>,
    source: &str,
    source_name: &str,
) -> Result<StructDef, LowerError> {
    let span = SourceSpan::from(node.start_byte()..node.end_byte());
    let name = node_text(&child_by_field(node, "name", source, source_name)?, source);
    let mut fields = Vec::new();
    for i in 0..node.named_child_count() {
        if let Some(child) = node.named_child(i as u32)
            && child.kind() == "field_definition"
        {
            fields.push(lower_field_definition(&child, source, source_name)?);
        }
    }
    Ok(StructDef { span, attrs, name, fields })
}

fn lower_field_definition(node: &Node, source: &str, source_name: &str) -> Result<Field, LowerError> {
    let span = SourceSpan::from(node.start_byte()..node.end_byte());
    let name = node_text(&child_by_field(node, "name", source, source_name)?, source);
    let type_ = lower_type_annotation_child(node, source, source_name)?;
    Ok(Field { span, name, type_ })
}

fn lower_tuple_definition(
    node: &Node,
    attrs: Vec<Attr>,
    source: &str,
    source_name: &str,
) -> Result<TupleStructDef, LowerError> {
    let span = SourceSpan::from(node.start_byte()..node.end_byte());
    let name = node_text(&child_by_field(node, "name", source, source_name)?, source);
    let mut types = Vec::new();
    for i in 0..node.named_child_count() {
        if let Some(child) = node.named_child(i as u32)
            && child.kind() != "identifier"
        {
            types.push(lower_type(&child, source, source_name)?);
        }
    }
    Ok(TupleStructDef { span, attrs, name, types })
}

fn lower_enum_definition(
    node: &Node,
    attrs: Vec<Attr>,
    source: &str,
    source_name: &str,
) -> Result<EnumDef, LowerError> {
    let span = SourceSpan::from(node.start_byte()..node.end_byte());
    let name = node_text(&child_by_field(node, "name", source, source_name)?, source);
    let mut variants = Vec::new();
    for i in 0..node.named_child_count() {
        if let Some(child) = node.named_child(i as u32)
            && child.kind() == "enum_variant"
        {
            variants.push(lower_enum_variant(&child, source, source_name)?);
        }
    }
    Ok(EnumDef { span, attrs, name, variants })
}

fn lower_enum_variant(node: &Node, source: &str, source_name: &str) -> Result<EnumVariant, LowerError> {
    let span = SourceSpan::from(node.start_byte()..node.end_byte());
    let name = node_text(&child_by_field(node, "name", source, source_name)?, source);
    let child_count = node.named_child_count();
    let data = if child_count > 1 {
        let first_body = node.named_child(1).unwrap();
        match first_body.kind() {
            "field_definition" => {
                let mut fields = Vec::new();
                for i in 1..child_count {
                    if let Some(child) = node.named_child(i as u32) {
                        fields.push(lower_field_definition(&child, source, source_name)?);
                    }
                }
                Some(EnumVariantData::Struct(fields))
            }
            _ => {
                let mut types = Vec::new();
                for i in 1..child_count {
                    if let Some(child) = node.named_child(i as u32) {
                        types.push(lower_type(&child, source, source_name)?);
                    }
                }
                Some(EnumVariantData::Tuple(types))
            }
        }
    } else {
        None
    };
    Ok(EnumVariant { span, name, data })
}

fn lower_type_annotation_child(node: &Node, source: &str, source_name: &str) -> Result<Type, LowerError> {
    for i in 0..node.named_child_count() {
        if let Some(child) = node.named_child(i as u32)
            && child.kind() == "type_annotation"
        {
            return lower_type(&child, source, source_name);
        }
    }
    for i in 0..node.named_child_count() {
        if let Some(child) = node.named_child(i as u32) {
            match child.kind() {
                "simple_type" | "generic_type" | "array_type" | "reference_type" => {
                    return lower_type(&child, source, source_name);
                }
                _ => {}
            }
        }
    }
    Err(span_error(node, source, source_name, "expected type"))
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
        "reference_type" => return lower_reference_type(node, source, source_name),
        "tuple_type" => return lower_tuple_type(node, source, source_name),
        _ => {}
    }
    for i in 0..node.named_child_count() {
        if let Some(child) = node.named_child(i as u32) {
            return match child.kind() {
                "simple_type" => lower_simple_type(&child, source, source_name),
                "generic_type" => lower_generic_type(&child, source, source_name),
                "array_type" => lower_array_type(&child, source, source_name),
                "reference_type" => lower_reference_type(&child, source, source_name),
                "tuple_type" => lower_tuple_type(&child, source, source_name),
                kind => Err(invalid_kind(node, source, source_name, kind, "type")),
            };
        }
    }
    Err(invalid_kind(node, source, source_name, node.kind(), "type"))
}

fn lower_reference_type(node: &Node, source: &str, source_name: &str) -> Result<Type, LowerError> {
    for i in 0..node.named_child_count() {
        if let Some(child) = node.named_child(i as u32) {
            let inner = lower_type(&child, source, source_name)?;
            return Ok(Type::Ref(Box::new(inner)));
        }
    }
    Err(span_error(
        node,
        source,
        source_name,
        "empty reference type",
    ))
}

fn lower_tuple_type(node: &Node, source: &str, source_name: &str) -> Result<Type, LowerError> {
    let mut elements = Vec::new();
    for i in 0..node.named_child_count() {
        if let Some(child) = node.named_child(i as u32) {
            elements.push(lower_type(&child, source, source_name)?);
        }
    }
    Ok(Type::Tuple(elements))
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
        "assignment_statement" => lower_assignment(node, source, source_name).map(Some),
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

fn lower_assignment(node: &Node, source: &str, source_name: &str) -> Result<Stmt, LowerError> {
    let span = SourceSpan::from(node.start_byte()..node.end_byte());
    let named = children(node);
    if named.len() < 2 {
        return Err(span_error(
            node,
            source,
            source_name,
            "incomplete assignment",
        ));
    }

    let target = lower_assign_target(&named[0], source, source_name)?;
    let op = lower_assign_op(node, source, source_name)?;
    let value = lower_expression(&named[1], source, source_name)?;

    Ok(Stmt::Assign {
        span,
        target,
        op,
        value: Box::new(value),
    })
}

fn lower_assign_target(
    node: &Node,
    source: &str,
    source_name: &str,
) -> Result<AssignTarget, LowerError> {
    let span = || SourceSpan::from(node.start_byte()..node.end_byte());
    match node.kind() {
        "identifier" => Ok(AssignTarget::Ident(node_text(node, source), span())),
        "index_expression" => {
            let children = children(node);
            if children.len() < 2 {
                return Err(span_error(
                    node,
                    source,
                    source_name,
                    "incomplete index expression in assignment",
                ));
            }
            let array = lower_expression(&children[0], source, source_name)?;
            let index = lower_expression(&children[1], source, source_name)?;
            Ok(AssignTarget::Index {
                span: span(),
                array: Box::new(array),
                index: Box::new(index),
            })
        }
        _ => Err(span_error(
            node,
            source,
            source_name,
            &format!("invalid assignment target: `{}`", node.kind()),
        )),
    }
}

fn lower_assign_op(node: &Node, source: &str, source_name: &str) -> Result<AssignOp, LowerError> {
    let op_node = node
        .child_by_field_name("operator")
        .ok_or_else(|| span_error(node, source, source_name, "missing assignment operator"))?;
    let text = node_text(&op_node, source);
    Ok(match text.as_str() {
        "=" => AssignOp::Eq,
        "+=" => AssignOp::AddEq,
        "-=" => AssignOp::SubEq,
        "*=" => AssignOp::MulEq,
        "/=" => AssignOp::DivEq,
        "%=" => AssignOp::RemEq,
        "&=" => AssignOp::BitAndEq,
        "|=" => AssignOp::BitOrEq,
        "^=" => AssignOp::BitXorEq,
        "<<=" => AssignOp::ShlEq,
        ">>=" => AssignOp::ShrEq,
        other => {
            return Err(span_error(
                &op_node,
                source,
                source_name,
                &format!("unknown assignment operator `{other}`"),
            ));
        }
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

    let mut index = 2;
    while index + 1 < named.len() && named[index + 1].kind() == "block" {
        let condition = lower_expression(&named[index], source, source_name)?;
        let block = lower_block(&named[index + 1], source, source_name)?;
        else_if.push((condition, block));
        index += 2;
    }
    if let Some(else_node) = named.get(index)
        && else_node.kind() == "block"
    {
        else_block = Some(lower_block(else_node, source, source_name)?);
    }

    Ok(Expr::If {
        span,
        condition: Box::new(condition),
        then_block,
        else_if,
        else_block,
    })
}

fn lower_match(node: &Node, source: &str, source_name: &str) -> Result<Expr, LowerError> {
    let span = SourceSpan::from(node.start_byte()..node.end_byte());
    let children = children(node);
    if children.is_empty() {
        return Err(span_error(node, source, source_name, "incomplete match expression"));
    }
    let value = lower_expression(&children[0], source, source_name)?;
    let mut arms = Vec::new();
    for i in 1..children.len() {
        if children[i].kind() == "match_arm" {
            arms.push(lower_match_arm(&children[i], source, source_name)?);
        }
    }
    Ok(Expr::Match { span, value: Box::new(value), arms })
}

fn lower_match_arm(node: &Node, source: &str, source_name: &str) -> Result<MatchArm, LowerError> {
    let span = SourceSpan::from(node.start_byte()..node.end_byte());
    let children = children(node);
    if children.len() < 2 {
        return Err(span_error(node, source, source_name, "incomplete match arm"));
    }
    let pattern = lower_pattern(&children[0], source, source_name)?;
    let body = Box::new(lower_expression(&children[1], source, source_name)?);
    Ok(MatchArm { span, pattern, body })
}

fn lower_pattern(node: &Node, source: &str, source_name: &str) -> Result<Pattern, LowerError> {
    let span = || SourceSpan::from(node.start_byte()..node.end_byte());
    match node.kind() {
        "wildcard_pattern" => Ok(Pattern::Wildcard(span())),
        "identifier_pattern" => {
            let name = node_text(&node.named_child(0).unwrap_or(*node), source);
            Ok(Pattern::Ident(name, span()))
        }
        "literal_pattern" => lower_literal_pattern(node, source, source_name),
        "struct_pattern" => lower_struct_pattern(node, source, source_name),
        "tuple_pattern" => {
            let patterns = children(node).iter()
                .map(|c| lower_pattern(c, source, source_name))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Pattern::Tuple(patterns, span()))
        }
        "enum_variant_pattern" => {
            let children = children(node);
            if children.is_empty() {
                return Err(span_error(node, source, source_name, "incomplete enum variant pattern"));
            }
            let name = node_text(&children[0], source);
            let patterns = children[1..].iter()
                .map(|c| lower_pattern(c, source, source_name))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Pattern::EnumVariant { span: span(), name, patterns })
        }
        kind => match node.named_child(0) {
            Some(child) => lower_pattern(&child, source, source_name),
            None => Err(invalid_kind(node, source, source_name, kind, "pattern")),
        },
    }
}

fn lower_literal_pattern(node: &Node, source: &str, source_name: &str) -> Result<Pattern, LowerError> {
    for i in 0..node.named_child_count() {
        if let Some(child) = node.named_child(i as u32) {
            return match child.kind() {
                "integer_literal" => {
                    let raw = node_text(&child, source);
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
                        Ok(v) => Ok(Pattern::Literal(LiteralPattern::Int(v), SourceSpan::from(child.start_byte()..child.end_byte()))),
                        Err(_) => Err(span_error(&child, source, "", &format!("invalid integer literal `{raw}`"))),
                    }
                }
                "bool_literal" => {
                    let v = node_text(&child, source) == "true";
                    Ok(Pattern::Literal(LiteralPattern::Bool(v), SourceSpan::from(child.start_byte()..child.end_byte())))
                }
                "char_literal" => {
                    let raw = node_text(&child, source);
                    let c = raw.chars().nth(1).unwrap_or('\0');
                    Ok(Pattern::Literal(LiteralPattern::Char(c), SourceSpan::from(child.start_byte()..child.end_byte())))
                }
                "string_literal" => {
                    let raw = node_text(&child, source);
                    let content = &raw[1..raw.len() - 1];
                    Ok(Pattern::Literal(LiteralPattern::String(content.to_string()), SourceSpan::from(child.start_byte()..child.end_byte())))
                }
                _ => Err(span_error(&child, source, source_name, &format!("unsupported literal pattern: `{}`", child.kind()))),
            };
        }
    }
    Err(span_error(node, source, source_name, "empty literal pattern"))
}

fn lower_struct_pattern(node: &Node, source: &str, source_name: &str) -> Result<Pattern, LowerError> {
    let span = SourceSpan::from(node.start_byte()..node.end_byte());
    let children = children(node);
    if children.is_empty() {
        return Err(span_error(node, source, source_name, "incomplete struct pattern"));
    }
    let name = node_text(&children[0], source);
    let mut fields = Vec::new();
    for i in 1..children.len() {
        let field_node = &children[i];
        let field_name = node_text(&child_by_field(field_node, "name", source, source_name)?, source);
        let pattern = match field_node.named_child(1) {
            Some(sub_pattern) => lower_pattern(&sub_pattern, source, source_name)?,
            None => Pattern::Ident(field_name.clone(), SourceSpan::from(field_node.start_byte()..field_node.end_byte())),
        };
        fields.push((field_name, pattern));
    }
    Ok(Pattern::Struct { span, name, fields })
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
        "unary_expression" => lower_unary(node, source, source_name),
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
        "tuple_expression" => {
            let children = children(node);
            let elements: Result<Vec<Expr>, _> = children
                .iter()
                .map(|c| lower_expression(c, source, source_name))
                .collect();
            elements.map(|e| Expr::Tuple(e, span()))
        }
        "field_access_expression" => {
            let children = children(node);
            if children.len() < 2 {
                return Err(span_error(node, source, source_name, "incomplete field access"));
            }
            let object = lower_expression(&children[0], source, source_name)?;
            let name = node_text(&child_by_field(node, "field", source, source_name)?, source);
            Ok(Expr::Field { span: span(), object: Box::new(object), name })
        }
        "match_expression" => lower_match(node, source, source_name),
        "enum_variant_expression" => {
            let type_name = node_text(
                &child_by_field(node, "type", source, source_name)?,
                source,
            );
            let variant_name = node_text(
                &child_by_field(node, "variant", source, source_name)?,
                source,
            );
            let args = if let Some(arg_node) = child_by_field_opt(node, "arguments") {
                let arg_children: Vec<Expr> = children(&arg_node)
                    .iter()
                    .map(|c| lower_expression(c, source, source_name))
                    .collect::<Result<Vec<_>, _>>()?;
                arg_children
            } else {
                Vec::new()
            };
            Ok(Expr::EnumVariant {
                span: span(),
                type_name,
                variant_name,
                args,
            })
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

fn lower_unary(node: &Node, source: &str, source_name: &str) -> Result<Expr, LowerError> {
    let span = SourceSpan::from(node.start_byte()..node.end_byte());
    let children = children(node);
    if children.is_empty() {
        return Err(span_error(
            node,
            source,
            source_name,
            "incomplete unary expression",
        ));
    }
    let op = lower_unary_op(
        &child_by_field(node, "operator", source, source_name)?,
        source,
        source_name,
    )?;
    let operand = lower_expression(&children[0], source, source_name)?;
    match (&op, &operand) {
        (UnaryOp::Neg, Expr::Int(v, _)) => Ok(Expr::Int(-v, span)),
        (UnaryOp::Neg, Expr::Float(v, _)) => Ok(Expr::Float(-v, span)),
        (UnaryOp::Not, Expr::Bool(b, _)) => Ok(Expr::Bool(!b, span)),
        (UnaryOp::Ref, _) => Ok(Expr::Ref {
            span,
            operand: Box::new(operand),
        }),
        _ => Ok(Expr::Unary {
            span,
            op,
            operand: Box::new(operand),
        }),
    }
}

fn lower_unary_op(node: &Node, source: &str, source_name: &str) -> Result<UnaryOp, LowerError> {
    Ok(match node_text(node, source).as_str() {
        "-" => UnaryOp::Neg,
        "!" | "not" => UnaryOp::Not,
        "&" => UnaryOp::Ref,
        other => {
            return Err(span_error(
                node,
                source,
                source_name,
                &format!("unknown unary operator `{other}`"),
            ));
        }
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

fn child_by_field_opt<'a>(node: &'a Node<'a>, field: &str) -> Option<Node<'a>> {
    node.child_by_field_name(field)
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

