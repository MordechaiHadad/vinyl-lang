pub mod error;
pub mod hir;
pub mod infer;

pub use error::CompileWarning;
pub use error::TypeError;
pub use infer::typeck;
