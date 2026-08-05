use miette::{Diagnostic, NamedSource, SourceSpan};
use thiserror::Error;

pub type InferResult<T> = Result<T, Box<TypeDiagnostic>>;

#[derive(Debug, Error, Diagnostic)]
#[error("{kind}")]
pub struct TypeDiagnostic {
    #[diagnostic(transparent)]
    pub kind: TypeDiagnosticKind,

    #[source_code]
    pub source_code: NamedSource<String>,

    #[label]
    pub span: SourceSpan,
}

#[derive(Debug, Error, Diagnostic)]
pub enum TypeDiagnosticKind {
    #[error("main function must return `unit`")]
    #[diagnostic(code(typeck::main_return_type))]
    MainReturnType,

    #[error("undefined variable `{name}`")]
    #[diagnostic(code(typeck::undefined_name))]
    UndefinedName { name: String },

    #[error("unknown type `{name}`")]
    #[diagnostic(code(typeck::unknown_type))]
    UnknownType { name: String },

    #[error("recursive type alias `{name}`")]
    #[diagnostic(code(typeck::recursive_type_alias))]
    RecursiveTypeAlias { name: String },

    #[error("type mismatch: expected `{expected}`, found `{found}`")]
    #[diagnostic(code(typeck::mismatch))]
    Mismatch {
        expected: crate::hir::Type,
        found: crate::hir::Type,
    },

    #[error("cannot assign to immutable variable `{name}`")]
    #[diagnostic(code(typeck::assign_to_immutable))]
    AssignToImmutable { name: String },

    #[error("cannot reference inner scope variable `{name}`")]
    #[diagnostic(code(typeck::inner_scope_ref))]
    InnerScopeRef { name: String },

    #[error("cannot pass immutable binding `{name}` as mutable reference")]
    #[diagnostic(code(typeck::immutable_as_mutable))]
    ImmutableAsMutable { name: String },

    #[error("function `{callee}` expects {expected} arguments, got {found}")]
    #[diagnostic(code(typeck::arg_count_mismatch))]
    ArgCountMismatch {
        callee: String,
        expected: usize,
        found: usize,
    },

    #[error("argument {index} to `{callee}` must be a reference; use `&`")]
    #[diagnostic(code(typeck::must_be_reference))]
    MustBeReference { callee: String, index: usize },

    #[error("cannot infer call target type")]
    #[diagnostic(code(typeck::cannot_infer_call_target))]
    CannotInferCallTarget,

    #[error("missing import for `{module}::{name}`; add `import {import_path};`")]
    #[diagnostic(code(typeck::missing_import))]
    MissingImport {
        module: String,
        name: String,
        import_path: String,
    },

    #[error("break outside of loop")]
    #[diagnostic(code(typeck::break_outside_loop))]
    BreakOutsideLoop,

    #[error("continue outside of loop")]
    #[diagnostic(code(typeck::continue_outside_loop))]
    ContinueOutsideLoop,

    #[error("struct `{type_name}` has no field `{field_name}`")]
    #[diagnostic(code(typeck::no_field))]
    NoField {
        type_name: String,
        field_name: String,
    },

    #[error("struct `{type_name}` is missing field `{field_name}`")]
    #[diagnostic(code(typeck::missing_field))]
    MissingField {
        type_name: String,
        field_name: String,
    },

    #[error("`{name}` is not a struct")]
    #[diagnostic(code(typeck::not_a_struct))]
    NotAStruct { name: String },

    #[error("tuple index out of bounds: `{index}`")]
    #[diagnostic(code(typeck::tuple_index_out_of_bounds))]
    TupleIndexOutOfBounds { index: String },

    #[error("enum `{type_name}` has no variant `{variant_name}`")]
    #[diagnostic(code(typeck::variant_not_found))]
    VariantNotFound {
        type_name: String,
        variant_name: String,
    },

    #[error("item `{type_name}::{variant_name}` is private or not found")]
    #[diagnostic(code(typeck::variant_private))]
    VariantPrivate {
        type_name: String,
        variant_name: String,
    },

    #[error("variant `{type_name}::{variant_name}` expects {expected} arguments, got {found}")]
    #[diagnostic(code(typeck::variant_arg_count_mismatch))]
    VariantArgCountMismatch {
        type_name: String,
        variant_name: String,
        expected: usize,
        found: usize,
    },

    #[error("cannot index type `{type_name}`")]
    #[diagnostic(code(typeck::cannot_index))]
    CannotIndex { type_name: String },

    #[error("index type must be an integer, found `{found}`")]
    #[diagnostic(code(typeck::index_must_be_integer))]
    IndexMustBeInteger { found: crate::hir::Type },

    #[error("cannot take reference to array index element")]
    #[diagnostic(code(typeck::cannot_ref_array_element))]
    CannotRefArrayElement,

    #[error("integer literal must be a numeric type, found `{found}`")]
    #[diagnostic(code(typeck::int_literal_mismatch))]
    IntLiteralMismatch { found: crate::hir::Type },

    #[error("float literal must be a float type, found `{found}`")]
    #[diagnostic(code(typeck::float_literal_mismatch))]
    FloatLiteralMismatch { found: crate::hir::Type },

    #[error("integer literal `{value}` is out of range for type `{found}`")]
    #[diagnostic(code(typeck::int_literal_out_of_range))]
    IntLiteralOutOfRange {
        value: i128,
        found: crate::hir::Type,
    },

    #[error("float literal `{value}` is out of range for type `{found}`")]
    #[diagnostic(code(typeck::float_literal_out_of_range))]
    FloatLiteralOutOfRange { value: f64, found: crate::hir::Type },

    #[error("uint literal must be an unsigned integer type, found `{found}`")]
    #[diagnostic(code(typeck::uint_literal_mismatch))]
    UIntLiteralMismatch { found: crate::hir::Type },

    #[error("uint literal `{value}` is out of range for type `{found}`")]
    #[diagnostic(code(typeck::uint_literal_out_of_range))]
    UIntLiteralOutOfRange {
        value: u128,
        found: crate::hir::Type,
    },

    #[error("integer power with negative exponent `{value}` is not defined")]
    #[diagnostic(code(typeck::pow_negative_exponent))]
    PowNegativeExponent { value: i128 },

    #[error("recursive type: `{a}` contains `{b}`")]
    #[diagnostic(code(typeck::recursive_type))]
    RecursiveType {
        a: crate::hir::Type,
        b: crate::hir::Type,
    },

    #[error("functions cannot return reference types")]
    #[diagnostic(code(typeck::cannot_return_ref))]
    CannotReturnRef,

    #[error("{feature} not supported yet")]
    #[diagnostic(code(typeck::unsupported_feature))]
    UnsupportedFeature { feature: String },

    #[error("unreachable statement")]
    #[diagnostic(code(typeck::unreachable_statement), severity(warning))]
    UnreachableStatement,

    #[error("cannot assign type `{expected}` to variable of type `{found}`")]
    #[diagnostic(code(typeck::assign_type_mismatch))]
    AssignTypeMismatch {
        expected: String,
        found: crate::hir::Type,
    },

    #[error("item `{module}::{name}` is private")]
    #[diagnostic(code(typeck::private_access))]
    PrivateAccess { module: String, name: String },

    #[error("field `{field_name}` of `{type_name}` is private")]
    #[diagnostic(code(typeck::private_field))]
    PrivateField {
        type_name: String,
        field_name: String,
    },

    #[error("match is not exhaustive; a `_` arm is required")]
    #[diagnostic(code(typeck::non_exhaustive_match))]
    NonExhaustiveMatch,

    #[error("match guard must be a bool, found `{found}`")]
    #[diagnostic(code(typeck::guard_not_bool))]
    GuardNotBool { found: crate::hir::Type },

    #[error("tuple pattern expects {expected} elements, got {found}")]
    #[diagnostic(code(typeck::tuple_arity_mismatch))]
    TupleArityMismatch { expected: usize, found: usize },
}
