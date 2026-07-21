use miette::{Diagnostic, NamedSource, SourceSpan};
use std::error::Error;
use std::fmt;
use tree_sitter::Tree;

#[derive(Debug, Diagnostic)]
#[diagnostic()]
pub struct ParseError {
    #[source_code]
    pub source: NamedSource<String>,

    #[label]
    pub span: SourceSpan,

    #[help]
    pub help: Option<String>,

    pub message: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl Error for ParseError {}

pub(crate) fn validate_with_name(filename: &str, tree: &Tree, source: &str) -> Vec<ParseError> {
    let mut errors = Vec::new();
    let cursor = &mut tree.walk();

    fn visit(
        cursor: &mut tree_sitter::TreeCursor,
        filename: &str,
        source: &str,
        errors: &mut Vec<ParseError>,
    ) {
        let node = cursor.node();
        if node.is_error() || node.is_missing() {
            let start = node.start_byte();
            let end = node.end_byte();

            let (message, help) = if node.is_missing() {
                let expected = node.kind();
                (
                    format!("expected `{expected}`"),
                    Some(format!("add `{expected}` here")),
                )
            } else {
                let snippet = &source[start..end];
                let context = snippet.chars().take(40).collect::<String>();
                let help = suggest_fix(snippet, source, start, end);
                (format!("unexpected `{context}`"), help)
            };

            errors.push(ParseError {
                message,
                help,
                source: NamedSource::new(filename, source.to_string()),
                span: SourceSpan::from(start..end),
            });
        }

        if cursor.goto_first_child() {
            loop {
                visit(cursor, filename, source, errors);
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }
    }

    visit(cursor, filename, source, &mut errors);
    errors
}

fn suggest_fix(snippet: &str, _source: &str, _start: usize, _end: usize) -> Option<String> {
    let opens = snippet.matches('(').count();
    let closes = snippet.matches(')').count();
    if opens > closes {
        return Some("add closing `)`".into());
    }

    let dq_open = snippet.matches('"').count();
    if !dq_open.is_multiple_of(2) {
        return Some("add closing `\"`".into());
    }

    if !snippet.ends_with(';') && !snippet.ends_with('}') && !snippet.ends_with('{') {
        let trimmed = snippet.trim_end();
        if trimmed.ends_with(')') || trimmed.ends_with('"') || trimmed.ends_with('}') {
            return Some("add `;` after this".into());
        }
    }

    None
}
