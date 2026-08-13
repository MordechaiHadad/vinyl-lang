pub mod error;
pub mod hir;
pub mod index;
pub mod infer;
pub mod module;

pub use error::{InferResult, TypeDiagnostic};
pub use infer::typeck;
pub use infer::typeck_with_index;
pub use infer::typeck_with_modules;
pub use infer::unused_import_warnings;
pub use infer::validate_main_return_type;
pub use infer::{Definition, DefinitionKind, HirExprRef, TypeckResult};
pub use miette::SourceSpan;
