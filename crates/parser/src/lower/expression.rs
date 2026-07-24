use miette::SourceSpan;
use tree_sitter::Node;

use crate::{
    ast::{
        expression::{Expression, MatchArm},
        operator::{BinaryOp, UnaryOp},
    },
    lower::{
        Lowerer,
        error::LowerError,
        helpers::{child_by_field_opt, children, node_text},
    },
};

impl<'a> Lowerer<'a> {
    pub(super) fn lower_expression(&self, node: &Node) -> Result<Expression, LowerError> {
        let span = || SourceSpan::from(node.start_byte()..node.end_byte());
        match node.kind() {
            "identifier" => Ok(Expression::Ident(node_text(node, self.source), span())),
            "string_literal" => self.lower_string(node),
            "raw_string_literal" => self.lower_raw_string(node),
            "char_literal" => self.lower_char(node),
            "integer_literal" => self.lower_int(node),
            "float_literal" => self.lower_float(node),
            "bool_literal" => Ok(Expression::Bool(
                node_text(node, self.source) == "true",
                span(),
            )),
            "unit_literal" => Ok(Expression::Unit(span())),
            "call_expression" => self.lower_call(node),
            "binary_expression" => self.lower_binary(node),
            "unary_expression" => self.lower_unary(node),
            "parenthesized_expression" => {
                for i in 0..node.named_child_count() {
                    if let Some(child) = node.named_child(i as u32) {
                        return self
                            .lower_expression(&child)
                            .map(Box::new)
                            .map(|e| Expression::Paren(e, span()));
                    }
                }
                Err(self.span_error(node, "empty parenthesized expression"))
            }
            "block" => self.lower_block(node).map(|s| Expression::Block(s, span())),
            "array_expression" => {
                let children = children(node);
                let elements: Result<Vec<Expression>, _> =
                    children.iter().map(|c| self.lower_expression(c)).collect();
                elements.map(|e| Expression::Array(e, span()))
            }
            "tuple_expression" => {
                let children = children(node);
                let elements: Result<Vec<Expression>, _> =
                    children.iter().map(|c| self.lower_expression(c)).collect();
                elements.map(|e| Expression::Tuple(e, span()))
            }
            "field_access_expression" => {
                let children = children(node);
                if children.len() < 2 {
                    return Err(self.span_error(node, "incomplete field access"));
                }
                let object = self.lower_expression(&children[0])?;
                let name = node_text(&self.child_by_field(node, "field")?, self.source);
                Ok(Expression::Field {
                    span: span(),
                    object: Box::new(object),
                    name,
                })
            }
            "match_expression" => self.lower_match(node),
            "enum_variant_expression" => {
                let type_name = node_text(&self.child_by_field(node, "type")?, self.source);
                let variant_name = node_text(&self.child_by_field(node, "variant")?, self.source);
                let args = if let Some(arg_node) = child_by_field_opt(node, "arguments") {
                    let arg_children: Vec<Expression> = children(&arg_node)
                        .iter()
                        .map(|c| self.lower_expression(c))
                        .collect::<Result<Vec<_>, _>>()?;
                    arg_children
                } else {
                    Vec::new()
                };
                Ok(Expression::EnumVariant {
                    span: span(),
                    type_name,
                    variant_name,
                    args,
                })
            }
            "index_expression" => {
                let children = children(node);
                if children.len() < 2 {
                    return Err(self.span_error(node, "incomplete index expression"));
                }
                let array = self.lower_expression(&children[0])?;
                let index = self.lower_expression(&children[1])?;
                Ok(Expression::Index {
                    span: span(),
                    array: Box::new(array),
                    index: Box::new(index),
                })
            }
            "if_expression" => self.lower_if(node),
            kind => Err(self.invalid_kind(node, kind, "expression")),
        }
    }

    pub(super) fn lower_any_expression(&self, node: &Node) -> Result<Expression, LowerError> {
        for i in (0..node.named_child_count()).rev() {
            if let Some(child) = node.named_child(i as u32) {
                match node.field_name_for_named_child(i as u32) {
                    Some(
                        "name" | "parameters" | "type" | "mut" | "arguments" | "return_type"
                        | "operator",
                    ) => continue,
                    _ => return self.lower_expression(&child),
                }
            }
        }
        Err(self.span_error(node, "expected expression"))
    }

    pub(super) fn lower_if(&self, node: &Node) -> Result<Expression, LowerError> {
        let span = SourceSpan::from(node.start_byte()..node.end_byte());
        let named = children(node);

        let condition = match named.first() {
            Some(n) => self.lower_expression(n)?,
            None => {
                return Err(self.span_error(node, "incomplete if: missing condition"));
            }
        };
        let then_block = match named.get(1) {
            Some(n) => self.lower_block(n)?,
            None => {
                return Err(self.span_error(node, "incomplete if: missing body"));
            }
        };

        let mut else_if = Vec::new();
        let mut else_block = None;

        let mut index = 2;
        while index + 1 < named.len() && named[index + 1].kind() == "block" {
            let condition = self.lower_expression(&named[index])?;
            let block = self.lower_block(&named[index + 1])?;
            else_if.push((condition, block));
            index += 2;
        }
        if let Some(else_node) = named.get(index)
            && else_node.kind() == "block"
        {
            else_block = Some(self.lower_block(else_node)?);
        }

        Ok(Expression::If {
            span,
            condition: Box::new(condition),
            then_block,
            else_if,
            else_block,
        })
    }

    pub(super) fn lower_string(&self, node: &Node) -> Result<Expression, LowerError> {
        let span = SourceSpan::from(node.start_byte()..node.end_byte());
        let raw = node_text(node, self.source);
        let content = if raw.starts_with('f') {
            let c = raw.trim_start_matches('f');
            &c[1..c.len() - 1]
        } else {
            &raw[1..raw.len() - 1]
        };
        Ok(Expression::String(content.to_string(), span))
    }

    pub(super) fn lower_raw_string(&self, node: &Node) -> Result<Expression, LowerError> {
        let span = SourceSpan::from(node.start_byte()..node.end_byte());
        let raw = node_text(node, self.source);
        let content = &raw[2..raw.len() - 1];
        Ok(Expression::String(content.to_string(), span))
    }

    pub(super) fn lower_char(&self, node: &Node) -> Result<Expression, LowerError> {
        let span = SourceSpan::from(node.start_byte()..node.end_byte());
        let raw = node_text(node, self.source);
        let c = raw.chars().nth(1).unwrap_or('\0');
        Ok(Expression::Char(c, span))
    }

    pub(super) fn lower_int(&self, node: &Node) -> Result<Expression, LowerError> {
        let span = SourceSpan::from(node.start_byte()..node.end_byte());
        let raw = node_text(node, self.source);
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
            Ok(v) => Ok(Expression::Int(v, span)),
            Err(_) => Err(self.span_error(node, &format!("invalid integer literal `{raw}`"))),
        }
    }

    pub(super) fn lower_float(&self, node: &Node) -> Result<Expression, LowerError> {
        let span = SourceSpan::from(node.start_byte()..node.end_byte());
        let raw = node_text(node, self.source);
        match raw.parse::<f64>() {
            Ok(v) => Ok(Expression::Float(v, span)),
            Err(_) => Err(self.span_error(node, &format!("invalid float literal `{raw}`"))),
        }
    }

    pub(super) fn lower_call(&self, node: &Node) -> Result<Expression, LowerError> {
        let span = SourceSpan::from(node.start_byte()..node.end_byte());
        let func_node = self.child_by_field(node, "function")?;
        let function = self.lower_expression(&func_node)?;

        let args_node = self.child_by_field(node, "arguments")?;
        let mut args = Vec::new();
        for i in 0..args_node.named_child_count() {
            if let Some(child) = args_node.named_child(i as u32) {
                args.push(self.lower_expression(&child)?);
            }
        }

        Ok(Expression::Call {
            span,
            function: Box::new(function),
            args,
        })
    }

    pub(super) fn lower_match(&self, node: &Node) -> Result<Expression, LowerError> {
        let span = SourceSpan::from(node.start_byte()..node.end_byte());
        let children = children(node);
        if children.is_empty() {
            return Err(self.span_error(node, "incomplete match expression"));
        }
        let value = self.lower_expression(&children[0])?;
        let mut arms = Vec::new();
        for child in children.iter().skip(1) {
            if child.kind() == "match_arm" {
                arms.push(self.lower_match_arm(child)?);
            }
        }
        Ok(Expression::Match {
            span,
            value: Box::new(value),
            arms,
        })
    }

    pub(super) fn lower_match_arm(&self, node: &Node) -> Result<MatchArm, LowerError> {
        let span = SourceSpan::from(node.start_byte()..node.end_byte());
        let children = children(node);
        if children.len() < 2 {
            return Err(self.span_error(node, "incomplete match arm"));
        }
        let pattern = self.lower_pattern(&children[0])?;
        let body = Box::new(self.lower_expression(&children[1])?);
        Ok(MatchArm {
            span,
            pattern,
            body,
        })
    }

    pub(super) fn lower_unary(&self, node: &Node) -> Result<Expression, LowerError> {
        let span = SourceSpan::from(node.start_byte()..node.end_byte());
        let children = children(node);
        if children.is_empty() {
            return Err(self.span_error(node, "incomplete unary expression"));
        }
        let op = self.lower_unary_op(&self.child_by_field(node, "operator")?)?;
        let operand = self.lower_expression(&children[0])?;
        match (&op, &operand) {
            (UnaryOp::Neg, Expression::Int(v, _)) => Ok(Expression::Int(-v, span)),
            (UnaryOp::Neg, Expression::Float(v, _)) => Ok(Expression::Float(-v, span)),
            (UnaryOp::Not, Expression::Bool(b, _)) => Ok(Expression::Bool(!b, span)),
            (UnaryOp::Ref, _) => Ok(Expression::Ref {
                span,
                operand: Box::new(operand),
            }),
            _ => Ok(Expression::Unary {
                span,
                op,
                operand: Box::new(operand),
            }),
        }
    }

    pub(super) fn lower_unary_op(&self, node: &Node) -> Result<UnaryOp, LowerError> {
        Ok(match node_text(node, self.source).as_str() {
            "-" => UnaryOp::Neg,
            "!" | "not" => UnaryOp::Not,
            "&" => UnaryOp::Ref,
            other => {
                return Err(self.span_error(node, &format!("unknown unary operator `{other}`")));
            }
        })
    }

    pub(super) fn lower_binary(&self, node: &Node) -> Result<Expression, LowerError> {
        let span = SourceSpan::from(node.start_byte()..node.end_byte());
        let children = children(node);
        if children.len() < 2 {
            return Err(self.span_error(node, "incomplete binary expression"));
        }
        let left = self.lower_expression(&children[0])?;
        let op = self.lower_binary_op(&self.child_by_field(node, "operator")?)?;
        let right = self.lower_expression(&children[1])?;
        Ok(Expression::Binary {
            span,
            left: Box::new(left),
            op,
            right: Box::new(right),
        })
    }

    pub(super) fn lower_binary_op(&self, node: &Node) -> Result<BinaryOp, LowerError> {
        Ok(match node_text(node, self.source).as_str() {
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
                return Err(self.span_error(node, &format!("unknown operator `{other}`")));
            }
        })
    }
}
