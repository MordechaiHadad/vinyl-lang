use miette::{NamedSource, SourceSpan};
use tree_sitter::Node;

use crate::{
    ParserDiagnostic,
    ast::item::{
        Attribute, EnumDef, EnumVariant, EnumVariantData, FunctionDef, FunctionParam, ImportDef,
        Item, StructDef, StructField, TupleDef, TypeAliasDef,
    },
    error::ParserDiagnosticKind,
    lower::{Lowerer, helpers::node_text},
};

impl<'a> Lowerer<'a> {
    pub(super) fn lower_item(
        &self,
        node: &Node,
        kind: &str,
        public: bool,
    ) -> Result<Item, ParserDiagnostic> {
        match kind {
            "function_definition" => self
                .lower_function(node, Vec::new(), public)
                .map(Item::Function),
            "struct_definition" => self
                .lower_struct_definition(node, Vec::new(), public)
                .map(Item::Struct),
            "tuple_definition" => self
                .lower_tuple_definition(node, Vec::new(), public)
                .map(Item::TupleStruct),
            "enum_definition" => self
                .lower_enum_definition(node, Vec::new(), public)
                .map(Item::Enum),
            "type_alias_definition" => self
                .lower_type_alias(node, Vec::new(), public)
                .map(Item::TypeAlias),
            "import_statement" => self.lower_import(node).map(Item::Import),
            kind => Err(self.invalid_kind(node, kind, "item")),
        }
    }

    pub(super) fn lower_struct_definition(
        &self,
        node: &Node,
        attrs: Vec<Attribute>,
        public: bool,
    ) -> Result<StructDef, ParserDiagnostic> {
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
            public,
            attrs,
            name,
            fields,
        })
    }

    pub(super) fn lower_field_definition(
        &self,
        node: &Node,
    ) -> Result<StructField, ParserDiagnostic> {
        let span = SourceSpan::from(node.start_byte()..node.end_byte());
        let public = has_public(node);
        let name = node_text(&self.child_by_field(node, "name")?, self.source);
        let type_ = self.lower_type_annotation_child(node)?;
        Ok(StructField {
            span,
            public,
            name,
            type_,
        })
    }

    pub(super) fn lower_tuple_definition(
        &self,
        node: &Node,
        attrs: Vec<Attribute>,
        public: bool,
    ) -> Result<TupleDef, ParserDiagnostic> {
        let span = SourceSpan::from(node.start_byte()..node.end_byte());
        let name = node_text(&self.child_by_field(node, "name")?, self.source);
        let mut types = Vec::new();
        for i in 0..node.named_child_count() {
            if let Some(child) = node.named_child(i as u32)
                && child.kind() != "value_identifier"
                && child.kind() != "type_identifier"
            {
                types.push(self.lower_type(&child)?);
            }
        }
        Ok(TupleDef {
            span,
            public,
            attrs,
            name,
            types,
        })
    }

    pub(super) fn lower_enum_definition(
        &self,
        node: &Node,
        attrs: Vec<Attribute>,
        public: bool,
    ) -> Result<EnumDef, ParserDiagnostic> {
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
            public,
            attrs,
            name,
            variants,
        })
    }

    pub(super) fn lower_enum_variant(&self, node: &Node) -> Result<EnumVariant, ParserDiagnostic> {
        let span = SourceSpan::from(node.start_byte()..node.end_byte());
        if has_public(node) {
            return Err(ParserDiagnostic {
                kind: ParserDiagnosticKind::EnumVariantPublicModifier,
                source_code: NamedSource::new(self.source_name, self.source.to_string()),
                span,
            });
        }
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
        public: bool,
    ) -> Result<FunctionDef, ParserDiagnostic> {
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
            public,
            attrs,
            name,
            params,
            return_type,
            body,
        })
    }

    pub(super) fn lower_type_alias(
        &self,
        node: &Node,
        attrs: Vec<Attribute>,
        public: bool,
    ) -> Result<TypeAliasDef, ParserDiagnostic> {
        let span = SourceSpan::from(node.start_byte()..node.end_byte());
        let name = node_text(&self.child_by_field(node, "name")?, self.source);
        let type_node = self.child_by_field(node, "type")?;
        let type_ = self.lower_type(&type_node)?;
        Ok(TypeAliasDef {
            span,
            public,
            attrs,
            name,
            type_,
        })
    }

    pub(super) fn lower_import(&self, node: &Node) -> Result<ImportDef, ParserDiagnostic> {
        let span = SourceSpan::from(node.start_byte()..node.end_byte());
        let import_path_node = self.child_by_field(node, "path")?;

        let mut prefix = Vec::new();
        for i in 0..import_path_node.named_child_count() {
            if let Some(child) = import_path_node.named_child(i as u32)
                && child.kind() == "import_prefix"
            {
                prefix.push(node_text(&child, self.source));
            }
        }

        let path_node = self.child_by_field(&import_path_node, "path")?;
        let mut path = Vec::new();
        let mut wildcard = false;
        for i in 0..path_node.named_child_count() {
            if let Some(child) = path_node.named_child(i as u32) {
                if child.kind() == "import_wildcard" {
                    wildcard = true;
                } else {
                    path.push(node_text(&child, self.source));
                }
            }
        }
        let symbols = self
            .child_by_field(&import_path_node, "symbols")
            .map(|symbols_node| {
                (0..symbols_node.named_child_count())
                    .filter_map(|index| symbols_node.named_child(index as u32))
                    .map(|child| node_text(&child, self.source))
                    .collect()
            })
            .unwrap_or_default();
        Ok(ImportDef {
            span,
            prefix,
            path,
            symbols,
            wildcard,
        })
    }

    pub(super) fn lower_params(&self, node: &Node) -> Result<Vec<FunctionParam>, ParserDiagnostic> {
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

    pub(super) fn lower_param(&self, node: &Node) -> Result<FunctionParam, ParserDiagnostic> {
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

    pub(super) fn lower_attribute(&self, node: &Node) -> Result<Attribute, ParserDiagnostic> {
        let span = SourceSpan::from(node.start_byte()..node.end_byte());
        let name = node_text(&self.child_by_field(node, "name")?, self.source);
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

fn has_public(node: &Node) -> bool {
    (0..node.child_count()).any(|i| node.child(i as u32).is_some_and(|c| c.kind() == "public"))
}
