pub mod error;
pub mod hir;
pub mod index;
pub mod infer;
pub mod module;

pub use error::{InferResult, TypeDiagnostic};
pub use infer::typeck;
pub use infer::typeck_with_index;
pub use infer::typeck_with_modules;
pub use infer::{Definition, DefinitionKind, HirExprRef, TypeckResult};
