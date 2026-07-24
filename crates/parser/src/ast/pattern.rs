use miette::SourceSpan;

#[derive(Debug)]
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
        name: String,
        patterns: Vec<Pattern>,
    },
}

impl Pattern {
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

#[derive(Debug)]
pub enum LiteralPattern {
    Int(i128),
    Bool(bool),
    Char(char),
    String(String),
}
