pub mod error;
pub mod hir;
pub mod infer;
pub mod module;

pub use error::CompileWarning;
pub use error::TypeError;
pub use infer::typeck;
pub use infer::typeck_with_modules;
