use miette::SourceSpan;

/// A pattern used by a match arm.
#[derive(Debug, Clone)]
pub enum Pattern {
    Wildcard(SourceSpan),
    Ident(String, SourceSpan),
    Literal(LiteralPattern, SourceSpan),
    Struct {
        span: SourceSpan,
        name: String,
        fields: Vec<(String, Pattern)>,
    },
    Tuple(Vec<Pattern>, SourceSpan),
    EnumVariant {
        span: SourceSpan,
        type_path: String,
        variant_name: String,
        patterns: Vec<Pattern>,
    },
}

impl Pattern {
    /// Returns the source span covering the pattern.
    pub fn span(&self) -> SourceSpan {
        match self {
            Pattern::Wildcard(s) => *s,
            Pattern::Ident(_, s) => *s,
            Pattern::Literal(_, s) => *s,
            Pattern::Struct { span, .. } => *span,
            Pattern::Tuple(_, s) => *s,
            Pattern::EnumVariant { span, .. } => *span,
        }
    }
}

/// A literal value used in a pattern.
#[derive(Debug, Clone)]
pub enum LiteralPattern {
    Int(i128),
    Bool(bool),
    Char(char),
    String(String),
}
