pub mod ast;
pub mod error;
pub mod lower;

pub use ast::*;
pub use error::ParserDiagnostic;
use tree_sitter::{Parser, Tree};

unsafe extern "C" {
    fn tree_sitter_vinyl() -> *const tree_sitter::ffi::TSLanguage;
}

fn language() -> tree_sitter::Language {
    unsafe { tree_sitter::Language::from_raw(tree_sitter_vinyl()) }
}

pub fn parse(source: &str) -> Result<Tree, Vec<ParserDiagnostic>> {
    parse_with_name("<input>", source)
}

pub fn parse_and_lower(source: &str) -> Result<Vec<Item>, Vec<ParserDiagnostic>> {
    parse_and_lower_with_name("<input>", source)
}

pub fn parse_with_name(filename: &str, source: &str) -> Result<Tree, Vec<ParserDiagnostic>> {
    let tree = parse_tree(source);
    let errors = error::validate_with_name(filename, &tree, source);
    if errors.is_empty() {
        Ok(tree)
    } else {
        Err(errors)
    }
}

/// Parses without validating, returning the tree-sitter tree even when the source
/// contains error or missing nodes (e.g. while the user is mid-edit).
pub fn parse_tree(source: &str) -> Tree {
    let mut parser = Parser::new();
    parser
        .set_language(&language())
        .expect("vinyl language should be valid");
    parser
        .parse(source, None)
        .expect("tree-sitter parse should not fail")
}

/// Byte range of the statement that `offset` falls inside, i.e. the outermost node
/// enclosing the cursor whose parent is a `block` or the `source_file`.
///
/// Deleting that range from the source yields a file that still parses, which is
/// what the LSP uses to analyze a partially-typed statement (e.g. `x.` with a
/// missing field) for completions.
pub fn statement_range_at(tree: &Tree, offset: usize) -> Option<(usize, usize)> {
    let query = offset.saturating_sub(1);
    let mut node = tree.root_node().descendant_for_byte_range(query, query)?;
    loop {
        match node.parent() {
            Some(parent) if matches!(parent.kind(), "block" | "source_file") => {
                return Some((node.start_byte(), node.end_byte()));
            }
            Some(parent) => node = parent,
            None => return None,
        }
    }
}

pub fn parse_and_lower_with_name(
    filename: &str,
    source: &str,
) -> Result<Vec<Item>, Vec<ParserDiagnostic>> {
    let tree = parse_with_name(filename, source)?;
    lower::lower(&tree, source, filename)
}

#[cfg(test)]
mod statement_range_tests {
    use super::{parse_tree, parse_with_name, statement_range_at};

    fn cleaned(source: &str, offset: usize) -> String {
        let tree = parse_tree(source);
        match statement_range_at(&tree, offset) {
            Some((start, end)) => format!("{}{}", &source[..start], &source[end..]),
            None => source.to_string(),
        }
    }

    fn assert_clean_parses(source: &str, offset: usize) -> String {
        let cleaned = cleaned(source, offset);
        assert!(
            parse_with_name("<test>", &cleaned).is_ok(),
            "cleaned source should parse:\n{cleaned}"
        );
        cleaned
    }

    #[test]
    fn removes_incomplete_statement_around_cursor() {
        let pipe = "struct Name { public first: int, last: int }\nfn main() {\n    let x = Name { first: 1, last: 2 };\n    x. |> math::double()\n}\n";
        let plain = "fn main() {\n    let p = Point { x: 1, y: 2 };\n    p.\n}\n";
        let enumeration = "enum Shape { public Circle, public Square(float64) }\nfn main(): unit {\n    let s = Shape::\n}\n";
        let if_brace = "fn main() {\n    if true { x. }\n}\n";

        let pipe_clean = assert_clean_parses(pipe, pipe.find("x. |>").unwrap() + 2);
        assert!(
            !pipe_clean.contains("math::double"),
            "pipe statement should be removed:\n{pipe_clean}"
        );
        assert!(
            pipe_clean.contains("let x = Name"),
            "let declaration should stay:\n{pipe_clean}"
        );

        let plain_clean = assert_clean_parses(plain, plain.rfind('.').unwrap() + 1);
        assert!(
            !plain_clean.contains("p."),
            "field access should be removed:\n{plain_clean}"
        );

        let enum_clean = assert_clean_parses(enumeration, enumeration.find("::").unwrap() + 2);
        assert!(
            !enum_clean.contains("Shape::"),
            "variant statement should be removed:\n{enum_clean}"
        );

        let if_clean = assert_clean_parses(if_brace, if_brace.rfind('.').unwrap() + 1);
        assert!(
            !if_clean.contains("x."),
            "if-body statement should be removed:\n{if_clean}"
        );
        assert!(
            if_clean.contains("if true"),
            "if should stay balanced:\n{if_clean}"
        );
    }
}
