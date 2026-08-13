use miette::SourceSpan;

use crate::ast::{expression::Expression, statement::Statement, types::Type};

/// A top-level declaration.
#[derive(Debug, Clone)]
pub enum Item {
    Function(FunctionDef),
    Struct(StructDef),
    TupleStruct(TupleDef),
    Enum(EnumDef),
    TypeAlias(TypeAliasDef),
    Import(ImportDef),
}

impl Item {
    /// Returns the source span covering the declaration.
    pub fn span(&self) -> SourceSpan {
        match self {
            Item::Function(f) => f.span,
            Item::Struct(s) => s.span,
            Item::TupleStruct(t) => t.span,
            Item::Enum(e) => e.span,
            Item::TypeAlias(a) => a.span,
            Item::Import(i) => i.span,
        }
    }
}

/// A function declaration.
#[derive(Debug, Clone)]
pub struct FunctionDef {
    pub span: SourceSpan,
    pub public: bool,
    pub attrs: Vec<Attribute>,
    pub name: String,
    pub params: Vec<FunctionParam>,
    pub return_type: Option<Type>,
    pub body: Vec<Statement>,
}

/// A function parameter.
#[derive(Debug, Clone)]
pub struct FunctionParam {
    pub span: SourceSpan,
    pub name: String,
    pub mutable: bool,
    pub type_: Type,
}

/// A named structure declaration.
#[derive(Debug, Clone)]
pub struct StructDef {
    pub span: SourceSpan,
    pub public: bool,
    pub attrs: Vec<Attribute>,
    pub name: String,
    pub fields: Vec<StructField>,
}

/// A structure field.
#[derive(Debug, Clone)]
pub struct StructField {
    pub span: SourceSpan,
    pub public: bool,
    pub name: String,
    pub type_: Type,
}

/// A tuple structure declaration.
#[derive(Debug, Clone)]
pub struct TupleDef {
    pub span: SourceSpan,
    pub public: bool,
    pub attrs: Vec<Attribute>,
    pub name: String,
    pub types: Vec<Type>,
}

/// An enum declaration.
#[derive(Debug, Clone)]
pub struct EnumDef {
    pub span: SourceSpan,
    pub public: bool,
    pub attrs: Vec<Attribute>,
    pub name: String,
    pub variants: Vec<EnumVariant>,
}

/// An enum variant.
#[derive(Debug, Clone)]
pub struct EnumVariant {
    pub span: SourceSpan,
    pub name: String,
    pub data: Option<EnumVariantData>,
}

/// A type alias declaration.
#[derive(Debug, Clone)]
pub struct TypeAliasDef {
    pub span: SourceSpan,
    pub public: bool,
    pub attrs: Vec<Attribute>,
    pub name: String,
    pub type_: Type,
}

/// The payload carried by an enum variant.
#[derive(Debug, Clone)]
pub enum EnumVariantData {
    Tuple(Vec<Type>),
    Struct(Vec<StructField>),
}

/// An import declaration.
#[derive(Debug, Clone)]
pub struct ImportDef {
    /// Source span covering the import declaration.
    pub span: SourceSpan,
    /// Relative import prefix such as `parent` or `package`.
    pub prefix: Vec<String>,
    /// Module path segments.
    pub path: Vec<String>,
    /// Source spans for module path segments.
    pub path_spans: Vec<SourceSpan>,
    /// Explicit symbols selected from the module path.
    pub symbols: Vec<String>,
    /// Source spans for explicit symbols selected from the module path.
    pub symbol_spans: Vec<SourceSpan>,
    /// Whether the import selects every public symbol.
    pub wildcard: bool,
}

/// An attribute attached to a declaration.
#[derive(Debug, Clone)]
pub struct Attribute {
    pub span: SourceSpan,
    pub name: String,
    pub args: Vec<Expression>,
}
