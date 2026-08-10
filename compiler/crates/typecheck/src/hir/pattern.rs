use miette::SourceSpan;

use crate::hir::types::Type;

#[derive(Debug, Clone)]
pub struct HirPattern {
    pub kind: HirPatternKind,
    pub type_: Type,
}

impl HirPattern {
    pub fn span(&self) -> SourceSpan {
        match &self.kind {
            HirPatternKind::Wildcard(span)
            | HirPatternKind::Ident { span, .. }
            | HirPatternKind::Literal { span, .. }
            | HirPatternKind::Struct { span, .. }
            | HirPatternKind::Tuple { span, .. }
            | HirPatternKind::EnumVariant { span, .. } => *span,
        }
    }
}

#[derive(Debug, Clone)]
pub enum HirPatternKind {
    Wildcard(SourceSpan),
    Ident {
        span: SourceSpan,
        name: String,
    },
    Literal {
        span: SourceSpan,
        value: LiteralValue,
    },
    Struct {
        span: SourceSpan,
        type_name: String,
        fields: Vec<(String, HirPattern)>,
    },
    Tuple {
        span: SourceSpan,
        elements: Vec<HirPattern>,
    },
    EnumVariant {
        span: SourceSpan,
        type_name: String,
        variant_index: usize,
        patterns: Vec<HirPattern>,
    },
}

#[derive(Debug, Clone)]
pub enum LiteralValue {
    Int(i128),
    Bool(bool),
    Char(char),
    String(String),
}
