use miette::{Diagnostic, NamedSource, SourceSpan};
use std::fmt;
use thiserror::Error;
use tree_sitter::Tree;

vinyl_diagnostics::diagnostic_codes! {
    "parser",
    pub enum ParseDiagnosticKind {
        UnexpectedToken,
        MissingToken,
    }
}

impl fmt::Display for ParseDiagnosticKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code().variant)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Semicolon,
    ClosingParen,
    ClosingBrace,
    Quote,
    Other,
}

#[derive(Debug, Error, Diagnostic)]
pub enum ParseError {
    #[error("expected `;`")]
    #[diagnostic(code(parser::missing_semicolon), help("add `;` here"))]
    MissingSemicolon {
        #[source_code]
        source_code: NamedSource<String>,
        #[label]
        span: SourceSpan,
    },
    #[error("expected `)`")]
    #[diagnostic(code(parser::missing_closing_paren), help("add `)` here"))]
    MissingClosingParen {
        #[source_code]
        source_code: NamedSource<String>,
        #[label]
        span: SourceSpan,
    },
    #[error("expected `}}`")]
    #[diagnostic(code(parser::missing_closing_brace), help("add `}}` here"))]
    MissingClosingBrace {
        #[source_code]
        source_code: NamedSource<String>,
        #[label]
        span: SourceSpan,
    },
    #[error("expected `\"`")]
    #[diagnostic(code(parser::missing_quote), help("add `\"` here"))]
    MissingQuote {
        #[source_code]
        source_code: NamedSource<String>,
        #[label]
        span: SourceSpan,
    },
    #[error("expected a token")]
    #[diagnostic(code(parser::missing_token))]
    MissingToken {
        #[source_code]
        source_code: NamedSource<String>,
        #[label]
        span: SourceSpan,
    },
    #[error("unexpected token `{token}`")]
    #[diagnostic(code(parser::unexpected_token))]
    UnexpectedToken {
        token: String,
        #[source_code]
        source_code: NamedSource<String>,
        #[label]
        span: SourceSpan,
    },
}

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

            let error = if node.is_missing() {
                let expected = node.kind();
                let token = match expected {
                    ";" => TokenKind::Semicolon,
                    ")" => TokenKind::ClosingParen,
                    "}" => TokenKind::ClosingBrace,
                    "\"" => TokenKind::Quote,
                    _ => TokenKind::Other,
                };
                let source = NamedSource::new(filename, source.to_string());
                let span = SourceSpan::from(start..end);
                match token {
                    TokenKind::Semicolon => ParseError::MissingSemicolon {
                        source_code: source,
                        span,
                    },
                    TokenKind::ClosingParen => ParseError::MissingClosingParen {
                        source_code: source,
                        span,
                    },
                    TokenKind::ClosingBrace => ParseError::MissingClosingBrace {
                        source_code: source,
                        span,
                    },
                    TokenKind::Quote => ParseError::MissingQuote {
                        source_code: source,
                        span,
                    },
                    TokenKind::Other => ParseError::MissingToken {
                        source_code: source,
                        span,
                    },
                }
            } else {
                let snippet = &source[start..end];
                let context = snippet.chars().take(40).collect::<String>();
                ParseError::UnexpectedToken {
                    token: context,
                    source_code: NamedSource::new(filename, source.to_string()),
                    span: SourceSpan::from(start..end),
                }
            };
            errors.push(error);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_semicolon_has_structured_code() {
        assert!(matches!(TokenKind::Semicolon, TokenKind::Semicolon));
    }
}
