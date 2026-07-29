use miette::SourceSpan;
use tree_sitter::Node;

use crate::{
    ParserDiagnostic,
    ast::pattern::{LiteralPattern, Pattern},
    lower::{
        Lowerer,
        helpers::{children, node_text},
    },
};

impl<'a> Lowerer<'a> {
    pub(super) fn lower_pattern(&self, node: &Node) -> Result<Pattern, ParserDiagnostic> {
        let span = || SourceSpan::from(node.start_byte()..node.end_byte());
        match node.kind() {
            "wildcard_pattern" => Ok(Pattern::Wildcard(span())),
            "identifier_pattern" => {
                let name = node_text(&node.named_child(0).unwrap_or(*node), self.source);
                Ok(Pattern::Ident(name, span()))
            }
            "literal_pattern" => self.lower_literal_pattern(node),
            "struct_pattern" => self.lower_struct_pattern(node),
            "tuple_pattern" => {
                let patterns = children(node)
                    .iter()
                    .map(|c| self.lower_pattern(c))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Pattern::Tuple(patterns, span()))
            }
            "enum_variant_pattern" => {
                let children = children(node);
                if children.is_empty() {
                    return Err(self.span_error(node, "incomplete enum variant pattern"));
                }
                let name = node_text(&children[0], self.source);
                let patterns = children[1..]
                    .iter()
                    .map(|c| self.lower_pattern(c))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Pattern::EnumVariant {
                    span: span(),
                    name,
                    patterns,
                })
            }
            kind => match node.named_child(0) {
                Some(child) => self.lower_pattern(&child),
                None => Err(self.invalid_kind(node, kind, "pattern")),
            },
        }
    }

    pub(super) fn lower_literal_pattern(&self, node: &Node) -> Result<Pattern, ParserDiagnostic> {
        for i in 0..node.named_child_count() {
            if let Some(child) = node.named_child(i as u32) {
                return match child.kind() {
                    "integer_literal" => {
                        let raw = node_text(&child, self.source);
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
                            Ok(v) => Ok(Pattern::Literal(
                                LiteralPattern::Int(v),
                                SourceSpan::from(child.start_byte()..child.end_byte()),
                            )),
                            Err(_) => Err(self
                                .span_error(&child, &format!("invalid integer literal `{raw}`"))),
                        }
                    }
                    "bool_literal" => {
                        let v = node_text(&child, self.source) == "true";
                        Ok(Pattern::Literal(
                            LiteralPattern::Bool(v),
                            SourceSpan::from(child.start_byte()..child.end_byte()),
                        ))
                    }
                    "char_literal" => {
                        let raw = node_text(&child, self.source);
                        let c = raw.chars().nth(1).unwrap_or('\0');
                        Ok(Pattern::Literal(
                            LiteralPattern::Char(c),
                            SourceSpan::from(child.start_byte()..child.end_byte()),
                        ))
                    }
                    "string_literal" => {
                        let raw = node_text(&child, self.source);
                        let content = &raw[1..raw.len() - 1];
                        Ok(Pattern::Literal(
                            LiteralPattern::String(content.to_string()),
                            SourceSpan::from(child.start_byte()..child.end_byte()),
                        ))
                    }
                    _ => Err(self.span_error(
                        &child,
                        &format!("unsupported literal pattern: `{}`", child.kind()),
                    )),
                };
            }
        }
        Err(self.span_error(node, "empty literal pattern"))
    }

    pub(super) fn lower_struct_pattern(&self, node: &Node) -> Result<Pattern, ParserDiagnostic> {
        let span = SourceSpan::from(node.start_byte()..node.end_byte());
        let children = children(node);
        if children.is_empty() {
            return Err(self.span_error(node, "incomplete struct pattern"));
        }
        let name = node_text(&children[0], self.source);
        let mut fields = Vec::new();
        for field_node in children.iter().skip(1) {
            let field_name = node_text(&self.child_by_field(field_node, "name")?, self.source);
            let pattern = match field_node.named_child(1) {
                Some(sub_pattern) => self.lower_pattern(&sub_pattern)?,
                None => Pattern::Ident(
                    field_name.clone(),
                    SourceSpan::from(field_node.start_byte()..field_node.end_byte()),
                ),
            };
            fields.push((field_name, pattern));
        }
        Ok(Pattern::Struct { span, name, fields })
    }
}
