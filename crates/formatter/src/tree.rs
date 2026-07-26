use tree_sitter::Node;

use crate::error::FormatError;

pub fn format_source(source: &str) -> Result<String, FormatError> {
    let tree = vinyl_parser::parse(source).map_err(|errors| {
        FormatError::Parse(Box::new(errors.into_iter().next().unwrap()))
    })?;
    let root = tree.root_node();
    let mut f = Formatter { source, output: String::new(), indent: 0 };
    f.format_root(root);
    let trimmed = f.output.trim_end().to_string();
    Ok(trimmed)
}

struct Formatter<'a> {
    source: &'a str,
    output: String,
    indent: usize,
}

impl Formatter<'_> {
    fn text(&self, node: Node) -> &str {
        &self.source[node.start_byte()..node.end_byte()]
    }

    fn emit(&mut self, text: &str) {
        if self.output.is_empty() || self.output.ends_with('\n') {
            for _ in 0..self.indent {
                self.output.push_str("    ");
            }
        }
        self.output.push_str(text);
    }

    fn emit_node(&mut self, node: Node) {
        let start = node.start_byte();
        let end = node.end_byte();
        if self.output.is_empty() || self.output.ends_with('\n') {
            for _ in 0..self.indent {
                self.output.push_str("    ");
            }
        }
        self.output.push_str(&self.source[start..end]);
    }

    fn newline(&mut self) {
        self.output.push('\n');
    }

    fn format_root(&mut self, node: Node) {
        let mut cursor = node.walk();
        let children: Vec<Node> = node.children(&mut cursor).collect();
        let mut i = 0;
        while i < children.len() {
            let child = children[i];
            match child.kind() {
                "comment" => {
                    self.format_comment(child);
                    self.newline();
                    i += 1;
                }
                "public" => {
                    self.emit("public ");
                    i += 1;
                }
                "attribute" => {
                    self.emit_node(child);
                    self.newline();
                    i += 1;
                }
                _ if child.is_named() => {
                    self.format_node(child);
                    i += 1;
                    if i < children.len() {
                        let next_kind = children[i].kind();
                        if next_kind == "public" || next_kind == "attribute" || next_kind == "comment"
                        {
                            self.newline();
                        } else {
                            self.newline();
                            self.newline();
                        }
                    }
                }
                _ => {
                    i += 1;
                }
            }
        }
    }

    fn format_node(&mut self, node: Node) {
        let kind = node.kind().to_string();
        match kind.as_str() {
            "function_definition" => self.format_function_def(node),
            "struct_definition" => self.format_struct_def(node),
            "tuple_definition" => self.format_tuple_def(node),
            "enum_definition" => self.format_enum_def(node),
            "block" => self.format_block(node),
            "parameters" | "arguments" => self.format_parenthesized_list(node),
            "type_annotation" => self.format_type_annotation(node),
            "binary_expression" | "pipe_expression" => self.format_infix(node),
            "unary_expression" => self.format_prefix(node),
            "if_expression" => self.format_if(node),
            "while_statement" => self.format_while(node),
            "loop_statement" => self.format_loop(node),
            "match_expression" => self.format_match(node),
            "match_arm" => self.format_match_arm(node),
            "import_statement" => self.format_import(node),
            "let_declaration" => self.format_let(node),
            "return_statement" => self.format_return(node),
            "expression_statement" => self.format_expr_stmt(node),
            "assignment_statement" => self.format_assignment(node),
            "call_expression" => self.format_call(node),
            "parenthesized_expression" => self.format_paren(node),
            "tuple_expression" => self.format_tuple_expr(node),
            "string_literal" | "raw_string_literal" | "char_literal" | "integer_literal" | "float_literal" | "bool_literal" | "unit_literal" | "primitive_type" => {
                 self.emit_node(node);
            }
            "struct_literal_expression" => self.format_struct_literal(node),
             "scoped_type_expression" | "scoped_value_expression" => self.format_scoped(node),
            "comment" => self.format_comment(node),
            _ => self.format_default(node),
        }
    }

    fn format_default(&mut self, node: Node) {
        let mut cursor = node.walk();
        let mut has_children = false;
        for child in node.children(&mut cursor) {
            has_children = true;
            if child.is_named() {
                self.format_node(child);
            } else {
                self.emit_node(child);
            }
        }
        if !has_children && node.is_named() {
            self.emit_node(node);
        }
    }

    fn format_function_def(&mut self, node: Node) {
        self.emit("fn ");
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            let kind = child.kind().to_string();
            match kind.as_str() {
                "value_identifier" | "type_identifier" => self.emit_node(child),
                "parameters" => self.format_parenthesized_list(child),
                "type_annotation" => self.format_type_annotation(child),
                "block" => {
                    self.emit(" ");
                    self.format_block(child);
                }
                _ => {}
            }
        }
    }

    fn format_struct_def(&mut self, node: Node) {
        self.emit("struct ");
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            let kind = child.kind().to_string();
            match kind.as_str() {
                "value_identifier" | "type_identifier" => self.emit_node(child),
                "{" => self.emit(" {"),
                "}" => {
                    self.newline();
                    self.emit("}");
                }
                "field_definition" => {
                    self.newline();
                    self.indent += 1;
                    self.format_node(child);
                    self.indent -= 1;
                }
                "," => self.emit(","),
                _ => {
                    if child.is_named() {
                        self.format_node(child);
                    } else {
                        let text = self.text(child).to_string();
                        if !text.trim().is_empty() {
                            self.emit(&text);
                        }
                    }
                }
            }
        }
    }

    fn format_tuple_def(&mut self, node: Node) {
        self.emit("tuple ");
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            let kind = child.kind().to_string();
            match kind.as_str() {
                "value_identifier" | "type_identifier" => self.emit_node(child),
                "(" => self.emit("("),
                ")" => self.emit(")"),
                "," => self.emit(", "),
                _ => {
                    if child.is_named() {
                        self.format_node(child);
                    } else {
                        self.emit_node(child);
                    }
                }
            }
        }
    }

    fn format_enum_def(&mut self, node: Node) {
        self.emit("enum ");
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            let kind = child.kind().to_string();
            match kind.as_str() {
                "value_identifier" | "type_identifier" => self.emit_node(child),
                "{" => self.emit(" {"),
                "}" => {
                    self.newline();
                    self.emit("}");
                }
                "enum_variant" => {
                    self.newline();
                    self.indent += 1;
                    self.format_node(child);
                    self.indent -= 1;
                }
                "," => self.emit(","),
                _ => {}
            }
        }
    }

    fn format_type_annotation(&mut self, node: Node) {
        self.emit(": ");
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.is_named() {
                self.format_node(child);
            }
        }
    }

    fn format_parenthesized_list(&mut self, node: Node) {
        self.emit("(");
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            let kind = child.kind().to_string();
            if kind.as_str() == "(" || kind.as_str() == ")" {
                continue;
            }
            if kind.as_str() == "," {
                self.emit(", ");
            } else if child.is_named() {
                self.format_node(child);
            }
        }
        self.emit(")");
    }

    fn format_block(&mut self, node: Node) {
        let count = node.child_count();
        if count <= 2 {
            self.emit("{}");
            return;
        }
        self.emit("{");
        self.newline();
        self.indent += 1;
        for i in 1..count - 1 {
            if let Some(child) = node.child(i as u32)
                && (child.is_named() || child.kind() == "comment")
            {
                self.format_node(child);
                self.newline();
            }
        }
        self.indent -= 1;
        self.emit("}");
    }

    fn format_infix(&mut self, node: Node) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.is_named() {
                self.format_node(child);
            } else {
                let text = self.text(child).to_string();
                let trimmed = text.trim().to_string();
                if !trimmed.is_empty() {
                    self.emit(&format!(" {} ", trimmed));
                }
            }
        }
    }

    fn format_prefix(&mut self, node: Node) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.is_named() {
                self.format_node(child);
            } else {
                self.emit_node(child);
            }
        }
    }

    fn format_if(&mut self, node: Node) {
        self.emit("if ");
        let mut cursor = node.walk();
        let mut after_else = false;
        for child in node.children(&mut cursor) {
            let kind = child.kind().to_string();
            match kind.as_str() {
                "if" => continue,
                "else" => after_else = true,
                "block" => {
                    if after_else {
                        self.emit(" else ");
                        after_else = false;
                    } else {
                        self.emit(" ");
                    }
                    self.format_block(child);
                }
                _ if child.is_named() => {
                    if after_else {
                        self.emit(" else if ");
                        after_else = false;
                    }
                    self.format_node(child);
                }
                _ => {}
            }
        }
    }

    fn format_while(&mut self, node: Node) {
        self.emit("while ");
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            let kind = child.kind().to_string();
            match kind.as_str() {
                "while" => {}
                "block" => {
                    self.emit(" ");
                    self.format_block(child);
                }
                _ if child.is_named() => {
                    self.format_node(child);
                }
                _ => {}
            }
        }
    }

    fn format_loop(&mut self, node: Node) {
        self.emit("loop ");
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "loop" {
                continue;
            }
            if child.is_named() {
                self.format_block(child);
            }
        }
    }

    fn format_match(&mut self, node: Node) {
        self.emit("match ");
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            let kind = child.kind().to_string();
            match kind.as_str() {
                "match" => {}
                "{" => {
                    self.emit(" {");
                    self.newline();
                    self.indent += 1;
                }
                "}" => {
                    self.indent -= 1;
                    self.emit("}");
                }
                "match_arm" => {
                    self.format_node(child);
                    self.newline();
                }
                _ if child.is_named() => {
                    self.format_node(child);
                }
                _ => {}
            }
        }
    }

    fn format_match_arm(&mut self, node: Node) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            let kind = child.kind().to_string();
            match kind.as_str() {
                "=>" => self.emit(" => "),
                "," => {}
                _ if child.is_named() => self.format_node(child),
                _ => self.emit_node(child),
            }
        }
    }

    fn format_import(&mut self, node: Node) {
        self.emit("import ");
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            let kind = child.kind().to_string();
            match kind.as_str() {
                "import" => {}
                ";" => self.emit(";"),
                "::" => self.emit("::"),
                _ if child.is_named() => self.format_node(child),
                _ => self.emit_node(child),
            }
        }
    }

    fn format_let(&mut self, node: Node) {
        self.emit("let ");
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            let kind = child.kind().to_string();
            match kind.as_str() {
                "let" => {}
                "mut" => self.emit("mut "),
                "=" => self.emit(" = "),
                ";" => self.emit(";"),
                _ if child.is_named() => self.format_node(child),
                _ => {
                    let text = self.text(child).to_string();
                    if text == ":" {
                        self.emit(": ");
                    } else {
                        self.emit(&text);
                    }
                }
            }
        }
    }

    fn format_return(&mut self, node: Node) {
        self.emit("return");
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            let kind = child.kind().to_string();
            match kind.as_str() {
                "return" | ";" => {}
                _ if child.is_named() => {
                    self.emit(" ");
                    self.format_node(child);
                    self.emit(";");
                }
                _ => {}
            }
        }
    }

    fn format_expr_stmt(&mut self, node: Node) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            let kind = child.kind().to_string();
            if kind.as_str() == ";" {
                self.emit(";");
            } else if child.is_named() {
                self.format_node(child);
            } else {
                self.emit_node(child);
            }
        }
    }

    fn format_assignment(&mut self, node: Node) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.is_named() {
                self.format_node(child);
            } else {
                let text = self.text(child).to_string();
                let trimmed = text.trim().to_string();
                if trimmed == "=" {
                    self.emit(" = ");
                } else if trimmed.ends_with('=') {
                    self.emit(&format!(" {} ", trimmed));
                } else if trimmed == ";" {
                    self.emit(";");
                } else {
                    self.emit(&trimmed);
                }
            }
        }
    }

    fn format_call(&mut self, node: Node) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.is_named() {
                self.format_node(child);
            } else {
                self.emit_node(child);
            }
        }
    }

    fn format_paren(&mut self, node: Node) {
        self.emit("(");
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.is_named() {
                self.format_node(child);
            }
        }
        self.emit(")");
    }

    fn format_tuple_expr(&mut self, node: Node) {
        self.emit("(");
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if !child.is_named() {
                let text = self.text(child).to_string();
                if text == "(" || text == ")" {
                    continue;
                }
                if text == "," {
                    self.emit(", ");
                }
            } else {
                self.format_node(child);
            }
        }
        self.emit(")");
    }

    fn format_struct_literal(&mut self, node: Node) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.is_named() {
                self.format_node(child);
            } else {
                self.emit_node(child);
            }
        }
    }

    fn format_scoped(&mut self, node: Node) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.is_named() {
                self.format_node(child);
            } else {
                self.emit_node(child);
            }
        }
    }

    fn format_comment(&mut self, node: Node) {
        self.emit_node(node);
    }
}
