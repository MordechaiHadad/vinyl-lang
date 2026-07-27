use tree_sitter::Node;

use crate::{
    ParserDiagnostic, ast::types::{Primitive, Type}, lower::{
        Lowerer,
        helpers::{children, node_text},
    }
};

impl<'a> Lowerer<'a> {
    pub(super) fn lower_type_annotation_child(&self, node: &Node) -> Result<Type, ParserDiagnostic> {
        for i in 0..node.named_child_count() {
            if let Some(child) = node.named_child(i as u32)
                && child.kind() == "type_annotation"
            {
                return self.lower_type(&child);
            }
        }
        for i in 0..node.named_child_count() {
            if let Some(child) = node.named_child(i as u32) {
                match child.kind() {
                    "simple_type" | "generic_type" | "array_type" | "reference_type" => {
                        return self.lower_type(&child);
                    }
                    _ => {}
                }
            }
        }
        Err(self.span_error(node, "expected type"))
    }

    pub(super) fn lower_type(&self, node: &Node) -> Result<Type, ParserDiagnostic> {
        match node.kind() {
            "simple_type" => return self.lower_simple_type(node),
            "generic_type" => return self.lower_generic_type(node),
            "array_type" => return self.lower_array_type(node),
            "reference_type" => return self.lower_reference_type(node),
            "tuple_type" => return self.lower_tuple_type(node),
            _ => {}
        }
        for i in 0..node.named_child_count() {
            if let Some(child) = node.named_child(i as u32) {
                return match child.kind() {
                    "simple_type" => self.lower_simple_type(&child),
                    "generic_type" => self.lower_generic_type(&child),
                    "array_type" => self.lower_array_type(&child),
                    "reference_type" => self.lower_reference_type(&child),
                    "tuple_type" => self.lower_tuple_type(&child),
                    kind => Err(self.invalid_kind(node, kind, "type")),
                };
            }
        }
        Err(self.invalid_kind(node, node.kind(), "type"))
    }

    pub(super) fn lower_reference_type(&self, node: &Node) -> Result<Type, ParserDiagnostic> {
        for i in 0..node.named_child_count() {
            if let Some(child) = node.named_child(i as u32) {
                let inner = self.lower_type(&child)?;
                return Ok(Type::Ref(Box::new(inner)));
            }
        }
        Err(self.span_error(node, "empty reference type"))
    }

    pub(super) fn lower_tuple_type(&self, node: &Node) -> Result<Type, ParserDiagnostic> {
        let mut elements = Vec::new();
        for i in 0..node.named_child_count() {
            if let Some(child) = node.named_child(i as u32) {
                elements.push(self.lower_type(&child)?);
            }
        }
        Ok(Type::Tuple(elements))
    }

    pub(super) fn lower_simple_type(&self, node: &Node) -> Result<Type, ParserDiagnostic> {
        let child = node
            .named_child(0)
            .ok_or_else(|| self.span_error(node, "empty simple type"))?;
        match child.kind() {
            "primitive_type" => {
                let name = node_text(&child, self.source);
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
                        return Err(
                            self.span_error(node, &format!("unknown primitive type `{name}`"))
                        );
                    }
                })
            }
            "type_identifier" => Ok(Type::Named(node_text(&child, self.source))),
            kind => Err(self.invalid_kind(&child, kind, "type")),
        }
    }

    pub(super) fn lower_generic_type(&self, node: &Node) -> Result<Type, ParserDiagnostic> {
        let name = node_text(
            &node
                .named_child(0)
                .ok_or_else(|| self.span_error(node, "expected identifier in generic type"))?,
            self.source,
        );
        let mut args = Vec::new();
        for i in 1..node.named_child_count() {
            if let Some(child) = node.named_child(i as u32) {
                args.push(self.lower_type(&child)?);
            }
        }
        Ok(Type::Generic { name, args })
    }

    pub(super) fn lower_array_type(&self, node: &Node) -> Result<Type, ParserDiagnostic> {
        let children = children(node);
        if children.len() < 2 {
            return Err(self.span_error(node, "incomplete array type"));
        }
        let element = self.lower_type(&children[0])?;
        let size_text = node_text(&children[1], self.source);
        let size: usize = size_text.parse().map_err(|_| {
            self.span_error(&children[1], &format!("invalid array size `{size_text}`"))
        })?;
        Ok(Type::Array {
            element: Box::new(element),
            size,
        })
    }
}
