use miette::{NamedSource, SourceSpan};
use tree_sitter::Node;

use crate::lower::{Lowerer, error::LowerError};

impl<'a> Lowerer<'a> {
    pub(super) fn invalid_kind(&self, node: &Node, kind: &str, context: &str) -> LowerError {
        self.span_error(node, &format!("unsupported {context}: `{kind}`"))
    }

    pub(super) fn child_by_field<'b>(
        &self,
        node: &Node<'b>,
        field: &str,
    ) -> Result<Node<'b>, LowerError> {
        node.child_by_field_name(field)
            .ok_or_else(|| self.span_error(node, &format!("missing field `{field}`")))
    }

    pub(super) fn span_error(&self, node: &Node, message: &str) -> LowerError {
        LowerError {
            message: message.to_string(),
            source: NamedSource::new(self.source_name, self.source.to_string()),
            span: SourceSpan::from(node.start_byte()..node.end_byte()),
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
