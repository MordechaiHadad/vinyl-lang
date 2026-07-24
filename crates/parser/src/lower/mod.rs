pub mod error;
pub mod expression;
pub mod helpers;
pub mod item;
pub mod pattern;
pub mod statement;
pub mod types;

use crate::ast::item::{Attribute, Item};
use tree_sitter::{Node, Tree};

use super::lower::error::LowerError;

pub struct Lowerer<'a> {
    pub source: &'a str,
    pub source_name: &'a str,
}

impl<'a> Lowerer<'a> {
    pub fn new(source: &'a str, source_name: &'a str) -> Self {
        Self {
            source,
            source_name,
        }
    }

    pub(super) fn lower_source_file(&self, node: &Node) -> Result<Vec<Item>, Vec<LowerError>> {
        let mut items = Vec::new();
        let mut errors = Vec::new();
        let mut pending_attrs: Vec<Attribute> = Vec::new();
        for i in 0..node.named_child_count() {
            if let Some(child) = node.named_child(i as u32) {
                match child.kind() {
                    "attribute" => match self.lower_attribute(&child) {
                        Ok(attr) => pending_attrs.push(attr),
                        Err(e) => errors.push(e),
                    },
                    "comment" => {}
                    kind => {
                        let mut item = self.lower_item(&child, kind);
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
}

pub fn lower(tree: &Tree, source: &str, source_name: &str) -> Result<Vec<Item>, Vec<LowerError>> {
    let root = tree.root_node();
    let lowerer = Lowerer::new(source, source_name);
    lowerer.lower_source_file(&root)
}
