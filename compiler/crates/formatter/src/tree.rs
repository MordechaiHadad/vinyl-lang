use tree_sitter::Node;

use crate::FormatterConfig;
use crate::error::FormatError;

/// Formats a source string by walking its syntax tree and emitting normalized
/// tokens, preserving CRLF line endings when the source uses them.
pub(crate) fn format_source_with_config(
    source: &str,
    config: &FormatterConfig,
) -> Result<String, FormatError> {
    let tree = vinyl_parser::parse(source).map_err(|errors| {
        let diagnostic = errors
            .into_iter()
            .next()
            .expect("parser error list should never be empty");
        FormatError::Parse(Box::new(diagnostic))
    })?;
    let root = tree.root_node();
    let indent_str = " ".repeat(config.indent_width);
    let mut formatter = Formatter {
        source,
        output: String::new(),
        indent: 0,
        indent_str,
    };
    formatter.format_root(root);
    let normalized = formatter
        .output
        .replace("\r\n", "\n")
        .trim_end()
        .to_string();
    let with_newline = if source.ends_with('\n') && !normalized.ends_with('\n') {
        format!("{normalized}\n")
    } else {
        normalized
    };
    let output = if source.contains("\r\n") {
        with_newline.replace('\n', "\r\n")
    } else {
        with_newline
    };
    Ok(output)
}

// todo: support formatting a range of the source code, currently just formats the whole source
/// Formats a byte range of a source string.
///
/// Range formatting is not yet implemented; the whole source is formatted and
/// the range is ignored. The LSP and CLI format whole documents.
pub(crate) fn format_range(source: &str, config: &FormatterConfig) -> Result<String, FormatError> {
    format_source_with_config(source, config)
}

/// The formatting engine: walks the syntax tree and accumulates normalized
/// output, tracking the current indentation level.
struct Formatter<'a> {
    /// The original source, used to emit token text verbatim.
    source: &'a str,
    /// The formatted output accumulated so far.
    output: String,
    /// Current indentation level, in units of `indent_str`.
    indent: usize,
    /// One level of indentation (e.g. four spaces).
    indent_str: String,
}

impl<'a> Formatter<'a> {
    /// Returns the node's exact slice of the source.
    fn text(&self, node: Node) -> &'a str {
        &self.source[node.start_byte()..node.end_byte()]
    }

    /// Returns the node's source text with surrounding whitespace trimmed.
    fn trimmed_text(&self, node: Node) -> &'a str {
        self.text(node).trim()
    }

    /// Appends `text`, first writing `indent` levels of indentation when the
    /// output is at the start of a line.
    fn emit(&mut self, text: &str) {
        if self.output.is_empty() || self.output.ends_with('\n') {
            for _ in 0..self.indent {
                self.output.push_str(&self.indent_str);
            }
        }
        self.output.push_str(text);
    }

    /// Emits the node's source text verbatim.
    fn emit_node(&mut self, node: Node) {
        let text = self.text(node);
        self.emit(text);
    }

    /// Formats a child node, dispatching on whether it is named.
    fn format_child(&mut self, child: Node) {
        if child.is_named() {
            self.format_node(child);
        } else {
            self.emit_node(child);
        }
    }

    /// Appends a newline to the output.
    fn newline(&mut self) {
        self.output.push('\n');
    }

    /// Formats the top-level definitions of a source file, separating items
    /// with a blank line while preserving a single blank line where present.
    fn format_root(&mut self, node: Node) {
        let mut cursor = node.walk();
        let children: Vec<Node> = node.children(&mut cursor).collect();
        let mut i = 0;
        let mut prev_end: Option<usize> = None;
        while i < children.len() {
            let child = children[i];
            match child.kind() {
                "public" => {
                    self.preserve_gap(prev_end, child.start_byte());
                    self.emit("public ");
                    i += 1;
                }
                "comment" => {
                    self.preserve_gap(prev_end, child.start_byte());
                    self.format_comment(child);
                    self.newline();
                    prev_end = Some(child.end_byte());
                    i += 1;
                }
                "attribute" => {
                    self.preserve_gap(prev_end, child.start_byte());
                    self.emit_node(child);
                    self.newline();
                    prev_end = Some(child.end_byte());
                    i += 1;
                }
                _ if child.is_named() => {
                    self.format_node(child);
                    prev_end = Some(child.end_byte());
                    i += 1;
                    if i < children.len() {
                        let next_kind = children[i].kind();
                        if next_kind == "public"
                            || next_kind == "attribute"
                            || next_kind == "comment"
                            || (next_kind == "import_statement"
                                && child.kind() == "import_statement")
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

    /// Collapses a gap of two or more newlines to a single blank line.
    fn preserve_gap(&mut self, prev_end: Option<usize>, start: usize) {
        if let Some(prev) = prev_end
            && start > prev
            && self.source[prev..start]
                .chars()
                .filter(|&c| c == '\n')
                .count()
                > 1
        {
            self.newline();
        }
    }

    /// Dispatches a named node to its formatter, falling back to
    /// [`format_default`](Self::format_default) for unhandled node kinds.
    fn format_node(&mut self, node: Node) {
        match node.kind() {
            "function_definition" => self.format_function_def(node),
            "struct_definition" => self.format_struct_def(node),
            "field_definition" => self.format_field_def(node),
            "tuple_definition" => self.format_tuple_def(node),
            "enum_definition" => self.format_enum_def(node),
            "type_alias_definition" => self.format_type_alias(node),
            "block" => self.format_block(node),
            "parameter" => self.format_parameter(node),
            "parameters"
            | "arguments"
            | "tuple_type"
            | "parenthesized_expression"
            | "tuple_expression" => self.format_paren_list(node),
            "type_annotation" => self.format_type_annotation(node),
            "binary_expression" | "pipe_expression" => self.format_infix(node),
            "unary_expression" => self.format_prefix(node),
            "if_expression" => self.format_if(node),
            "while_statement" => self.format_while(node),
            "loop_statement" => self.format_loop(node),
            "match_expression" => self.format_match(node),
            "match_arm" => self.format_match_arm(node),
            "array_type" | "array_expression" => self.format_array(node),
            "struct_literal_expression" => self.format_struct_literal_expression(node),
            "struct_literal_fields" => self.format_struct_literal_fields(node),
            "struct_pattern" => self.format_struct_pattern(node),
            "field_pattern" => self.format_field_pattern(node),
            "enum_variant" => self.format_enum_variant(node),
            "import_group" => self.format_import_group(node),
            "import_statement" => self.format_import(node),
            "let_declaration" => self.format_let(node),
            "return_statement" => self.format_return(node),
            "expression_statement" => self.format_expr_stmt(node),
            "assignment_statement" => self.format_assignment(node),
            "string_literal" | "raw_string_literal" | "char_literal" | "integer_literal"
            | "float_literal" | "bool_literal" | "unit_literal" | "primitive_type" => {
                self.emit_node(node);
            }
            "comment" => self.format_comment(node),
            _ => self.format_default(node),
        }
    }

    /// Formats an unrecognized node by recursively formatting its children,
    /// emitting a leaf node's source text verbatim.
    fn format_default(&mut self, node: Node) {
        if node.child_count() == 0 && node.is_named() {
            self.emit_node(node);
            return;
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.format_child(child);
        }
    }

    /// Formats a `fn` definition: name, parameters, optional return type, and
    /// body block.
    fn format_function_def(&mut self, node: Node) {
        self.emit("fn ");
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "value_identifier" | "type_identifier" => self.emit_node(child),
                "parameters" => self.format_paren_list(child),
                "type_annotation" => self.format_type_annotation(child),
                "block" => {
                    self.emit(" ");
                    self.format_block(child);
                }
                _ => {}
            }
        }
    }

    /// Formats a struct definition with each field on its own line.
    fn format_struct_def(&mut self, node: Node) {
        self.format_braced_members(node, "struct", "field_definition");
    }

    /// Formats a struct field, emitting the `public` keyword when present.
    fn format_field_def(&mut self, node: Node) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.is_named() {
                self.format_node(child);
            } else {
                match child.kind() {
                    "public" => self.emit("public "),
                    ":" => self.emit(": "),
                    _ => {}
                }
            }
        }
    }

    /// Formats a tuple type definition.
    fn format_tuple_def(&mut self, node: Node) {
        self.emit("tuple ");
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "value_identifier" | "type_identifier" => self.emit_node(child),
                "tuple" => {}
                "(" => self.emit(" ("),
                ")" => self.emit(")"),
                "," => self.emit(", "),
                _ if child.is_named() => self.format_node(child),
                _ => self.emit_node(child),
            }
        }
    }

    /// Formats an enum definition with each variant on its own line.
    fn format_enum_def(&mut self, node: Node) {
        self.format_braced_members(node, "enum", "enum_variant");
    }

    /// Formats a `struct` or `enum` definition: each `member_kind` child goes
    /// on its own indented line, with members separated by commas.
    fn format_braced_members(&mut self, node: Node, keyword: &str, member_kind: &str) {
        self.emit(&format!("{keyword} "));
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "{" => self.emit(" {"),
                "}" => {
                    self.newline();
                    self.emit("}");
                }
                "," => self.emit(","),
                _ if child.kind() == keyword => {}
                _ if child.kind() == member_kind => {
                    self.newline();
                    self.indent += 1;
                    self.format_node(child);
                    self.indent -= 1;
                }
                _ if child.is_named() => self.format_node(child),
                _ => {
                    let text = self.trimmed_text(child);
                    if !text.is_empty() {
                        self.emit(text);
                    }
                }
            }
        }
    }

    /// Formats a `type` alias definition.
    fn format_type_alias(&mut self, node: Node) {
        self.emit("type ");
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "type" => {}
                "=" => self.emit(" = "),
                ";" => self.emit(";"),
                _ if child.is_named() => self.format_node(child),
                _ => self.emit_node(child),
            }
        }
    }

    /// Formats a `: Type` annotation.
    fn format_type_annotation(&mut self, node: Node) {
        self.emit(": ");
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.is_named() {
                self.format_node(child);
            }
        }
    }

    /// Formats a comma-separated list inside parentheses (parameters,
    /// arguments, tuple types, parenthesized and tuple expressions).
    fn format_paren_list(&mut self, node: Node) {
        self.emit("(");
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "(" | ")" => {}
                "," => self.emit(", "),
                _ if child.is_named() => self.format_node(child),
                _ => {}
            }
        }
        self.emit(")");
    }

    /// Formats a bracketed array, either a list `[a, b]` or a fill `[v; s]`,
    /// for both values and types.
    fn format_array(&mut self, node: Node) {
        self.emit("[");
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "[" | "]" => {}
                ";" => self.emit("; "),
                "," => self.emit(", "),
                _ if child.is_named() => self.format_node(child),
                _ => {}
            }
        }
        self.emit("]");
    }

    /// Formats a struct literal inline: `Name { x: 1, y: 2 }`.
    fn format_struct_literal_expression(&mut self, node: Node) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "struct_literal_fields" => {
                    self.emit(" ");
                    self.format_struct_literal_fields(child);
                }
                _ if child.is_named() => self.format_node(child),
                _ => {}
            }
        }
    }

    /// Formats the braces and field pairs of a struct literal inline.
    fn format_struct_literal_fields(&mut self, node: Node) {
        if node.child_count() <= 2 {
            self.emit("{}");
            return;
        }
        self.emit("{ ");
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "{" | "}" => {}
                "," => self.emit(", "),
                ":" => self.emit(": "),
                _ if child.is_named() => self.format_node(child),
                _ => {}
            }
        }
        self.emit(" }");
    }

    /// Formats a struct pattern inline: `Name { x, y: pat }`.
    fn format_struct_pattern(&mut self, node: Node) {
        let mut cursor = node.walk();
        let children: Vec<Node> = node.children(&mut cursor).collect();
        let has_fields = children.len() > 3;
        for child in children {
            match child.kind() {
                "{" => self.emit(if has_fields { " { " } else { " {" }),
                "}" => self.emit(if has_fields { " }" } else { "}" }),
                "," => self.emit(", "),
                _ if child.is_named() => self.format_node(child),
                _ => {}
            }
        }
    }

    /// Formats a single struct pattern field, padding the colon with spaces.
    fn format_field_pattern(&mut self, node: Node) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                ":" => self.emit(": "),
                _ if child.is_named() => self.format_node(child),
                _ => {}
            }
        }
    }

    /// Formats an enum variant, keeping its tuple payload unspaced and its
    /// braced payload spaced: `Variant(int32)` or `Variant { public x: int }`.
    fn format_enum_variant(&mut self, node: Node) {
        let mut cursor = node.walk();
        let children: Vec<Node> = node.children(&mut cursor).collect();
        let brace_has_fields = children
            .iter()
            .any(|child| child.kind() == "field_definition");
        for child in children {
            match child.kind() {
                "(" => self.emit("("),
                ")" => self.emit(")"),
                "{" => self.emit(if brace_has_fields { " { " } else { " {" }),
                "}" => self.emit(if brace_has_fields { " }" } else { "}" }),
                "," => self.emit(", "),
                _ if child.is_named() => self.format_node(child),
                _ => {}
            }
        }
    }

    /// Formats an import symbol group: `import foo::{a, b}`.
    fn format_import_group(&mut self, node: Node) {
        self.emit("{");
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "{" | "}" => {}
                "," => self.emit(", "),
                _ if child.is_named() => self.format_node(child),
                _ => {}
            }
        }
        self.emit("}");
    }

    /// Formats a parameter, emitting the `mut` keyword when present.
    fn format_parameter(&mut self, node: Node) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "mut" {
                self.emit("mut ");
            } else if child.is_named() {
                self.format_node(child);
            }
        }
    }

    /// Formats a block, preserving a single blank line between statements.
    fn format_block(&mut self, node: Node) {
        let count = node.child_count();
        if count <= 2 {
            self.emit("{}");
            return;
        }
        self.emit("{");
        self.newline();
        self.indent += 1;
        let mut prev_end: Option<usize> = None;
        for i in 1..count - 1 {
            if let Some(child) = node.child(i as u32)
                && (child.is_named() || child.kind() == "comment")
            {
                self.preserve_gap(prev_end, child.start_byte());
                self.format_node(child);
                self.newline();
                prev_end = Some(child.end_byte());
            }
        }
        self.indent -= 1;
        self.emit("}");
    }

    /// Formats a binary or pipe expression, padding operators with spaces.
    fn format_infix(&mut self, node: Node) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.is_named() {
                self.format_node(child);
            } else {
                let text = self.trimmed_text(child);
                if !text.is_empty() {
                    self.emit(&format!(" {text} "));
                }
            }
        }
    }

    /// Formats a unary expression.
    fn format_prefix(&mut self, node: Node) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.format_child(child);
        }
    }

    /// Formats an if/else-if/else expression.
    fn format_if(&mut self, node: Node) {
        self.emit("if ");
        let mut cursor = node.walk();
        let mut after_else = false;
        for child in node.children(&mut cursor) {
            match child.kind() {
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

    /// Formats a `while` loop.
    fn format_while(&mut self, node: Node) {
        self.emit("while ");
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
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

    /// Formats an infinite `loop` block.
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

    /// Formats a `match` expression with each arm on its own indented line.
    fn format_match(&mut self, node: Node) {
        self.emit("match ");
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
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

    /// Formats a single match arm, padding the `=>` and any guard `if` with
    /// spaces.
    fn format_match_arm(&mut self, node: Node) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "=>" => self.emit(" => "),
                "," => {}
                "if" => self.emit(" if "),
                _ if child.is_named() => self.format_node(child),
                _ => self.emit_node(child),
            }
        }
    }

    /// Formats an import statement.
    fn format_import(&mut self, node: Node) {
        self.emit("import ");
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "import" => {}
                ";" => self.emit(";"),
                "::" => self.emit("::"),
                _ if child.is_named() => self.format_node(child),
                _ => self.emit_node(child),
            }
        }
    }

    /// Formats a `let` declaration, emitting the `mut` keyword when present.
    fn format_let(&mut self, node: Node) {
        self.emit("let ");
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "let" => {}
                "mut" => self.emit("mut "),
                "=" => self.emit(" = "),
                ";" => self.emit(";"),
                _ if child.is_named() => self.format_node(child),
                _ => self.emit_node(child),
            }
        }
    }

    /// Formats a return statement, always emitting the trailing semicolon.
    fn format_return(&mut self, node: Node) {
        self.emit("return");
        let mut has_value = false;
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.is_named() {
                self.emit(" ");
                self.format_node(child);
                self.emit(";");
                has_value = true;
            }
        }
        if !has_value {
            self.emit(";");
        }
    }

    /// Formats an expression statement, emitting the trailing semicolon.
    fn format_expr_stmt(&mut self, node: Node) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == ";" {
                self.emit(";");
            } else {
                self.format_child(child);
            }
        }
    }

    /// Formats an assignment statement, padding the operator with spaces.
    fn format_assignment(&mut self, node: Node) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.is_named() {
                self.format_node(child);
            } else {
                let trimmed = self.trimmed_text(child);
                if trimmed == "=" {
                    self.emit(" = ");
                } else if trimmed.ends_with('=') {
                    self.emit(&format!(" {trimmed} "));
                } else if trimmed == ";" {
                    self.emit(";");
                } else {
                    self.emit(trimmed);
                }
            }
        }
    }

    /// Emits a comment verbatim.
    fn format_comment(&mut self, node: Node) {
        self.emit_node(node);
    }
}
