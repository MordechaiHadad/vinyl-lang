use miette::SourceSpan;
use tree_sitter::Node;

use crate::{
    ast::item::{
        Attribute, EnumDef, EnumVariant, EnumVariantData, FunctionDef, FunctionParam, Item,
        StructDef, StructField, TupleDef,
    },
    lower::{Lowerer, error::LowerError, helpers::node_text},
};

impl<'a> Lowerer<'a> {
    pub(super) fn lower_item(&self, node: &Node, kind: &str) -> Result<Item, LowerError> {
        match kind {
            "function_definition" => self.lower_function(node, Vec::new()).map(Item::Function),
            "struct_definition" => self
                .lower_struct_definition(node, Vec::new())
                .map(Item::Struct),
            "tuple_definition" => self
                .lower_tuple_definition(node, Vec::new())
                .map(Item::TupleStruct),
            "enum_definition" => self.lower_enum_definition(node, Vec::new()).map(Item::Enum),
            kind => Err(self.invalid_kind(node, kind, "item")),
        }
    }

    pub(super) fn lower_struct_definition(
        &self,
        node: &Node,
        attrs: Vec<Attribute>,
    ) -> Result<StructDef, LowerError> {
        let span = SourceSpan::from(node.start_byte()..node.end_byte());
        let name = node_text(&self.child_by_field(node, "name")?, self.source);
        let mut fields = Vec::new();
        for i in 0..node.named_child_count() {
            if let Some(child) = node.named_child(i as u32)
                && child.kind() == "field_definition"
            {
                fields.push(self.lower_field_definition(&child)?);
            }
        }
        Ok(StructDef {
            span,
            attrs,
            name,
            fields,
        })
    }

    pub(super) fn lower_field_definition(&self, node: &Node) -> Result<StructField, LowerError> {
        let span = SourceSpan::from(node.start_byte()..node.end_byte());
        let name = node_text(&self.child_by_field(node, "name")?, self.source);
        let type_ = self.lower_type_annotation_child(node)?;
        Ok(StructField { span, name, type_ })
    }

    pub(super) fn lower_tuple_definition(
        &self,
        node: &Node,
        attrs: Vec<Attribute>,
    ) -> Result<TupleDef, LowerError> {
        let span = SourceSpan::from(node.start_byte()..node.end_byte());
        let name = node_text(&self.child_by_field(node, "name")?, self.source);
        let mut types = Vec::new();
        for i in 0..node.named_child_count() {
            if let Some(child) = node.named_child(i as u32)
                && child.kind() != "identifier"
            {
                types.push(self.lower_type(&child)?);
            }
        }
        Ok(TupleDef {
            span,
            attrs,
            name,
            types,
        })
    }

    pub(super) fn lower_enum_definition(
        &self,
        node: &Node,
        attrs: Vec<Attribute>,
    ) -> Result<EnumDef, LowerError> {
        let span = SourceSpan::from(node.start_byte()..node.end_byte());
        let name = node_text(&self.child_by_field(node, "name")?, self.source);
        let mut variants = Vec::new();
        for i in 0..node.named_child_count() {
            if let Some(child) = node.named_child(i as u32)
                && child.kind() == "enum_variant"
            {
                variants.push(self.lower_enum_variant(&child)?);
            }
        }
        Ok(EnumDef {
            span,
            attrs,
            name,
            variants,
        })
    }

    pub(super) fn lower_enum_variant(&self, node: &Node) -> Result<EnumVariant, LowerError> {
        let span = SourceSpan::from(node.start_byte()..node.end_byte());
        let name = node_text(&self.child_by_field(node, "name")?, self.source);
        let child_count = node.named_child_count();
        let data = if child_count > 1 {
            let first_body = node.named_child(1).unwrap();
            match first_body.kind() {
                "field_definition" => {
                    let mut fields = Vec::new();
                    for i in 1..child_count {
                        if let Some(child) = node.named_child(i as u32) {
                            fields.push(self.lower_field_definition(&child)?);
                        }
                    }
                    Some(EnumVariantData::Struct(fields))
                }
                _ => {
                    let mut types = Vec::new();
                    for i in 1..child_count {
                        if let Some(child) = node.named_child(i as u32) {
                            types.push(self.lower_type(&child)?);
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

    pub(super) fn lower_function(
        &self,
        node: &Node,
        attrs: Vec<Attribute>,
    ) -> Result<FunctionDef, LowerError> {
        let span = SourceSpan::from(node.start_byte()..node.end_byte());
        let name = node_text(&self.child_by_field(node, "name")?, self.source);
        let params_node = self.child_by_field(node, "parameters")?;
        let params = self.lower_params(&params_node)?;

        let return_type = match node.child_by_field_name("return_type") {
            Some(ann) => Some(self.lower_type(&ann)?),
            None => None,
        };

        let body_node = self.child_by_field(node, "body")?;
        let body = self.lower_block(&body_node)?;

        Ok(FunctionDef {
            span,
            attrs,
            name,
            params,
            return_type,
            body,
        })
    }

    pub(super) fn lower_params(&self, node: &Node) -> Result<Vec<FunctionParam>, LowerError> {
        let mut params = Vec::new();
        for i in 0..node.named_child_count() {
            if let Some(child) = node.named_child(i as u32)
                && child.kind() == "parameter"
            {
                params.push(self.lower_param(&child)?);
            }
        }
        Ok(params)
    }

    pub(super) fn lower_param(&self, node: &Node) -> Result<FunctionParam, LowerError> {
        let span = SourceSpan::from(node.start_byte()..node.end_byte());
        let mutable = node.child_by_field_name("mut").is_some();
        let name = node_text(&self.child_by_field(node, "name")?, self.source);
        let type_ann = self.child_by_field(node, "type")?;
        let type_ = self.lower_type(&type_ann)?;
        Ok(FunctionParam {
            span,
            name,
            mutable,
            type_,
        })
    }

    pub(super) fn lower_attribute(&self, node: &Node) -> Result<Attribute, LowerError> {
        let span = SourceSpan::from(node.start_byte()..node.end_byte());
        let name = node_text(
            &self.child_by_field(node, "name")?,
            self.source,
        );
        let mut args = Vec::new();
        for i in 0..node.named_child_count() {
            if let Some(child) = node.named_child(i as u32)
                && node.field_name_for_named_child(i as u32) != Some("name")
            {
                args.push(self.lower_expression(&child)?);
            }
        }
        Ok(Attribute { span, name, args })
    }
}
