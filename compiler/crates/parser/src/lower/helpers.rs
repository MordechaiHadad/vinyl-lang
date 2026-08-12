use miette::{NamedSource, SourceSpan};
use tree_sitter::Node;

use crate::error::ParserDiagnosticKind;
use crate::{ParserDiagnostic, lower::Lowerer};

impl<'a> Lowerer<'a> {
    pub(super) fn invalid_kind(&self, node: &Node, kind: &str, context: &str) -> ParserDiagnostic {
        self.span_error(node, &format!("unsupported {context}: `{kind}`"))
    }

    pub(super) fn child_by_field<'b>(
        &self,
        node: &Node<'b>,
        field: &str,
    ) -> Result<Node<'b>, ParserDiagnostic> {
        node.child_by_field_name(field)
            .ok_or_else(|| self.span_error(node, &format!("missing field `{field}`")))
    }

    pub(super) fn span_error(&self, node: &Node, message: &str) -> ParserDiagnostic {
        self.error_at(
            node,
            ParserDiagnosticKind::Lowering {
                message: message.to_string(),
            },
        )
    }

    pub(super) fn error_at(&self, node: &Node, kind: ParserDiagnosticKind) -> ParserDiagnostic {
        self.error_at_span(SourceSpan::from(node.start_byte()..node.end_byte()), kind)
    }

    pub(super) fn error_at_span(
        &self,
        span: SourceSpan,
        kind: ParserDiagnosticKind,
    ) -> ParserDiagnostic {
        ParserDiagnostic {
            kind,
            source_code: NamedSource::new(self.source_name, self.source.to_string()),
            span,
        }
    }
}

pub(super) fn child_by_field_opt<'a>(node: &'a Node<'a>, field: &str) -> Option<Node<'a>> {
    node.child_by_field_name(field)
}

pub(super) fn children<'a>(node: &'a Node<'a>) -> Vec<Node<'a>> {
    let mut v = Vec::new();
    for i in 0..node.named_child_count() {
        if let Some(child) = node.named_child(i as u32) {
            v.push(child);
        }
    }
    v
}

pub(super) fn node_text(node: &Node, source: &str) -> String {
    source[node.start_byte()..node.end_byte()].to_string()
}

pub(super) fn parse_integer_literal(raw: &str) -> Option<u128> {
    if let Some(hex) = raw.strip_prefix("0x") {
        u128::from_str_radix(hex, 16).ok()
    } else if let Some(oct) = raw.strip_prefix("0o") {
        u128::from_str_radix(oct, 8).ok()
    } else if let Some(bin) = raw.strip_prefix("0b") {
        u128::from_str_radix(bin, 2).ok()
    } else {
        raw.parse().ok()
    }
}
