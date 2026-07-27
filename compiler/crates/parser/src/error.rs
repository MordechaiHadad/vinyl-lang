use miette::{Diagnostic, NamedSource, SourceSpan};
use std::error::Error;
use std::fmt;
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

impl TokenKind {
    const fn code_name(self) -> &'static str {
        match self {
            Self::Semicolon => "semicolon",
            Self::ClosingParen => "closing_paren",
            Self::ClosingBrace => "closing_brace",
            Self::Quote => "quote",
            Self::Other => "other",
        }
    }
}

#[derive(Debug, Diagnostic)]
#[diagnostic()]
pub struct ParseError {
    #[source_code]
    pub source: NamedSource<String>,

    #[label]
    pub span: SourceSpan,

    #[diagnostic(skip)]
    pub kind: ParseDiagnosticKind,
    #[diagnostic(skip)]
    pub expected: Option<TokenKind>,
    #[diagnostic(skip)]
    pub token: Option<String>,
}

impl ParseError {
    pub fn diagnostic_code(&self) -> vinyl_diagnostics::DetailedCode {
        self.kind
            .code()
            .with_detail(self.expected.map(TokenKind::code_name).unwrap_or("unknown"))
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            ParseDiagnosticKind::UnexpectedToken => {
                write!(
                    f,
                    "unexpected token `{}`",
                    self.token.as_deref().unwrap_or("")
                )
            }
            ParseDiagnosticKind::MissingToken => f.write_str("expected a token here"),
        }
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

            let (kind, expected, token) = if node.is_missing() {
                let expected = node.kind();
                let token = match expected {
                    ";" => TokenKind::Semicolon,
                    ")" => TokenKind::ClosingParen,
                    "}" => TokenKind::ClosingBrace,
                    "\"" => TokenKind::Quote,
                    _ => TokenKind::Other,
                };
                (ParseDiagnosticKind::MissingToken, Some(token), None)
            } else {
                let snippet = &source[start..end];
                let context = snippet.chars().take(40).collect::<String>();
                (ParseDiagnosticKind::UnexpectedToken, None, Some(context))
            };

            errors.push(ParseError {
                kind,
                expected,
                token,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_semicolon_has_structured_code() {
        assert_eq!(
            ParseDiagnosticKind::MissingToken
                .code()
                .with_detail(TokenKind::Semicolon.code_name())
                .to_string(),
            "parser::ParseDiagnosticKind::MissingToken::semicolon"
        );
    }
}
