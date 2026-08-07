use miette::{Diagnostic, NamedSource, SourceSpan};
use thiserror::Error;
use tree_sitter::Tree;

#[derive(Debug, Error, Diagnostic)]
#[error("{kind}")]
pub struct ParserDiagnostic {
    #[diagnostic(transparent)]
    pub kind: ParserDiagnosticKind,

    #[source_code]
    pub source_code: NamedSource<String>,

    #[label]
    pub span: SourceSpan,
}

#[derive(Debug, Error, Diagnostic)]
pub enum ParserDiagnosticKind {
    #[error("enum variants are public by default; remove the `public` modifier")]
    #[diagnostic(code(parser::enum_variant_public_modifier))]
    EnumVariantPublicModifier,

    #[error("expected `{expected}`")]
    #[diagnostic(code(parser::missing_token))]
    #[help("add `{expected}` here")]
    MissingToken { expected: String },

    #[error("unexpected token `{token}`")]
    #[diagnostic(code(parser::unexpected_token))]
    UnexpectedToken { token: String },

    #[error("{message}")]
    #[diagnostic(code(parser::lowering_error))]
    Lowering { message: String },
}

pub(crate) fn validate_with_name(
    filename: &str,
    tree: &Tree,
    source: &str,
) -> Vec<ParserDiagnostic> {
    let mut diagnostics = Vec::new();
    let mut cursor = tree.walk();
    let shared_source = NamedSource::new(filename, source.to_string());

    fn visit(
        cursor: &mut tree_sitter::TreeCursor,
        source: &str,
        shared_source: &NamedSource<String>,
        diagnostics: &mut Vec<ParserDiagnostic>,
    ) {
        let node = cursor.node();

        if !node.has_error() {
            return;
        }

        if node.is_missing() || node.is_error() {
            let start = node.start_byte();
            let end = node.end_byte();
            let span = SourceSpan::from(start..end);

            let kind = if node.is_missing() {
                ParserDiagnosticKind::MissingToken {
                    expected: node.kind().to_string(),
                }
            } else {
                let snippet = source.get(start..end).unwrap_or("");
                let token = snippet.chars().take(40).collect::<String>();
                ParserDiagnosticKind::UnexpectedToken { token }
            };

            diagnostics.push(ParserDiagnostic {
                kind,
                source_code: shared_source.clone(),
                span,
            });
        }

        if cursor.goto_first_child() {
            loop {
                visit(cursor, source, shared_source, diagnostics);
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }
    }

    visit(&mut cursor, source, &shared_source, &mut diagnostics);
    diagnostics
}
